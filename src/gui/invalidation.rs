//! Domain-neutral invalidation masks for retained UI rebuild decisions.

/// Bitmask clipped to a caller-defined set of valid invalidation flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InvalidationMask {
    bits: u16,
}

/// Invalidation mask for retained render segments with predeclared groups.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RetainedSegmentMask<
    const VALID_MASK: u16,
    const STATIC_MASK: u16,
    const OVERLAY_MASK: u16,
> {
    mask: InvalidationMask,
}

/// Monotonic revision counters for retained render segments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetainedSegmentRevisions<const SEGMENTS: usize> {
    /// Per-segment revision counters in host-defined segment order.
    pub revisions: [u64; SEGMENTS],
}

impl<const SEGMENTS: usize> Default for RetainedSegmentRevisions<SEGMENTS> {
    fn default() -> Self {
        Self {
            revisions: [0; SEGMENTS],
        }
    }
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

impl<const SEGMENTS: usize> RetainedSegmentRevisions<SEGMENTS> {
    /// Build retained segment revisions from explicit counters.
    pub const fn new(revisions: [u64; SEGMENTS]) -> Self {
        Self { revisions }
    }

    /// Return whether any revision counter is non-zero.
    pub fn has_revisions(self) -> bool {
        self.revisions.iter().any(|revision| *revision != 0)
    }

    /// Bump revisions whose matching segment bits are present.
    pub fn bump_for_bits(&mut self, bits: u16, segment_bits: [u16; SEGMENTS]) {
        for (revision, segment_bit) in self.revisions.iter_mut().zip(segment_bits) {
            if (bits & segment_bit) != 0 {
                *revision = revision.saturating_add(1);
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::{InvalidationMask, RetainedSegmentMask, RetainedSegmentRevisions};

    const VALID_MASK: u16 = 0b0111;

    #[test]
    fn invalidation_mask_clips_to_valid_bits() {
        let mask = InvalidationMask::from_bits(0b1111, VALID_MASK);

        assert_eq!(mask.bits(), VALID_MASK);
    }

    #[test]
    fn invalidation_mask_reports_intersections() {
        let mask = InvalidationMask::from_bits(0b0101, VALID_MASK);

        assert!(mask.intersects(0b0001));
        assert!(!mask.intersects(0b0010));
    }

    #[test]
    fn invalidation_mask_insert_preserves_only_valid_bits() {
        let mut mask = InvalidationMask::empty();

        mask.insert(0b1010, VALID_MASK);

        assert_eq!(mask.bits(), 0b0010);
    }

    #[test]
    fn retained_segment_mask_tracks_static_overlay_and_valid_bits() {
        type Mask = RetainedSegmentMask<0b1111, 0b0011, 0b1100>;

        let mut mask = Mask::from_bits(0b1_1111);
        assert_eq!(mask.bits(), 0b1111);
        assert!(mask.requires_static_rebuild());
        assert!(mask.requires_overlay_rebuild());

        mask = Mask::empty();
        mask.insert(0b10000);
        assert!(mask.is_empty());
        mask.insert(0b0100);
        assert!(!mask.requires_static_rebuild());
        assert!(mask.requires_overlay_rebuild());
    }

    #[test]
    fn retained_segment_revisions_report_and_bump_changed_segments() {
        let mut revisions = RetainedSegmentRevisions::<3>::default();

        assert!(!revisions.has_revisions());
        revisions.bump_for_bits(0b101, [0b001, 0b010, 0b100]);

        assert_eq!(revisions.revisions, [1, 0, 1]);
        assert!(revisions.has_revisions());
    }
}
