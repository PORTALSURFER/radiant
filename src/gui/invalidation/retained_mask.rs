use super::InvalidationMask;

/// Invalidation mask for retained render segments with predeclared groups.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RetainedSegmentMask<
    const VALID_MASK: u16,
    const STATIC_MASK: u16,
    const OVERLAY_MASK: u16,
> {
    mask: InvalidationMask,
}

impl<const VALID_MASK: u16, const STATIC_MASK: u16, const OVERLAY_MASK: u16>
    RetainedSegmentMask<VALID_MASK, STATIC_MASK, OVERLAY_MASK>
{
    /// Return an empty retained segment mask.
    pub const fn empty() -> Self {
        Self {
            mask: InvalidationMask::empty(),
        }
    }

    /// Return a mask containing every valid segment bit.
    pub const fn all() -> Self {
        Self {
            mask: InvalidationMask::all(VALID_MASK),
        }
    }

    /// Construct a retained segment mask from raw bits.
    pub const fn from_bits(bits: u16) -> Self {
        Self {
            mask: InvalidationMask::from_bits(bits, VALID_MASK),
        }
    }

    /// Return raw bit contents for diagnostics and tests.
    pub const fn bits(self) -> u16 {
        self.mask.bits()
    }

    /// Return `true` when the mask contains no segments.
    pub const fn is_empty(self) -> bool {
        self.mask.is_empty()
    }

    /// Return `true` when any static segment requires rebuild.
    pub const fn requires_static_rebuild(self) -> bool {
        self.mask.intersects(STATIC_MASK)
    }

    /// Return `true` when any overlay segment requires rebuild.
    pub const fn requires_overlay_rebuild(self) -> bool {
        self.mask.intersects(OVERLAY_MASK)
    }

    /// Insert one or more valid segment bits into this mask.
    pub fn insert(&mut self, bits: u16) {
        self.mask.insert(bits, VALID_MASK);
    }
}
