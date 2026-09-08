//! VORAX Core Engine Entrypoint
//!
//! Orchestrates:
//! 1. High-speed UDP Ingestion
//! 2. Bounded Ring Buffer Dispatching
//! 3. DSP Phase Unwrapping & Amplitude Extraction
//! 4. Real-time Telemetry Logging

use std::net::SocketAddr;
use std::time::Duration;
use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod protocol;
mod ingest;
mod dsp;
mod spatial;

use ingest::spawn_ingest;
use dsp::{unwrap_phase, remove_linear_trend};

#[derive(Parser, Debug)]
#[command(name = "vorax-core", about = "VORAX High-Throughput RF Ingestion & DSP Core")]
struct Args {
    /// Ingest UDP bind address
    #[arg(short, long, default_value = "0.0.0.0:5555")]
    bind: SocketAddr,
}

mod ws;
use std::sync::Arc;
use ws::{WsBroadcaster, TelemetryPayload};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let args = Args::parse();
    info!("========================================================");
    info!("        VORAX RF SPATIAL INTELLIGENCE CORE v0.1.0        ");
    info!("========================================================");

    let broadcaster = Arc::new(WsBroadcaster::new());
    let ws_bind: SocketAddr = "0.0.0.0:8765".parse()?;
    broadcaster.clone().start_server(ws_bind).await?;

    let handle = spawn_ingest(args.bind).await?;
    let stats = handle.stats.clone();
    let rx = handle.rx;
    let ws_out = broadcaster.clone();
    let stats_ref = stats.clone();

    // Background dedicated DSP & Spatial worker thread (pinned to numerical crunching)
    std::thread::spawn(move || {
        let mut frame_count: u64 = 0;
        let mut dispatcher = dsp::StateDispatcher::new(200.0, dsp::doppler::DEFAULT_CARRIER_FREQ_HZ);
        let music = spatial::MusicEngine2D::new(16, 1);
        let multilaterator = spatial::Multilaterator::new(6.0, 6.0, 3.0);
        let mut voxel_grid = spatial::VoxelOccupancyGrid::new();
        let mut latest_target = spatial::Point3D::new(3.0, 3.0, 1.0);
        let mut node_obs_map = std::collections::HashMap::new();

        while let Ok(frame) = rx.recv() {
            frame_count += 1;
            
            // Extract raw phases and amplitudes
            let mut phases = frame.phases_raw();
            let amplitudes = frame.amplitudes();

            // Perform phase sanitization
            unwrap_phase(&mut phases);
            remove_linear_trend(&mut phases);

            // Execute Phase 2 Dual-Mode Motion & Biometrics Pipeline
            let dsp_output = dispatcher.dispatch(&amplitudes);

            let node_id = frame.header.node_id;
            let seq = frame.header.sequence_number;
            let ts = frame.header.timestamp_us;
            let mean_amp: f32 = amplitudes.iter().sum::<f32>() / amplitudes.len() as f32;
            let center_amp = amplitudes.get(32).copied().unwrap_or(0.0);

            // 3D Spatial Localization: Run 2D-MUSIC on periodic frames
            if frame_count % 7 == 0 {
                let complex_sc: Vec<spatial::Complex32> = amplitudes
                    .iter()
                    .zip(phases.iter())
                    .map(|(&a, &p)| spatial::Complex32::from_polar(a, p))
                    .collect();

                let cov = spatial::compute_smoothed_covariance(&complex_sc, 16);
                let peak = music.estimate_peak(&cov);

                let node_pos = match node_id {
                    1 => spatial::Point3D::new(0.2, 0.2, 1.2),
                    2 => spatial::Point3D::new(5.8, 0.2, 1.2),
                    3 => spatial::Point3D::new(5.8, 5.8, 1.2),
                    4 => spatial::Point3D::new(0.2, 5.8, 1.2),
                    _ => spatial::Point3D::new(0.2, 0.2, 1.2),
                };

                node_obs_map.insert(
                    node_id,
                    spatial::NodeObservation {
                        node_id,
                        node_position: node_pos,
                        peak,
                    },
                );

                if node_obs_map.len() >= 3 {
                    let obs_vec: Vec<spatial::NodeObservation> = node_obs_map.values().cloned().collect();
                    if let Some(solved) = multilaterator.solve_position_2d(&obs_vec) {
                        latest_target = solved;
                        voxel_grid.update_target(&latest_target, dsp_output.confidence);
                    }
                }
            }

            let (rr, hr, hrv, alert) = match &dsp_output.vitals {
                Some(v) => (
                    v.respiration_rate_brpm,
                    v.heart_rate_bpm,
                    v.hrv_rmssd_ms,
                    format!("{:?}", v.alert_state),
                ),
                None => (15.0, 72.0, 35.0, "Nominal".to_string()),
            };
            let velocity = dsp_output.doppler.as_ref().map(|d| d.peak_velocity_m_s).unwrap_or(0.0);

            // Stream frames to WebSocket clients at ~30 FPS (every 7th frame at 200 Hz)
            if frame_count % 7 == 0 {
                let snap = stats_ref.snapshot();

                ws_out.broadcast(&TelemetryPayload {
                    node_id,
                    seq,
                    timestamp_us: ts,
                    pps: snap.pps,
                    mean_amp,
                    center_amp,
                    heart_rate_bpm: hr,
                    respiration_rate_brpm: rr,
                    hrv_rmssd_ms: hrv,
                    velocity_m_s: velocity,
                    target_x: latest_target.x,
                    target_y: latest_target.y,
                    target_z: latest_target.z,
                    alert_state: alert,
                    presence_detected: dsp_output.presence_detected,
                    presence_confidence: dsp_output.confidence,
                    amplitudes: amplitudes.clone(),
                    phases: phases.clone(),
                });
            }

            // Periodically log telemetry
            if frame_count % 400 == 0 {
                info!(
                    "📡 [DSP NODE {:02}] Seq={:06} | State={:?} | MeanAmp={:.2} | HR={:.1} BPM | RR={:.1} BrPM | Conf={:.2}",
                    node_id,
                    seq,
                    dsp_output.state,
                    mean_amp,
                    hr,
                    rr,
                    dsp_output.confidence
                );
            }
        }
    });

    // Telemetry heartbeat loop
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let snap = stats.snapshot();
        info!(
            "⚡ [INGEST METRICS] Rate: {} pkts/s | Ingested: {} | Dropped: {} | Malformed: {}",
            snap.pps, snap.received, snap.dropped, snap.malformed
        );
    }
}
