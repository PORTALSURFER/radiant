//! Stable fingerprint builder for retained UI cache invalidation.

use crate::gui::types::Rgba8;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Deterministic 64-bit fingerprint builder for retained UI cache keys.
///
/// This is intentionally small and allocation-free. It is not a cryptographic
/// hash; use it for invalidation signatures where stability and cheap mixing
/// matter more than adversarial collision resistance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StableFingerprint {
    state: u64,
}

impl Default for StableFingerprint {
    fn default() -> Self {
        Self::new()
    }
}

impl StableFingerprint {
    /// Create an empty fingerprint builder.
    pub const fn new() -> Self {
        Self {
            state: FNV_OFFSET_BASIS,
        }
    }

    /// Return the completed fingerprint value.
    pub const fn finish(self) -> u64 {
        self.state
    }

    /// Mix one byte into the fingerprint.
    pub fn mix_u8(&mut self, value: u8) {
        self.state ^= u64::from(value);
        self.state = self.state.wrapping_mul(FNV_PRIME);
    }

    /// Mix one little-endian `u16` value into the fingerprint.
    pub fn mix_u16(&mut self, value: u16) {
        for byte in value.to_le_bytes() {
            self.mix_u8(byte);
        }
    }

    /// Mix one little-endian `u32` value into the fingerprint.
    pub fn mix_u32(&mut self, value: u32) {
        for byte in value.to_le_bytes() {
            self.mix_u8(byte);
        }
    }

    /// Mix one little-endian `u64` value into the fingerprint.
    pub fn mix_u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.mix_u8(byte);
        }
    }

    /// Mix one `usize` value into the fingerprint.
    pub fn mix_usize(&mut self, value: usize) {
        self.mix_u64(value as u64);
    }

    /// Mix one boolean value into the fingerprint.
    pub fn mix_bool(&mut self, value: bool) {
        self.mix_u8(u8::from(value));
    }

    /// Mix one `f32` bit pattern into the fingerprint.
    pub fn mix_f32(&mut self, value: f32) {
        self.mix_u32(value.to_bits());
    }

    /// Mix one UTF-8 string into the fingerprint.
    pub fn mix_str(&mut self, value: &str) {
        self.mix_usize(value.len());
        for byte in value.as_bytes() {
            self.mix_u8(*byte);
        }
    }

    /// Mix an optional string into the fingerprint.
    pub fn mix_option_str(&mut self, value: Option<&str>) {
        if let Some(value) = value {
            self.mix_bool(true);
            self.mix_str(value);
            return;
        }
        self.mix_bool(false);
    }

    /// Mix an optional `usize` into the fingerprint.
    pub fn mix_option_usize(&mut self, value: Option<usize>) {
        if let Some(value) = value {
            self.mix_bool(true);
            self.mix_usize(value);
            return;
        }
        self.mix_bool(false);
    }

    /// Mix an optional `u16` into the fingerprint.
    pub fn mix_option_u16(&mut self, value: Option<u16>) {
        if let Some(value) = value {
            self.mix_bool(true);
            self.mix_u16(value);
            return;
        }
        self.mix_bool(false);
    }

    /// Mix an optional `u32` into the fingerprint.
    pub fn mix_option_u32(&mut self, value: Option<u32>) {
        if let Some(value) = value {
            self.mix_bool(true);
            self.mix_u32(value);
            return;
        }
        self.mix_bool(false);
    }

    /// Mix an optional `i8` into the fingerprint.
    pub fn mix_option_i8(&mut self, value: Option<i8>) {
        if let Some(value) = value {
            self.mix_bool(true);
            self.mix_u8(value as u8);
            return;
        }
        self.mix_bool(false);
    }

    /// Mix one RGBA color into the fingerprint.
    pub fn mix_rgba8(&mut self, color: Rgba8) {
        self.mix_u8(color.r);
        self.mix_u8(color.g);
        self.mix_u8(color.b);
        self.mix_u8(color.a);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprints_are_stable_for_identical_inputs() {
        let mut first = StableFingerprint::new();
        first.mix_str("button");
        first.mix_u32(42);
        first.mix_bool(true);

        let mut second = StableFingerprint::new();
        second.mix_str("button");
        second.mix_u32(42);
        second.mix_bool(true);

        assert_eq!(first.finish(), second.finish());
    }

    #[test]
    fn option_presence_changes_fingerprint() {
        let mut present = StableFingerprint::new();
        present.mix_option_str(Some("hover"));

        let mut missing = StableFingerprint::new();
        missing.mix_option_str(None);

        assert_ne!(present.finish(), missing.finish());
    }

    #[test]
    fn color_channels_affect_fingerprint() {
        let mut first = StableFingerprint::new();
        first.mix_rgba8(Rgba8 {
            r: 1,
            g: 2,
            b: 3,
            a: 4,
        });

        let mut changed = StableFingerprint::new();
        changed.mix_rgba8(Rgba8 {
            r: 1,
            g: 2,
            b: 3,
            a: 5,
        });

        assert_ne!(first.finish(), changed.finish());
    }
}
