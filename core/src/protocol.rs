//! VORAX Core — CSI Packet Protocol Types
//!
//! These structs mirror the ESP32-S3 firmware wire format exactly.
//! Any change here MUST be reflected in the firmware's C struct definition.

use std::fmt;

/// Magic bytes identifying a valid VORAX CSI frame: ASCII "VX"
pub const PACKET_MAGIC: u16 = 0x5658;

/// Maximum subcarriers per packet (64 for HT20, 128 for HT40)
pub const MAX_SUBCARRIERS: usize = 128;

/// Raw CSI packet header — matches `#[repr(C, packed)]` on firmware side.
/// Total size: 20 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct CsiPacketHeader {
    /// Magic identifier: 0x5658 ("VX") — drop any packet where this doesn't match
    pub magic: u16,
    /// Unique sensor node identifier (0–65535)
    pub node_id: u16,
    /// Monotonically increasing sequence number — detect dropped packets
    pub sequence_number: u32,
    /// Hardware microsecond timestamp from esp_timer_get_time()
    pub timestamp_us: u64,
    /// WiFi channel index (e.g. 6 for 2.4 GHz channel 6)
    pub channel: u8,
    /// Received Signal Strength Indicator (dBm, signed)
    pub rssi: i8,
    /// Measured noise floor (dBm, signed)
    pub noise_floor: i8,
    /// Number of subcarriers in this frame (64 for HT20, 128 for HT40)
    pub subcarrier_count: u8,
}

impl CsiPacketHeader {
    /// Wire size of this header in bytes
    pub const WIRE_SIZE: usize = std::mem::size_of::<CsiPacketHeader>();

    /// Returns true if magic bytes are valid
    pub fn is_valid(&self) -> bool {
        self.magic == PACKET_MAGIC
    }

    /// Deserialize from a raw byte slice (little-endian, packed).
    /// Returns None if the slice is too short or magic is invalid.
    pub fn from_bytes(buf: &[u8]) -> Option<Self> {
        if buf.len() < Self::WIRE_SIZE {
            return None;
        }
        // SAFETY: repr(C, packed) with known size; buf is long enough.
        let header = unsafe {
            std::ptr::read_unaligned(buf.as_ptr() as *const CsiPacketHeader)
        };
        if header.is_valid() { Some(header) } else { None }
    }
}

impl fmt::Display for CsiPacketHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "node={} seq={} ts={}µs ch={} rssi={}dBm nf={}dBm sc={}",
            { self.node_id },
            { self.sequence_number },
            { self.timestamp_us },
            self.channel,
            self.rssi,
            self.noise_floor,
            self.subcarrier_count,
        )
    }
}

/// A single OFDM subcarrier's complex I/Q sample.
/// Firmware packs imaginary before real (matches ESP-IDF CSI callback order).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C, packed)]
pub struct SubcarrierComplex {
    pub imaginary: i8,
    pub real: i8,
}

impl SubcarrierComplex {
    /// Amplitude: sqrt(I² + Q²) as f32
    #[inline(always)]
    pub fn amplitude(self) -> f32 {
        let i = self.imaginary as f32;
        let r = self.real as f32;
        (i * i + r * r).sqrt()
    }

    /// Phase angle in radians: atan2(Q, I)
    #[inline(always)]
    pub fn phase(self) -> f32 {
        (self.imaginary as f32).atan2(self.real as f32)
    }
}

/// A fully parsed CSI frame: header + subcarrier payload.
#[derive(Debug, Clone)]
pub struct CsiFrame {
    pub header: CsiPacketHeader,
    /// Complex I/Q pairs, one per subcarrier. Length == header.subcarrier_count.
    pub subcarriers: Vec<SubcarrierComplex>,
    /// Wall-clock arrival time (for jitter measurement)
    pub received_at: std::time::Instant,
}

impl CsiFrame {
    /// Total wire size for a frame with `subcarrier_count` subcarriers
    pub fn wire_size(subcarrier_count: u8) -> usize {
        CsiPacketHeader::WIRE_SIZE + (subcarrier_count as usize) * std::mem::size_of::<SubcarrierComplex>()
    }

    /// Deserialize a complete frame from raw UDP payload bytes.
    pub fn from_bytes(buf: &[u8]) -> Option<Self> {
        let header = CsiPacketHeader::from_bytes(buf)?;
        let sc_count = header.subcarrier_count as usize;
        let expected_len = Self::wire_size(header.subcarrier_count);

        if buf.len() < expected_len {
            return None;
        }

        let sc_bytes = &buf[CsiPacketHeader::WIRE_SIZE..expected_len];
        let subcarriers: Vec<SubcarrierComplex> = sc_bytes
            .chunks_exact(2)
            .take(sc_count)
            .map(|chunk| SubcarrierComplex {
                imaginary: chunk[0] as i8,
                real: chunk[1] as i8,
            })
            .collect();

        Some(CsiFrame {
            header,
            subcarriers,
            received_at: std::time::Instant::now(),
        })
    }

    /// Compute per-subcarrier amplitude vector (f32)
    pub fn amplitudes(&self) -> Vec<f32> {
        self.subcarriers.iter().map(|sc| sc.amplitude()).collect()
    }

    /// Compute per-subcarrier phase vector (radians, f32) — NOT unwrapped yet
    pub fn phases_raw(&self) -> Vec<f32> {
        self.subcarriers.iter().map(|sc| sc.phase()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_wire_size_is_20_bytes() {
        assert_eq!(CsiPacketHeader::WIRE_SIZE, 20);
    }

    #[test]
    fn subcarrier_amplitude_and_phase() {
        let sc = SubcarrierComplex { real: 3, imaginary: 4 };
        assert!((sc.amplitude() - 5.0).abs() < 0.001);
        assert!((sc.phase() - (4.0f32).atan2(3.0)).abs() < 0.001);
    }

    #[test]
    fn invalid_magic_rejected() {
        let mut buf = [0u8; 18];
        buf[0] = 0xFF; buf[1] = 0xFF; // bad magic
        assert!(CsiPacketHeader::from_bytes(&buf).is_none());
    }
}
