//! VORAX 4-Node Spatial RF Mesh Simulator
//!
//! Models a multi-static Wi-Fi sensing mesh in a 6m x 6m monitored room:
//! - 4 Fixed Anchor Nodes at room corners
//! - Moving human target with walking trajectory + chest breathing micro-displacement
//! - Dynamic Fresnel zone path lengths, Doppler shifts, and per-subcarrier phase/amplitude synthesis
//! - Generates valid 20-byte VORAX binary frames for each node

use std::f32::consts::PI;
use super::protocol::{CsiPacketHeader, SubcarrierComplex, PACKET_MAGIC};

pub const SPEED_OF_LIGHT_M_S: f32 = 299_792_458.0;
pub const CARRIER_FREQ_HZ: f32 = 2_412_000_000.0; // 2.412 GHz (Channel 1)
pub const SUBCARRIER_SPACING_HZ: f32 = 312_500.0; // 312.5 kHz OFDM spacing
pub const SUBCARRIER_COUNT: u8 = 64;

/// 3D Spatial Vector in meters
#[derive(Debug, Clone, Copy)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn distance_to(&self, other: &Vector3) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Anchor Node in 3D Space
#[derive(Debug, Clone)]
pub struct MeshNode {
    pub id: u16,
    pub position: Vector3,
    pub channel: u8,
    pub sequence_number: u32,
}

impl MeshNode {
    pub fn new(id: u16, x: f32, y: f32, z: f32) -> Self {
        Self {
            id,
            position: Vector3::new(x, y, z),
            channel: 1,
            sequence_number: 0,
        }
    }
}

/// 4-Node Multi-Static Spatial Mesh Simulator
#[derive(Debug, Clone)]
pub struct SpatialMeshSimulator {
    pub nodes: [MeshNode; 4],
    pub tx_position: Vector3,
    pub room_size: (f32, f32),
    pub elapsed_time_sec: f32,
}

impl SpatialMeshSimulator {
    /// Initializes a standard 6m x 6m room with 4 corner anchors mounted at 1.2m height
    pub fn new() -> Self {
        let node1 = MeshNode::new(1, 0.2, 0.2, 1.2); // Corner (0, 0)
        let node2 = MeshNode::new(2, 5.8, 0.2, 1.2); // Corner (6, 0)
        let node3 = MeshNode::new(3, 5.8, 5.8, 1.2); // Corner (6, 6)
        let node4 = MeshNode::new(4, 0.2, 5.8, 1.2); // Corner (0, 6)

        Self {
            nodes: [node1, node2, node3, node4],
            tx_position: Vector3::new(0.2, 0.2, 1.2), // Node 1 is primary TX beacon
            room_size: (6.0, 6.0),
            elapsed_time_sec: 0.0,
        }
    }

    /// Evaluates current target position at time t (walking an elliptical loop in the room)
    pub fn target_position(&self, t: f32) -> Vector3 {
        // Walking trajectory: 2m radius loop around center (3.0, 3.0), cycle period = 10 seconds (~1.25 m/s pace)
        let angle = 2.0 * PI * (t / 10.0);
        let x = 3.0 + 1.8 * angle.cos();
        let y = 3.0 + 1.8 * angle.sin();

        // Superimpose thoracic chest displacement from breathing (0.25 Hz / 15 breaths/min, 4mm amplitude)
        let chest_expansion = 0.004 * (2.0 * PI * 0.25 * t).sin();
        let z = 1.0 + chest_expansion;

        Vector3::new(x, y, z)
    }

    /// Advances simulation time by dt seconds (e.g. 0.005s for 200 Hz)
    pub fn step(&mut self, dt: f32) {
        self.elapsed_time_sec += dt;
    }

