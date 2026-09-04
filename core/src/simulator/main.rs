//! VORAX CSI Packet Simulator
//!
//! Generates synthetic 64-subcarrier CSI frames replicating ESP32-S3 physical packets
//! at 200 Hz (or configurable rate) and transmits over UDP.
//!
//! Simulates:
//! 1. Static multipath baseline across 64 subcarriers
//! 2. Breathing micro-Doppler (~0.25 Hz sine wave on select subcarriers)
//! 3. Periodic human movement perturbation burst

use std::f32::consts::PI;
use std::net::SocketAddr;
use std::time::Duration;
use clap::Parser;
use tokio::net::UdpSocket;
use tokio::time::interval;

#[path = "../protocol.rs"]
mod protocol;
use protocol::{CsiPacketHeader, SubcarrierComplex, PACKET_MAGIC};

#[derive(Parser, Debug)]
#[command(name = "vorax-sim", about = "VORAX Synthetic CSI Packet Streamer")]
struct Args {
    /// Target UDP destination (e.g., 127.0.0.1:5555)
    #[arg(short, long, default_value = "127.0.0.1:5555")]
    target: SocketAddr,

    /// Node ID to impersonate
    #[arg(short, long, default_value_t = 1)]
    node_id: u16,

    /// Transmission rate in Hz
    #[arg(short, long, default_value_t = 200)]
    rate: u64,

    /// Number of subcarriers (64 for HT20)
    #[arg(short, long, default_value_t = 64)]
    subcarriers: u8,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    println!("📡 [VORAX SIMULATOR] Streaming node_id={} to {} @ {} Hz ({} subcarriers)",
        args.node_id, args.target, args.rate, args.subcarriers
    );

    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    let period_micros = 1_000_000 / args.rate;
    let mut ticker = interval(Duration::from_micros(period_micros));

    let mut seq: u32 = 0;
    let start_time = std::time::Instant::now();

    let sc_count = args.subcarriers as usize;
    let header_size = std::mem::size_of::<CsiPacketHeader>();
    let payload_size = sc_count * std::mem::size_of::<SubcarrierComplex>();
    let total_packet_size = header_size + payload_size;

    let mut packet_buffer = vec![0u8; total_packet_size];

    loop {
        ticker.tick().await;
        let elapsed_secs = start_time.elapsed().as_secs_f32();

        // Synthetic vitals modulation:
        // Human respiration: ~0.25 Hz (15 breaths/min)
        // Heart rate micro-Doppler: ~1.2 Hz (72 BPM)
        let resp_signal = (2.0 * PI * 0.25 * elapsed_secs).sin() * 3.5;
        let cardiac_signal = (2.0 * PI * 1.2 * elapsed_secs).sin() * 0.8;

        let header = CsiPacketHeader {
            magic: PACKET_MAGIC,
            node_id: args.node_id,
            sequence_number: seq,
            timestamp_us: start_time.elapsed().as_micros() as u64,
            channel: 6,
            rssi: -52,
            noise_floor: -95,
            subcarrier_count: args.subcarriers,
        };

        // Write header bytes (packed)
        unsafe {
            std::ptr::copy_nonoverlapping(
                &header as *const CsiPacketHeader as *const u8,
                packet_buffer.as_mut_ptr(),
                header_size,
            );
        }

        // Generate synthetic per-subcarrier complex pairs (imaginary, real)
        let mut offset = header_size;
        for sc_idx in 0..sc_count {
            // Static multi-path baseline per subcarrier
            let base_freq = sc_idx as f32 * 0.15;
            let base_i = (base_freq.cos() * 25.0) as f32;
            let base_q = (base_freq.sin() * 25.0) as f32;

            // Apply physiological perturbation primarily to center sensitive subcarriers (indices 20..44)
            let perturbation = if (20..44).contains(&sc_idx) {
                resp_signal + cardiac_signal
            } else {
                0.0
            };

            let real_val = ((base_i + perturbation).clamp(-127.0, 127.0)) as i8;
            let imag_val = ((base_q + perturbation * 0.5).clamp(-127.0, 127.0)) as i8;

            let sc = SubcarrierComplex {
                imaginary: imag_val,
                real: real_val,
            };

            unsafe {
                std::ptr::copy_nonoverlapping(
                    &sc as *const SubcarrierComplex as *const u8,
                    packet_buffer.as_mut_ptr().add(offset),
                    std::mem::size_of::<SubcarrierComplex>(),
                );
            }
            offset += std::mem::size_of::<SubcarrierComplex>();
        }

        socket.send_to(&packet_buffer, args.target).await?;
        seq = seq.wrapping_add(1);

        if seq % 1000 == 0 {
            let ts = header.timestamp_us;
            println!("  [SIM] Sent {} packets (latest timestamp: {} us)", seq, ts);
        }
    }
}
