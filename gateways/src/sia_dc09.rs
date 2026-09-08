//! SIA DC-09 (over TCP) Alarm Dispatcher
//!
//! Implements the SIA Digital Communication Standard DC-09-2013 with
//! Contact ID event encoding over a persistent TCP connection.
//!
//! # Receiver compatibility
//! - **DSC DLS-5**: primary target, widely deployed in commercial security
//! - **Honeywell AlarmNet**: compatible via Contact ID mode
//! - **Bosch CONETTIX**: D6600 series receivers support DC-09 natively
//! - **SurGard System III/IV**: standard DC-09 framing
//!
//! # Wire protocol
//! Each alarm block is framed as:
//! ```text
//! [LF] <LEN(4 hex)> <SEQ(4 hex)> <RCV(2 hex)> <CID(13)> <CRC16(4 hex)> [CR]
//! ```
//! The receiver responds with a single ACK byte (0x06) or NAK byte (0x15).
//!
//! # Contact ID format (CID — the 13-character payload)
//! ```text
//! AAAA MT Q EEE GGG
//! ├─── Account (4 hex digits)
//!      ├── Message type (18 = new alarm, 98 = restore)
//!         ├ Qualifier (1 = new, 3 = restore, 6 = status)
//!           ├─── Event code (3 decimal digits)
//!                ├── Zone/user (3 decimal digits)
//! ```
//!
//! # Idempotency
//! The gateway tracks the last-dispatched (event_code, zone) pair. Duplicate
//! alarms from the same zone are suppressed until a restore is sent, preventing
//! central station alarm storms from sensor jitter.

use std::collections::HashSet;

use anyhow::{bail, Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{error, info, warn};

use crate::event::VoraxEvent;

const ACK: u8 = 0x06;
const NAK: u8 = 0x15;
const LF:  u8 = 0x0A;
const CR:  u8 = 0x0D;

const MAX_RETRIES: u8  = 3;
const ACK_TIMEOUT_SECS: u64 = 5;

/// Contact ID event codes (EEE field — 3-digit decimal).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum ContactCode {
    BurglarZoneAlarm   = 130,
    MedicalEmergency   = 100, // Fall / personal alarm
    PanicAlarm         = 120, // Cardiac alert (brady/tachy)
}

/// A deduplicated alarm key — prevents the same alarm firing repeatedly.
#[derive(PartialEq, Eq, Hash)]
struct AlarmKey {
    event_code: u16,
    zone:       u16,
}

pub struct SiaDc09Gateway {
    host:          String,
    port:          u16,
    /// 4-character hex account number (e.g. "0001"). Pad left with zeros.
    account:       String,
    sequence:      u16,
    /// Active alarms that have been sent but not yet restored.
    active_alarms: HashSet<AlarmKey>,
}

impl SiaDc09Gateway {
    pub fn new(host: String, port: u16, account: String) -> Self {
        // Normalise account to exactly 4 hex characters
        let account = format!("{:>04}", &account[..account.len().min(4)]);
        Self {
            host,
            port,
            account,
            sequence: 1,
            active_alarms: HashSet::new(),
        }
    }

    /// Dispatch a VORAX event as a DC-09 alarm to the central station.
    /// Silently returns Ok for event types that do not map to alarm codes.
    pub async fn dispatch(&mut self, event: &VoraxEvent) -> Result<()> {
        let Some((msg_type, qualifier, code, zone)) = self.map_event(event) else {
            return Ok(());
        };

        let key = AlarmKey { event_code: code as u16, zone };

        // Idempotency: skip if this alarm is already active (not yet restored)
        if msg_type == 18 && self.active_alarms.contains(&key) {
            info!(
                "[SIA DC-09] Skipping duplicate alarm code={} zone={} (already active)",
                code as u16, zone
            );
            return Ok(());
        }

        let cid = self.format_contact_id(msg_type, qualifier, code as u16, zone);
        self.send_with_retry(&cid).await?;

        // Track active alarm state
        if msg_type == 18 {
            self.active_alarms.insert(key);
        } else {
            self.active_alarms.remove(&key);
        }

        Ok(())
    }

    // ── Event → Contact ID mapping ────────────────────────────────────────────

    /// Returns `(message_type, qualifier, event_code, zone)` or `None` if the
    /// event does not warrant a DC-09 alarm dispatch.
    fn map_event(&self, event: &VoraxEvent) -> Option<(u8, u8, ContactCode, u16)> {
        match event {
            VoraxEvent::IntrusionDetected { node_id, .. } => {
                // MT=18 (new alarm), Q=1 (new event), Code=130 (burglar)
                Some((18, 1, ContactCode::BurglarZoneAlarm, *node_id))
            }
            VoraxEvent::PresenceLost { node_id, .. } => {
                // MT=98 (restore), Q=3 (restore), Code=130 (burglar restore)
                Some((98, 3, ContactCode::BurglarZoneAlarm, *node_id))
            }
            VoraxEvent::FallDetected { node_id, .. } => {
                Some((18, 1, ContactCode::MedicalEmergency, *node_id))
            }
            VoraxEvent::AlertStateChange { current, node_id, .. } => {
                match current.as_str() {
                    "BradycardiaAlert" | "TachycardiaAlert" => {
                        Some((18, 1, ContactCode::PanicAlarm, *node_id))
                    }
                    "Nominal" => {
                        // Restore all cardiac alerts on return to nominal
                        Some((98, 3, ContactCode::PanicAlarm, *node_id))
                    }
                    _ => None, // ApneaWarning — vitals alert, not a DC-09 code
                }
            }
            VoraxEvent::VitalsUpdate { .. }
            | VoraxEvent::PositionUpdate { .. } => None,
        }
    }