    /// Generates a binary packet for a specific node in the mesh
    pub fn generate_node_packet(&mut self, node_index: usize, timestamp_us: u64) -> Vec<u8> {
        let target_pos = self.target_position(self.elapsed_time_sec);
        let node = &mut self.nodes[node_index];
        node.sequence_number = node.sequence_number.wrapping_add(1);

        // Path lengths
        let d_tx_target = self.tx_position.distance_to(&target_pos);
        let d_target_rx = target_pos.distance_to(&node.position);
        let d_reflected = d_tx_target + d_target_rx;
        let d_direct = self.tx_position.distance_to(&node.position).max(0.1);

        // Direct path attenuation & Reflected path attenuation (radar range equation)
        let a_direct = (35.0 / d_direct.sqrt()).clamp(15.0, 45.0);
        let a_refl = (12.0 / (d_tx_target * d_target_rx).sqrt()).clamp(2.0, 16.0);

        // Calculate 64 subcarriers
        let mut subcarriers = Vec::with_capacity(SUBCARRIER_COUNT as usize);

        for k in 0..SUBCARRIER_COUNT {
            let sc_offset = (k as i32 - 32) as f32;
            let f_k = CARRIER_FREQ_HZ + sc_offset * SUBCARRIER_SPACING_HZ;
            let lambda_k = SPEED_OF_LIGHT_M_S / f_k;

            // Direct LoS phase
            let phi_direct = (2.0 * PI * (d_direct % lambda_k) / lambda_k) % (2.0 * PI);
            // Reflected NLoS path phase
            let phi_refl = (2.0 * PI * (d_reflected % lambda_k) / lambda_k) % (2.0 * PI);

            // Vector sum of direct path + moving reflected multipath
            let i_val = a_direct * phi_direct.cos() + a_refl * phi_refl.cos();
            let q_val = a_direct * phi_direct.sin() + a_refl * phi_refl.sin();

            // Clamp into signed 8-bit integers
            let i_clamped = i_val.clamp(-127.0, 127.0) as i8;
            let q_clamped = q_val.clamp(-127.0, 127.0) as i8;

            subcarriers.push(SubcarrierComplex {
                imaginary: q_clamped,
                real: i_clamped,
            });
        }

        // Build 20-byte VORAX wire header
        let header = CsiPacketHeader {
            magic: PACKET_MAGIC,
            node_id: node.id,
            sequence_number: node.sequence_number,
            timestamp_us,
            channel: node.channel,
            rssi: -55 + (node_index as i8 * 3),
            noise_floor: -92,
            subcarrier_count: SUBCARRIER_COUNT,
        };

        // Serialize into raw byte buffer
        let total_size = CsiPacketHeader::WIRE_SIZE + (SUBCARRIER_COUNT as usize * 2);
        let mut buffer = Vec::with_capacity(total_size);

        // Serialize header struct fields (Little-Endian)
        buffer.extend_from_slice(&header.magic.to_le_bytes());
        buffer.extend_from_slice(&header.node_id.to_le_bytes());
        buffer.extend_from_slice(&header.sequence_number.to_le_bytes());
        buffer.extend_from_slice(&header.timestamp_us.to_le_bytes());
        buffer.push(header.channel);
        buffer.push(header.rssi as u8);
        buffer.push(header.noise_floor as u8);
        buffer.push(header.subcarrier_count);

        // Serialize subcarrier I/Q bytes
        for sc in subcarriers {
            buffer.push(sc.imaginary as u8);
            buffer.push(sc.real as u8);
        }

        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_generates_valid_packets_for_all_nodes() {
        let mut sim = SpatialMeshSimulator::new();

        for node_idx in 0..4 {
            let bytes = sim.generate_node_packet(node_idx, 1_000_000);
            assert_eq!(bytes.len(), 20 + 64 * 2);

            let header = CsiPacketHeader::from_bytes(&bytes).expect("Header should be valid");
            let node_id = header.node_id;
            let subcarrier_count = header.subcarrier_count;
            assert_eq!(node_id, (node_idx as u16) + 1);
            assert_eq!(subcarrier_count, 64);
        }
    }

    #[test]
    fn test_target_movement_produces_phase_differentials_across_nodes() {
        let mut sim = SpatialMeshSimulator::new();

        let pkt_node1_t0 = sim.generate_node_packet(0, 0);
        let pkt_node3_t0 = sim.generate_node_packet(2, 0);

        // Advance 1 second (target moves ~1.25m)
        sim.step(1.0);
        let pkt_node1_t1 = sim.generate_node_packet(0, 1_000_000);
        let pkt_node3_t1 = sim.generate_node_packet(2, 1_000_000);

        // Verify subcarriers change dynamically over motion
        assert_ne!(pkt_node1_t0[20..30], pkt_node1_t1[20..30]);
        assert_ne!(pkt_node3_t0[20..30], pkt_node3_t1[20..30]);
    }
}
