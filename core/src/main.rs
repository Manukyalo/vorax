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

use ingest::spawn_ingest;
use dsp::{unwrap_phase, remove_linear_trend};

#[derive(Parser, Debug)]
#[command(name = "vorax-core", about = "VORAX High-Throughput RF Ingestion & DSP Core")]
struct Args {
    /// Ingest UDP bind address
    #[arg(short, long, default_value = "0.0.0.0:5555")]
    bind: SocketAddr,
}

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

    let handle = spawn_ingest(args.bind).await?;
    let stats = handle.stats.clone();
    let rx = handle.rx;

    // Background dedicated DSP worker thread (pinned to numerical crunching)
    std::thread::spawn(move || {
        let mut frame_count: u64 = 0;
        while let Ok(frame) = rx.recv() {
            frame_count += 1;
            
            // Extract raw phases and amplitudes
            let mut phases = frame.phases_raw();
            let amplitudes = frame.amplitudes();

            // Perform phase sanitization
            unwrap_phase(&mut phases);
            remove_linear_trend(&mut phases);

            // Periodically log telemetry
            if frame_count % 400 == 0 {
                let node_id = frame.header.node_id;
                let seq = frame.header.sequence_number;
                let sc_count = frame.header.subcarrier_count;
                let mean_amp: f32 = amplitudes.iter().sum::<f32>() / amplitudes.len() as f32;
                let center_amp = amplitudes.get(32).copied().unwrap_or(0.0);
                info!(
                    "📡 [DSP NODE {:02}] Seq={:06} | MeanAmp={:.2} | CenterAmp={:.2} | Subcarriers={}",
                    node_id,
                    seq,
                    mean_amp,
                    center_amp,
                    sc_count
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
