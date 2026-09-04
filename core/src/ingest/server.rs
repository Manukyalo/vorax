//! VORAX Core — UDP Ingest Server
//!
//! Listens for CSI frames from ESP32-S3 nodes (or the simulator),
//! parses them, and pushes them onto a bounded lock-free ring buffer
//! for downstream DSP consumers.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossbeam::channel::{bounded, Receiver, Sender, TrySendError};
use tokio::net::UdpSocket;
use tracing::{debug, error, info, warn};

use crate::protocol::CsiFrame;

/// Size of the ingest ring buffer (number of frames).
/// At 200 Hz × 4 nodes = 800 frames/s — this gives ~250ms of headroom.
const RING_BUFFER_CAPACITY: usize = 256;

/// Maximum UDP datagram we'll ever receive (header + 128 subcarriers × 2 bytes)
const MAX_DATAGRAM_BYTES: usize = 512;

/// Shared ingestion statistics — updated atomically by the receiver task.
#[derive(Debug, Default)]
pub struct IngestStats {
    pub packets_received: AtomicU64,
    pub packets_dropped: AtomicU64,   // ring buffer full
    pub packets_malformed: AtomicU64, // failed to parse
    pub packets_per_second: AtomicU64,
}

impl IngestStats {
    pub fn snapshot(&self) -> IngestStatsSnapshot {
        IngestStatsSnapshot {
            received: self.packets_received.load(Ordering::Relaxed),
            dropped: self.packets_dropped.load(Ordering::Relaxed),
            malformed: self.packets_malformed.load(Ordering::Relaxed),
            pps: self.packets_per_second.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IngestStatsSnapshot {
    pub received: u64,
    pub dropped: u64,
    pub malformed: u64,
    pub pps: u64,
}

/// Handle returned to callers — owns the consumer end of the ring buffer
/// and a reference to live statistics.
pub struct IngestHandle {
    pub rx: Receiver<CsiFrame>,
    pub stats: Arc<IngestStats>,
}

/// Spawn the UDP ingest task.
///
/// Binds to `bind_addr`, parses incoming CSI frames, and pushes them onto
/// an internal bounded channel. Returns an `IngestHandle` for the consumer.
pub async fn spawn_ingest(bind_addr: SocketAddr) -> Result<IngestHandle> {
    let socket = UdpSocket::bind(bind_addr).await?;
    info!("VORAX ingest listening on {}", bind_addr);

    let (tx, rx): (Sender<CsiFrame>, Receiver<CsiFrame>) = bounded(RING_BUFFER_CAPACITY);
    let stats = Arc::new(IngestStats::default());

    let stats_clone = Arc::clone(&stats);

    tokio::spawn(async move {
        ingest_loop(socket, tx, stats_clone).await;
    });

    Ok(IngestHandle { rx, stats })
}

/// Core receive loop — runs in its own tokio task indefinitely.
async fn ingest_loop(socket: UdpSocket, tx: Sender<CsiFrame>, stats: Arc<IngestStats>) {
    let mut buf = [0u8; MAX_DATAGRAM_BYTES];

    // Rate counter state
    let mut rate_window_start = Instant::now();
    let mut rate_count: u64 = 0;

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, src)) => {
                debug!("recv {} bytes from {}", len, src);

                match CsiFrame::from_bytes(&buf[..len]) {
                    Some(frame) => {
                        stats.packets_received.fetch_add(1, Ordering::Relaxed);
                        rate_count += 1;

                        match tx.try_send(frame) {
                            Ok(_) => {}
                            Err(TrySendError::Full(_)) => {
                                // DSP consumer is behind — drop oldest implicitly
                                stats.packets_dropped.fetch_add(1, Ordering::Relaxed);
                                warn!("ring buffer full — frame dropped (DSP consumer lagging)");
                            }
                            Err(TrySendError::Disconnected(_)) => {
                                error!("consumer channel disconnected — shutting down ingest");
                                return;
                            }
                        }
                    }
                    None => {
                        stats.packets_malformed.fetch_add(1, Ordering::Relaxed);
                        debug!("malformed packet from {} ({} bytes) — discarded", src, len);
                    }
                }

                // Update PPS counter every second
                let elapsed = rate_window_start.elapsed();
                if elapsed >= Duration::from_secs(1) {
                    let pps = (rate_count as f64 / elapsed.as_secs_f64()) as u64;
                    stats.packets_per_second.store(pps, Ordering::Relaxed);
                    rate_window_start = Instant::now();
                    rate_count = 0;
                }
            }
            Err(e) => {
                error!("UDP recv error: {}", e);
                // Don't exit on transient errors — log and continue
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }
    }
}
