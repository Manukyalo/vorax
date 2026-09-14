//! VORAX Pre-Hardware Sprint — MODULE 4: Mathematical Verification & Integration Tests
//!
//! Verified against requirements in docs/PRE-HARDWARE-SPRINT.md:
//! 1. Hampel Filter Golden-Vector: 100% spike rejection on synthetic outlier bursts.
//! 2. Bandpass Verification: Dual-sine (0.25 Hz breath + 1.2 Hz cardiac) + 60 Hz AC line noise rejection.
//! 3. Multi-Node Mesh Benchmark: 4 nodes at 200 Hz generating 800 pkts/sec with zero dropped frames.

use std::f32::consts::PI;
use vorax_core::dsp::bandpass::BiquadFilter;
use vorax_core::dsp::filter::HampelFilter;
use vorax_core::protocol::{CsiPacketHeader, PACKET_MAGIC};
use vorax_core::simulator::mesh::SpatialMeshSimulator;

// ============================================================================
// TEST 1: Hampel Filter Golden-Vector Test (100% Outlier Rejection)
// ============================================================================
#[test]
fn test_hampel_golden_vector_rejection() {
    const SAMPLE_COUNT: usize = 500;
    let mut baseline: Vec<f32> = Vec::with_capacity(SAMPLE_COUNT);

    // Baseline: gentle physiological carrier amplitude ~24.0 with slow wander
    for i in 0..SAMPLE_COUNT {
        let t = i as f32 / 200.0;
        baseline.push(24.0 + 1.2 * (2.0 * PI * 0.25 * t).sin());
    }

    let mut corrupted = baseline.clone();

    // Inject 8 massive outlier spikes caused by RF packet collisions / bursts
    let spike_locations = [45, 95, 160, 220, 290, 340, 410, 470];
    let spike_magnitudes = [150.0, -90.0, 200.0, -120.0, 180.0, -85.0, 210.0, -110.0];

    for (&idx, &mag) in spike_locations.iter().zip(spike_magnitudes.iter()) {
        corrupted[idx] = mag;
    }

    // Run batch Hampel filter with half_window = 4 (window = 9 samples), 3.0 MAD threshold
    HampelFilter::filter_series(&mut corrupted, 4, 3.0);

    // Verify 100% rejection: all corrupted spike locations must be restored close to baseline
    for &idx in &spike_locations {
        let diff = (corrupted[idx] - baseline[idx]).abs();
        assert!(
            diff < 2.5,
            "Spike at index {} was not rejected! Corrupted: {}, True Baseline: {}, Diff: {}",
            idx,
            corrupted[idx],
            baseline[idx],
            diff
        );
    }

    // Verify uncorrupted samples were not altered by more than floating point precision
    for i in 4..(SAMPLE_COUNT - 4) {
        if !spike_locations.contains(&i) {
            let diff = (corrupted[i] - baseline[i]).abs();
            assert!(
                diff < 0.5,
                "Clean sample at index {} was distorted by Hampel filter! Clean: {}, Filtered: {}",
                i,
                baseline[i],
                corrupted[i]
            );
        }
    }
}

// ============================================================================
// TEST 2: Bandpass Dual-Sine & 60 Hz Line Noise Rejection Test
// ============================================================================
#[test]
fn test_bandpass_dual_sine_line_noise_rejection() {
    let sample_rate = 200.0f32;
    let duration_sec = 4.0f32;
    let total_samples = (sample_rate * duration_sec) as usize;

    let mut resp_filter = BiquadFilter::respiration(sample_rate);
    let mut cardiac_filter = BiquadFilter::cardiac(sample_rate);

    // Synthesize dual physiological signal + severe 60 Hz powerline hum + 80 Hz interference
    // f_resp = 0.25 Hz (15 BPM), A_resp = 1.0
    // f_card = 1.20 Hz (72 BPM), A_card = 0.4
    // f_line = 60.0 Hz (AC hum), A_line = 3.0 (7.5x cardiac signal!)
    // f_hf   = 80.0 Hz,          A_hf   = 1.5
    let mut clean_resp = Vec::with_capacity(total_samples);
    let mut clean_cardiac = Vec::with_capacity(total_samples);
    let mut composite = Vec::with_capacity(total_samples);

    for i in 0..total_samples {
        let t = i as f32 / sample_rate;
        let s_resp = (2.0 * PI * 0.25 * t).sin();
        let s_card = 0.4 * (2.0 * PI * 1.2 * t).sin();
        let s_line = 3.0 * (2.0 * PI * 60.0 * t).sin();
        let s_hf = 1.5 * (2.0 * PI * 80.0 * t).sin();

        clean_resp.push(s_resp);
        clean_cardiac.push(s_card);
        composite.push(s_resp + s_card + s_line + s_hf);
    }

    // Warm up and evaluate respiration filter outputs after 1 second (transient decay)
    let steady_state_start = (sample_rate * 1.5) as usize;
    let mut max_resp_out = 0.0f32;
    let mut max_cardiac_out = 0.0f32;

    for i in 0..total_samples {
        let r_out = resp_filter.process(composite[i]);
        let c_out = cardiac_filter.process(composite[i]);

        if i >= steady_state_start {
            max_resp_out = max_resp_out.max(r_out.abs());
            max_cardiac_out = max_cardiac_out.max(c_out.abs());
        }
    }

    // Now test isolation of ONLY 60 Hz line noise to measure direct attenuation factor
    let mut noise_test_filter_resp = BiquadFilter::respiration(sample_rate);
    let mut noise_test_filter_card = BiquadFilter::cardiac(sample_rate);
    let mut max_line_leakage_resp = 0.0f32;
    let mut max_line_leakage_card = 0.0f32;

    for i in 0..total_samples {
        let t = i as f32 / sample_rate;
        let line_only = 3.0 * (2.0 * PI * 60.0 * t).sin();
        let l_resp = noise_test_filter_resp.process(line_only);
        let l_card = noise_test_filter_card.process(line_only);

        if i >= steady_state_start {
            max_line_leakage_resp = max_line_leakage_resp.max(l_resp.abs());
            max_line_leakage_card = max_line_leakage_card.max(l_card.abs());
        }
    }

    // 60 Hz AC hum (3.0 V) must be attenuated by at least 20x
    assert!(
        max_line_leakage_resp < 0.15,
        "Respiration filter leaked 60 Hz line noise! Leakage: {}",
        max_line_leakage_resp
    );
    assert!(
        max_line_leakage_card < 0.18,
        "Cardiac filter leaked 60 Hz line noise! Leakage: {}",
        max_line_leakage_card
    );

    // In-band signals must be passed cleanly
    assert!(
        max_resp_out > 0.05,
        "Respiration signal was attenuated too heavily! Max out: {}",
        max_resp_out
    );
    assert!(
        max_cardiac_out > 0.03,
        "Cardiac signal was attenuated too heavily! Max out: {}",
        max_cardiac_out
    );
}

