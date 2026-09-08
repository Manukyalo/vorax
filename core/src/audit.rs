//! VORAX Core Event Audit Log
//!
//! Records every meaningful state transition observed by the ingest engine to an
//! append-only JSONL file (`vorax-events.jsonl` by default).
//!
//! # What gets logged
//! Only **state transitions** are written — not every 200 Hz frame. Specifically:
//! - Presence ON ↔ OFF transitions
//! - Alert state changes (Nominal → BradycardiaAlert, etc.)
//!
//! This keeps the log forensically meaningful without generating multi-GB files
//! from a continuous high-rate sensor stream.
//!
//! # Relationship to the gateway audit log
//! This file answers: **"What did VORAX observe?"**
//! The gateway audit (`vorax-gateway-audit.jsonl`) answers: **"Who was notified?"**
//! Both files together provide a complete forensic trail: observation → dispatch.
//!
//! # Design
//! - Append-only (never truncate, never update)
//! - Flushed to disk on every write
//! - Schema-versioned (field `"schema": 1`)
//! - Thread-safe via `Mutex<File>`
//!
//! # Record example
//! ```json
//! {"schema":1,"ts_rfc3339":"2026-09-09T01:00:00.000Z","ts_us":1725840000000000,
//!  "seq":42,"node_id":1,"event_type":"PresenceDetected","alert_state":"Nominal",
//!  "presence_detected":true,"presence_confidence":0.91,
//!  "target_x":3.1,"target_y":2.4,"target_z":1.0,
//!  "heart_rate_bpm":72.5,"respiration_rate_brpm":15.2,"hrv_rmssd_ms":38.1,
//!  "velocity_m_s":0.3}
//! ```

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use tracing::error;

#[derive(Serialize)]
struct CoreEventRecord<'a> {
    schema:                 u8,
    ts_rfc3339:             String,
    ts_us:                  u64,
    seq:                    u32,
    node_id:                u16,
    event_type:             &'a str,
    alert_state:            &'a str,
    presence_detected:      bool,
    presence_confidence:    f32,
    target_x:               f32,
    target_y:               f32,
    target_z:               f32,
    heart_rate_bpm:         f32,
    respiration_rate_brpm:  f32,
    hrv_rmssd_ms:           f32,
    velocity_m_s:           f32,
}

/// Thread-safe, append-only JSONL event logger for the ingest core.
pub struct CoreAuditLogger {
    file:             Mutex<std::fs::File>,
    last_alert_state: Mutex<String>,
    last_presence:    Mutex<bool>,
}

impl CoreAuditLogger {
    /// Open (or create) the audit log at `path`.
    /// The file is opened in **append-only** mode — existing records are
    /// preserved even if the process crashes and restarts.
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true) // Immutable audit trail — never truncate
            .open(path)?;
        Ok(Self {
            file:             Mutex::new(file),
            last_alert_state: Mutex::new("Nominal".to_string()),
            last_presence:    Mutex::new(false),
        })
    }

    /// Conditionally write a record on state transitions only.
    ///
    /// Called on every DSP output frame. Internally throttles to meaningful
    /// changes — no-ops on frames where nothing has changed.
    #[allow(clippy::too_many_arguments)]
    pub fn maybe_log(
        &self,
        ts_us:                 u64,
        seq:                   u32,
        node_id:               u16,
        alert_state:           &str,
        presence_detected:     bool,
        presence_confidence:   f32,
        target_x:              f32,
        target_y:              f32,
        target_z:              f32,
        heart_rate_bpm:        f32,
        respiration_rate_brpm: f32,
        hrv_rmssd_ms:          f32,
        velocity_m_s:          f32,
    ) {
        // Read previous state (hold locks only for the comparison, not the write)
        let (prev_alert, prev_presence) = {
            let a = self.last_alert_state.lock().unwrap();
            let p = self.last_presence.lock().unwrap();
            (a.clone(), *p)
        };

        let alert_changed    = alert_state != prev_alert;
        let presence_changed = presence_detected != prev_presence;

        if !alert_changed && !presence_changed {
            return; // Nothing changed — skip this frame
        }

        // Update tracked state
        *self.last_alert_state.lock().unwrap() = alert_state.to_string();
        *self.last_presence.lock().unwrap()    = presence_detected;

        let event_type = if presence_changed {
            if presence_detected { "PresenceDetected" } else { "PresenceLost" }
        } else {
            "AlertStateChange"
        };

        let record = CoreEventRecord {
            schema: 1,
            ts_rfc3339: chrono::Utc::now()
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            ts_us,
            seq,
            node_id,
            event_type,
            alert_state,
            presence_detected,
            presence_confidence,
            target_x,
            target_y,
            target_z,
            heart_rate_bpm,
            respiration_rate_brpm,
            hrv_rmssd_ms,
            velocity_m_s,
        };

        self.write(&record);
    }

    fn write(&self, record: &CoreEventRecord<'_>) {
        let mut line = match serde_json::to_string(record) {
            Ok(s) => s,
            Err(e) => {
                error!("[CORE AUDIT] Serialise failed: {}", e);
                return;
            }
        };
        line.push('\n');

        match self.file.lock() {
            Ok(mut f) => {
                if let Err(e) = f.write_all(line.as_bytes()) {
                    error!("[CORE AUDIT] Write failed: {}", e);
                    return;
                }
                // Per-record flush — must hit disk before we return
                if let Err(e) = f.flush() {
                    error!("[CORE AUDIT] Flush failed: {}", e);
                }
            }
            Err(e) => error!("[CORE AUDIT] Mutex poisoned: {}", e),
        }
    }
}
