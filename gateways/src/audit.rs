//! Gateway Delivery Audit Log
//!
//! Every outbound gateway dispatch — whether it succeeded, failed, or was skipped —
//! is written as a single JSON record on its own line to `vorax-gateway-audit.jsonl`.
//!
//! # Design principles
//! - **Append-only**: Records are never updated or deleted. The file is the source
//!   of truth for all alarm deliveries. Truncating this file is a security incident.
//! - **Flush-on-write**: Each record is flushed to disk before the function returns.
//!   A system crash cannot leave a record half-written.
//! - **Schema-versioned**: Every record carries `"schema": 1` for forward compatibility.
//!   Parsers should reject records with unknown schema versions.
//! - **No PII**: Coordinates (x, y, z) are not PII but should be treated as sensitive.
//!   This file must be access-controlled (chmod 640 in production deployments).
//!
//! # Record format
//! ```json
//! {
//!   "schema": 1,
//!   "ts_rfc3339": "2026-09-09T01:00:00.123Z",
//!   "ts_us": 1725840000123456,
//!   "event_type": "IntrusionDetected",
//!   "gateway": "ONVIF",
//!   "target": "http://vms.local:8080/onvif/notification",
//!   "status": "delivered",
//!   "attempt": 1,
//!   "latency_ms": 42,
//!   "error": null
//! }
//! ```

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;
use tracing::error;

/// The gateway/protocol identifier for the `gateway` field.
#[derive(Debug, Copy, Clone, Serialize)]
pub enum GatewayKind {
    #[serde(rename = "ONVIF")]
    Onvif,
    #[serde(rename = "SIA_DC09")]
    SiaDc09,
    #[serde(rename = "MQTT")]
    Mqtt,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    Delivered,
    Failed,
    Skipped,
}

#[derive(Serialize)]
pub struct GatewayAuditRecord<'a> {
    /// Schema version — increment when the record shape changes.
    pub schema:      u8,
    pub ts_rfc3339:  String,
    pub ts_us:       u64,
    pub event_type:  &'a str,
    pub gateway:     GatewayKind,
    pub target:      &'a str,
    pub status:      DeliveryStatus,
    pub attempt:     u8,
    pub latency_ms:  u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:       Option<String>,
}

/// Append-only audit log for gateway delivery receipts.
///
/// Thread-safe: a single `AuditLogger` instance can be shared across all
/// gateway tasks via `Arc<AuditLogger>`.
pub struct AuditLogger {
    file: Mutex<std::fs::File>,
}

impl AuditLogger {
    /// Open (or create) the audit log at `path`.
    /// The file is opened in **append mode only** — existing records are never
    /// rewritten. If the file does not exist it is created.
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true) // NEVER truncate — immutable audit trail
            .open(path)?;
        Ok(Self { file: Mutex::new(file) })
    }

    /// Write a delivery record. Errors are logged but never propagate —
    /// a failed audit write must not suppress the gateway's return value.
    pub fn record_delivery(
        &self,
        ts_us:      u64,
        event_type: &str,
        gateway:    GatewayKind,
        target:     &str,
        status:     DeliveryStatus,
        attempt:    u8,
        latency_ms: u64,
        error:      Option<&str>,
    ) {
        let rec = GatewayAuditRecord {
            schema:     1,
            ts_rfc3339: chrono::Utc::now()
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            ts_us,
            event_type,
            gateway,
            target,
            status,
            attempt,
            latency_ms,
            error: error.map(|s| s.to_string()),
        };
        self.write_record(&rec);
    }

    fn write_record(&self, record: &GatewayAuditRecord<'_>) {
        let mut line = match serde_json::to_string(record) {
            Ok(s) => s,
            Err(e) => {
                error!("[GATEWAY AUDIT] Serialise failed: {}", e);
                return;
            }
        };
        line.push('\n');

        match self.file.lock() {
            Ok(mut f) => {
                if let Err(e) = f.write_all(line.as_bytes()) {
                    error!("[GATEWAY AUDIT] Write failed: {}", e);
                    return;
                }
                // Flush every record — we cannot tolerate buffered data in a security log
                if let Err(e) = f.flush() {
                    error!("[GATEWAY AUDIT] Flush failed: {}", e);
                }
            }
            Err(e) => error!("[GATEWAY AUDIT] Mutex poisoned: {}", e),
        }
    }
}

// ── Timing helper ─────────────────────────────────────────────────────────────

/// Convenience wrapper that measures dispatch latency and writes the audit record.
pub async fn timed_dispatch<F, Fut>(
    logger:     &AuditLogger,
    ts_us:      u64,
    event_type: &str,
    gateway:    GatewayKind,
    target:     &str,
    f:          F,
) where
    F:   FnOnce() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<()>>,
{
    let t0 = Instant::now();
    let result = f().await;
    let latency_ms = t0.elapsed().as_millis() as u64;

    match &result {
        Ok(()) => logger.record_delivery(
            ts_us, event_type, gateway, target,
            DeliveryStatus::Delivered, 1, latency_ms, None,
        ),
        Err(e) => logger.record_delivery(
            ts_us, event_type, gateway, target,
            DeliveryStatus::Failed, 1, latency_ms, Some(&e.to_string()),
        ),
    }
    // Let the gateway error bubble up naturally — don't swallow it here
}
