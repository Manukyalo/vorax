//! VORAX WebSockets Broadcast Server
//!
//! Exposes an async WebSocket endpoint on `0.0.0.0:8765` to stream sanitized
//! telemetry, amplitudes, unwrapped phases, and estimated biometrics to the web UI.

use std::net::SocketAddr;
use std::sync::Arc;
use futures_util::SinkExt;
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize)]
pub struct TelemetryPayload {
    pub node_id: u16,
    pub seq: u32,
    pub timestamp_us: u64,
    pub pps: u64,
    pub mean_amp: f32,
    pub center_amp: f32,
    pub heart_rate_bpm: f32,
    pub respiration_rate_brpm: f32,
    pub hrv_rmssd_ms: f32,
    pub velocity_m_s: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub target_z: f32,
    pub alert_state: String,
    pub presence_detected: bool,
    pub presence_confidence: f32,
    pub amplitudes: Vec<f32>,
    pub phases: Vec<f32>,
}

pub struct WsBroadcaster {
    tx: broadcast::Sender<String>,
}

impl WsBroadcaster {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(64);
        Self { tx }
    }

    pub fn broadcast(&self, payload: &TelemetryPayload) {
        if let Ok(json) = serde_json::to_string(payload) {
            // Drop error if no active clients are connected
            let _ = self.tx.send(json);
        }
    }

    pub async fn start_server(self: Arc<Self>, bind_addr: SocketAddr) -> anyhow::Result<()> {
        let listener = TcpListener::bind(bind_addr).await?;
        info!("🌐 [VORAX WEBSOCKET] Telemetry server listening on ws://{}", bind_addr);

        tokio::spawn(async move {
            while let Ok((stream, peer_addr)) = listener.accept().await {
                let rx = self.tx.subscribe();
                tokio::spawn(handle_connection(stream, peer_addr, rx));
            }
        });

        Ok(())
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    peer_addr: SocketAddr,
    mut rx: broadcast::Receiver<String>,
) {
    info!("🔗 [WS CLIENT] Connected from {}", peer_addr);
    let mut ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            warn!("Failed to establish WS handshake with {}: {}", peer_addr, e);
            return;
        }
    };

    while let Ok(msg) = rx.recv().await {
        if let Err(e) = ws_stream.send(Message::Text(msg.into())).await {
            warn!("WS client {} disconnected ({})", peer_addr, e);
            break;
        }
    }
}
