/// Supported native animation and presentation target cadences.
///
/// These values are maximum requested cadences. The effective cadence remains
/// bounded by the display capability and native present mode.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FrameRate {
    /// Request a maximum cadence of 30 frames per second.
    Hz30,
    /// Request a maximum cadence of 60 frames per second.
    Hz60,
    /// Request a maximum cadence of 120 frames per second.
    Hz120,
}

impl FrameRate {
    /// Return the native target-fps value represented by this cadence.
    pub const fn as_u32(self) -> u32 {
        match self {
            Self::Hz30 => 30,
            Self::Hz60 => 60,
            Self::Hz120 => 120,
        }
    }

    /// Recover a typed cadence when a raw target-fps value is supported.
    pub const fn from_u32(target_fps: u32) -> Option<Self> {
        match target_fps {
            30 => Some(Self::Hz30),
            60 => Some(Self::Hz60),
            120 => Some(Self::Hz120),
            _ => None,
        }
    }
}
