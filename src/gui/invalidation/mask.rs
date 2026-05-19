/// Bitmask clipped to a caller-defined set of valid invalidation flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InvalidationMask {
    bits: u16,
}

impl InvalidationMask {
    /// Return an empty invalidation mask.
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    /// Return a mask containing every valid bit.
    pub const fn all(valid_mask: u16) -> Self {
        Self { bits: valid_mask }
    }

    /// Construct a mask from raw bits, dropping bits outside `valid_mask`.
    pub const fn from_bits(bits: u16, valid_mask: u16) -> Self {
        Self {
            bits: bits & valid_mask,
        }
    }

    /// Return raw bit contents for diagnostics and tests.
    pub const fn bits(self) -> u16 {
        self.bits
    }

    /// Return `true` when the mask contains no invalidation flags.
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    /// Return `true` when at least one bit from `group_mask` is present.
    pub const fn intersects(self, group_mask: u16) -> bool {
        (self.bits & group_mask) != 0
    }

    /// Insert one or more valid bits into this mask.
    pub fn insert(&mut self, bits: u16, valid_mask: u16) {
        self.bits |= bits & valid_mask;
    }
}