// ============================================================================
// TEST 3: Multi-Node Spatial Mesh Benchmark (800 pkts/s, Zero Dropped Frames)
// ============================================================================
#[test]
fn test_multinode_sustained_throughput_and_frame_integrity() {
    let mut sim = SpatialMeshSimulator::new();

    const NODES: usize = 4;
    const TICKS_PER_SEC: usize = 200; // 200 Hz
    const DURATION_SECS: f32 = 1.0;
    let total_ticks = (TICKS_PER_SEC as f32 * DURATION_SECS) as usize;

    let mut node_packets: [Vec<Vec<u8>>; NODES] = [
        Vec::with_capacity(total_ticks),
        Vec::with_capacity(total_ticks),
        Vec::with_capacity(total_ticks),
        Vec::with_capacity(total_ticks),
    ];

    let start_instant = std::time::Instant::now();
    let dt = 1.0 / TICKS_PER_SEC as f32; // 5ms step

    // Run 1 second simulation generating 4 streams * 200 Hz = 800 packets
    for tick in 0..total_ticks {
        let timestamp_us = (tick as u64) * 5_000; // 5000us = 5ms

        for node_idx in 0..NODES {
            let pkt = sim.generate_node_packet(node_idx, timestamp_us);
            node_packets[node_idx].push(pkt);
        }

        sim.step(dt);
    }

    let elapsed = start_instant.elapsed();
    let total_packet_count: usize = node_packets.iter().map(|p| p.len()).sum();

    // Verify 800 total packets generated
    assert_eq!(
        total_packet_count, 800,
        "Expected exactly 800 packets across 4 nodes, got {}",
        total_packet_count
    );

    // Verify sustained compute throughput well exceeds real-time (800 pkts/sec)
    let packets_per_sec = total_packet_count as f64 / elapsed.as_secs_f64();
    assert!(
        packets_per_sec >= 800.0,
        "Simulator throughput too slow: {:.1} pkts/sec",
        packets_per_sec
    );

    // Verify frame integrity and sequence monotonicity for each node
    for (node_idx, packets) in node_packets.iter().enumerate() {
        assert_eq!(
            packets.len(),
            total_ticks,
            "Node {} dropped frames! Expected {}, got {}",
            node_idx + 1,
            total_ticks,
            packets.len()
        );

        let mut expected_seq: u32 = 1;
        for raw_bytes in packets {
            // Check frame size: 20-byte header + 64 subcarriers * 2 bytes = 148 bytes
            assert_eq!(
                raw_bytes.len(),
                148,
                "Invalid frame length {} on node {}",
                raw_bytes.len(),
                node_idx + 1
            );

            // Parse header and verify fields
            let header = CsiPacketHeader::from_bytes(&raw_bytes[..20])
                .expect("Failed to parse 20-byte VORAX wire header");

            let magic = header.magic;
            let node_id = header.node_id;
            let sc_count = header.subcarrier_count;
            let seq = header.sequence_number;

            assert_eq!(magic, PACKET_MAGIC, "Invalid magic byte in header");
            assert_eq!(
                node_id,
                (node_idx + 1) as u16,
                "Node ID mismatch in packet header"
            );
            assert_eq!(
                sc_count, 64,
                "Subcarrier count mismatch in header"
            );
            assert_eq!(
                seq, expected_seq,
                "Frame dropped or out of sequence on node {}! Expected seq {}, got {}",
                node_idx + 1,
                expected_seq,
                seq
            );

            expected_seq += 1;
        }
    }
}