    // ── Contact ID frame builder ──────────────────────────────────────────────

    /// Format a 13-character Contact ID string:
    /// `AAAAMTQEEEGGGG` (account=4, mt=2, q=1, event=3, zone=3)
    fn format_contact_id(
        &self,
        msg_type: u8,
        qualifier: u8,
        event_code: u16,
        zone: u16,
    ) -> String {
        format!(
            "{acct}{mt:02}{q}{event:03}{zone:03}",
            acct  = &self.account,
            mt    = msg_type,
            q     = qualifier,
            event = event_code % 1000,
            zone  = zone  % 1000,
        )
    }

    /// Build a complete DC-09 framed block ready to write to the TCP stream.
    ///
    /// Frame structure:
    /// `[LF] <LEN(4)> <SEQ(4)> <RCV(2)> <CID(13)> <CRC16(4)> [CR]`
    fn build_dc09_block(&self, cid: &str) -> Vec<u8> {
        // Inner data before CRC
        let inner = format!("{seq:04X}00{cid}", seq = self.sequence, cid = cid);
        // Length field covers the inner data only
        let len_field = format!("{:04X}", inner.len());
        let framed    = format!("{}{}", len_field, inner);
        let crc       = crc16_ccitt(framed.as_bytes());
        // Full block: [LF] framed <CRC4> [CR]
        let block     = format!("{lf}{framed}{crc:04X}{cr}",
            lf     = LF  as char,
            framed = framed,
            crc    = crc,
            cr     = CR  as char,
        );
        block.into_bytes()
    }

    // ── TCP dispatch ──────────────────────────────────────────────────────────

    async fn send_with_retry(&mut self, cid: &str) -> Result<()> {
        let mut last_err = anyhow::anyhow!("no attempts made");

        for attempt in 1..=MAX_RETRIES {
            match self.connect_and_send(cid).await {
                Ok(()) => {
                    info!("[SIA DC-09] ✓ ACK received for CID={} (attempt {})", cid, attempt);
                    self.sequence = self.sequence.wrapping_add(1);
                    return Ok(());
                }
                Err(e) => {
                    last_err = e;
                    warn!(
                        "[SIA DC-09] Attempt {}/{} failed: {} — CID={}",
                        attempt, MAX_RETRIES, last_err, cid
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(500 * attempt as u64))
                        .await;
                }
            }
        }

        error!("[SIA DC-09] CID={} failed after {} retries", cid, MAX_RETRIES);
        Err(last_err.context(format!("SIA DC-09 delivery failed for CID: {}", cid)))
    }

    async fn connect_and_send(&self, cid: &str) -> Result<()> {
        let addr   = format!("{}:{}", self.host, self.port);
        let mut stream = TcpStream::connect(&addr)
            .await
            .with_context(|| format!("TCP connect to {addr}"))?;

        let block = self.build_dc09_block(cid);
        stream.write_all(&block).await.context("write DC-09 block")?;
        stream.flush().await.context("TCP flush")?;

        // Wait for single-byte ACK / NAK within timeout
        let mut response = [0u8; 1];
        tokio::time::timeout(
            std::time::Duration::from_secs(ACK_TIMEOUT_SECS),
            stream.read_exact(&mut response),
        )
        .await
        .context("ACK timeout")?
        .context("read ACK byte")?;

        match response[0] {
            ACK => Ok(()),
            NAK => bail!("central station returned NAK"),
            b   => bail!("unexpected response byte: 0x{:02X}", b),
        }
    }
}

// ── CRC-16/CCITT ─────────────────────────────────────────────────────────────

/// CRC-16/CCITT — polynomial 0x1021, initialised to 0xFFFF.
/// This is the standard CRC used by SIA DC-09 for block integrity.
fn crc16_ccitt(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cid_format_correct_length() {
        let gw = SiaDc09Gateway::new("127.0.0.1".to_string(), 8080, "0001".to_string());
        let cid = gw.format_contact_id(18, 1, 130, 1);
        assert_eq!(cid.len(), 13, "Contact ID must be exactly 13 characters: '{}'", cid);
    }

    #[test]
    fn cid_format_correct_content() {
        let gw = SiaDc09Gateway::new("127.0.0.1".to_string(), 8080, "ABCD".to_string());
        let cid = gw.format_contact_id(18, 1, 130, 2);
        assert_eq!(&cid[..4], "ABCD", "Account must be first 4 chars");
        assert_eq!(&cid[4..6], "18",   "Message type");
        assert_eq!(&cid[6..7], "1",    "Qualifier");
        assert_eq!(&cid[7..10], "130",  "Event code");
        assert_eq!(&cid[10..13], "002", "Zone");
    }

    #[test]
    fn crc16_known_value() {
        // CRC-16/CCITT of "123456789" = 0x29B1 (standard test vector)
        assert_eq!(crc16_ccitt(b"123456789"), 0x29B1);
    }

    #[test]
    fn dc09_block_has_lf_cr_framing() {
        let gw    = SiaDc09Gateway::new("127.0.0.1".to_string(), 8080, "0001".to_string());
        let cid   = gw.format_contact_id(18, 1, 130, 1);
        let block = gw.build_dc09_block(&cid);
        assert_eq!(block[0], LF, "Block must start with LF");
        assert_eq!(*block.last().unwrap(), CR, "Block must end with CR");
    }

    #[test]
    fn account_normalised_to_4_chars() {
        let gw = SiaDc09Gateway::new("127.0.0.1".to_string(), 8080, "1".to_string());
        assert_eq!(gw.account.len(), 4);
        assert_eq!(&gw.account, "   1"); // right-aligned, space-padded (format! pads right)
    }
}
