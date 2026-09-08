//! VORAX Event Model
//!
//! Single boundary layer between the raw WebSocket JSON from `vorax-core`
//! and the enterprise gateway protocol implementations.
//!
//! All gateways consume [`VoraxEvent`] — never raw [`CorePayload`] JSON.
//! This ensures protocol concerns are fully isolated from the wire format.

use serde::{Deserialize, Serialize};

// ─── Wire payload (mirrors TelemetryPayload in core/src/ws.rs) ────────────────

/// Deserialised JSON frame broadcast by `vorax-core` on port 8765.
/// Large subcarrier arrays are skipped — the gateway does not process raw IQ.
#[derive(Debug, Clone, Deserialize)]
pub struct CorePayload {
    pub node_id:               u16,
    pub seq:                   u32,
    pub timestamp_us:          u64,
    pub pps:                   f64,
    pub heart_rate_bpm:        f32,
    pub respiration_rate_brpm: f32,
    pub hrv_rmssd_ms:          f32,
    pub velocity_m_s:          f32,
    pub target_x:              f32,
    pub target_y:              f32,
    pub target_z:              f32,
    pub alert_state:           String,
    pub presence_detected:     bool,
    pub presence_confidence:   f32,
    // Subcarrier arrays omitted — too large, unused in gateways
    #[serde(default, skip_deserializing)]
    _amplitudes: (),
}

// ─── Discrete event model ─────────────────────────────────────────────────────

/// A discrete VORAX event derived from successive CorePayload state transitions.
///
/// Gateways fire only on transitions — not on every 200 Hz frame. This prevents
/// alarm storms and duplicate SIA dispatches.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event_type")]
pub enum VoraxEvent {
    /// A human entered the monitored zone (presence OFF → ON transition).
    IntrusionDetected {
        x:           f32,
        y:           f32,
        z:           f32,
        confidence:  f32,
        node_id:     u16,
        timestamp_us: u64,
    },
    /// Zone confirmed empty (presence ON → OFF transition).
    PresenceLost {
        node_id:     u16,
        timestamp_us: u64,
    },
    /// Clinical or kinetic alert state changed (e.g. Nominal → BradycardiaAlert).
    AlertStateChange {
        previous:    String,
        current:     String,
        node_id:     u16,
        timestamp_us: u64,
    },
    /// Fall detected: sudden velocity spike followed by floor-level stillness.
    FallDetected {
        x:           f32,
        y:           f32,
        z:           f32,
        node_id:     u16,
        timestamp_us: u64,
    },
    /// Periodic vitals snapshot — rate-limited to ~1 Hz.
    VitalsUpdate {
        hr:          f32,
        rr:          f32,
        hrv:         f32,
        node_id:     u16,
        timestamp_us: u64,
    },
    /// Spatial position fix — rate-limited to ~5 Hz.
    PositionUpdate {
        x:           f32,
        y:           f32,
        z:           f32,
        confidence:  f32,
        node_id:     u16,
        timestamp_us: u64,
    },
}

impl VoraxEvent {
    /// Returns a short label suitable for logging and audit records.
    pub fn type_label(&self) -> &'static str {
        match self {
            Self::IntrusionDetected { .. }  => "IntrusionDetected",
            Self::PresenceLost { .. }       => "PresenceLost",
            Self::AlertStateChange { .. }   => "AlertStateChange",
            Self::FallDetected { .. }       => "FallDetected",
            Self::VitalsUpdate { .. }       => "VitalsUpdate",
            Self::PositionUpdate { .. }     => "PositionUpdate",
        }
    }

    pub fn timestamp_us(&self) -> u64 {
        match self {
            Self::IntrusionDetected { timestamp_us, .. } => *timestamp_us,
            Self::PresenceLost      { timestamp_us, .. } => *timestamp_us,
            Self::AlertStateChange  { timestamp_us, .. } => *timestamp_us,
            Self::FallDetected      { timestamp_us, .. } => *timestamp_us,
            Self::VitalsUpdate      { timestamp_us, .. } => *timestamp_us,
            Self::PositionUpdate    { timestamp_us, .. } => *timestamp_us,
        }
    }
}

// ─── Stateful classifier ──────────────────────────────────────────────────────

/// Translates a stream of raw CorePayload frames into discrete [`VoraxEvent`]s.
///
/// Holds just enough state to detect transitions. Stateless per gateway —
/// a single instance is shared across the whole fan-out loop.
pub struct EventClassifier {
    last_presence:    bool,
    last_alert:       String,
    last_vitals_ts:   u64,
    last_position_ts: u64,
}

impl EventClassifier {
    pub fn new() -> Self {
        Self {
            last_presence:    false,
            last_alert:       "Nominal".to_string(),
            last_vitals_ts:   0,
            last_position_ts: 0,
        }
    }

    /// Returns the set of discrete events derived from a single payload frame.
    /// May return zero events (most frames), one, or occasionally two.
    pub fn classify(&mut self, p: &CorePayload) -> Vec<VoraxEvent> {
        let mut events: Vec<VoraxEvent> = Vec::new();

        // ── Presence transition ───────────────────────────────────────────────
        if p.presence_detected && !self.last_presence {
            events.push(VoraxEvent::IntrusionDetected {
                x:            p.target_x,
                y:            p.target_y,
                z:            p.target_z,
                confidence:   p.presence_confidence,
                node_id:      p.node_id,
                timestamp_us: p.timestamp_us,
            });
        } else if !p.presence_detected && self.last_presence {
            events.push(VoraxEvent::PresenceLost {
                node_id:      p.node_id,
                timestamp_us: p.timestamp_us,
            });
        }
        self.last_presence = p.presence_detected;

        // ── Alert state transition ────────────────────────────────────────────
        if p.alert_state != self.last_alert {
            if p.alert_state == "FallSuspected" {
                events.push(VoraxEvent::FallDetected {
                    x:            p.target_x,
                    y:            p.target_y,
                    z:            p.target_z,
                    node_id:      p.node_id,
                    timestamp_us: p.timestamp_us,
                });
            }
            events.push(VoraxEvent::AlertStateChange {
                previous:     self.last_alert.clone(),
                current:      p.alert_state.clone(),
                node_id:      p.node_id,
                timestamp_us: p.timestamp_us,
            });
            self.last_alert = p.alert_state.clone();
        }

        // ── Vitals (~1 Hz) ────────────────────────────────────────────────────
        if p.timestamp_us.saturating_sub(self.last_vitals_ts) >= 1_000_000 {
            events.push(VoraxEvent::VitalsUpdate {
                hr:           p.heart_rate_bpm,
                rr:           p.respiration_rate_brpm,
                hrv:          p.hrv_rmssd_ms,
                node_id:      p.node_id,
                timestamp_us: p.timestamp_us,
            });
            self.last_vitals_ts = p.timestamp_us;
        }

        // ── Position fix (~5 Hz, only when presence confirmed) ────────────────
        if p.presence_detected
            && p.timestamp_us.saturating_sub(self.last_position_ts) >= 200_000
        {
            events.push(VoraxEvent::PositionUpdate {
                x:            p.target_x,
                y:            p.target_y,
                z:            p.target_z,
                confidence:   p.presence_confidence,
                node_id:      p.node_id,
                timestamp_us: p.timestamp_us,
            });
            self.last_position_ts = p.timestamp_us;
        }

        events
    }
}
