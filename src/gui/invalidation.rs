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

/// Role of one named retained render segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetainedSegmentKind {
    /// Stable content that usually requires scene or surface rebuild work.
    Static,
    /// Lightweight overlay content that can often repaint without rebuilding stable content.
    Overlay,
}

/// Named retained render segment metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetainedSegment {
    /// Human-readable segment name for diagnostics.
    pub name: &'static str,
    /// Segment role used to group static and overlay invalidations.
    pub kind: RetainedSegmentKind,
}

impl RetainedSegment {
    /// Build a static retained segment.
    pub const fn static_segment(name: &'static str) -> Self {
        Self {
            name,
            kind: RetainedSegmentKind::Static,
        }
    }

    /// Build an overlay retained segment.
    pub const fn overlay(name: &'static str) -> Self {
        Self {
            name,
            kind: RetainedSegmentKind::Overlay,
        }
    }
}

/// Named retained render segment plan with generated bit assignments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetainedSegmentPlan<const SEGMENTS: usize> {
    segments: [RetainedSegment; SEGMENTS],
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

impl<const SEGMENTS: usize> RetainedSegmentPlan<SEGMENTS> {
    /// Build a segment plan.
    ///
    /// At most 16 segments are represented because masks use `u16` bits.
    pub const fn new(segments: [RetainedSegment; SEGMENTS]) -> Self {
        assert!(SEGMENTS <= 16);
        Self { segments }
    }

    /// Return all segment metadata.
    pub const fn segments(&self) -> &[RetainedSegment; SEGMENTS] {
        &self.segments
    }

    /// Return the bit assigned to a segment index.
    pub const fn bit(index: usize) -> Option<u16> {
        if index < SEGMENTS && index < 16 {
            Some(1u16 << index)
        } else {
            None
        }
    }

    /// Return the mask containing every valid segment bit.
    pub const fn valid_mask(&self) -> u16 {
        if SEGMENTS == 16 {
            u16::MAX
        } else {
            (1u16 << SEGMENTS) - 1
        }
    }

    /// Return the mask containing all static segments.
    pub const fn static_mask(&self) -> u16 {
        let mut index = 0;
        let mut mask = 0;
        while index < SEGMENTS {
            if matches!(self.segments[index].kind, RetainedSegmentKind::Static) {
                mask |= 1u16 << index;
            }
            index += 1;
        }
        mask
    }

    /// Return the mask containing all overlay segments.
    pub const fn overlay_mask(&self) -> u16 {
        let mut index = 0;
        let mut mask = 0;
        while index < SEGMENTS {
            if matches!(self.segments[index].kind, RetainedSegmentKind::Overlay) {
                mask |= 1u16 << index;
            }
            index += 1;
        }
        mask
    }

    /// Build a clipped invalidation mask from raw bits.
    pub const fn mask(&self, bits: u16) -> InvalidationMask {
        InvalidationMask::from_bits(bits, self.valid_mask())
    }

    /// Return an invalidation mask for one segment index.
    pub const fn mask_for_index(&self, index: usize) -> Option<InvalidationMask> {
        match Self::bit(index) {
            Some(bit) => Some(self.mask(bit)),
            None => None,
        }
    }

    /// Return an invalidation mask for one named segment.
    pub fn mask_for_name(&self, name: &str) -> Option<InvalidationMask> {
        self.segments
            .iter()
            .position(|segment| segment.name == name)
            .and_then(|index| self.mask_for_index(index))
    }

    /// Return whether a mask invalidates at least one static segment.
    pub const fn requires_static_rebuild(&self, mask: InvalidationMask) -> bool {
        mask.intersects(self.static_mask())
    }

    /// Return whether a mask invalidates at least one overlay segment.
    pub const fn requires_overlay_rebuild(&self, mask: InvalidationMask) -> bool {
        mask.intersects(self.overlay_mask())
    }

    /// Bump revisions for segments contained in `mask`.
    pub fn bump_revisions(
        &self,
        revisions: &mut RetainedSegmentRevisions<SEGMENTS>,
        mask: InvalidationMask,
    ) {
        let mut bits = [0; SEGMENTS];
        let mut index = 0;
        while index < SEGMENTS {
            bits[index] = Self::bit(index).unwrap_or(0);
            index += 1;
        }
        revisions.bump_for_bits(mask.bits(), bits);
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
    use super::{
        InvalidationMask, RetainedSegment, RetainedSegmentMask, RetainedSegmentPlan,
        RetainedSegmentRevisions,
    };

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

    #[test]
    fn retained_segment_plan_names_groups_and_bumps_revisions() {
        const PLAN: RetainedSegmentPlan<3> = RetainedSegmentPlan::new([
            RetainedSegment::static_segment("base"),
            RetainedSegment::overlay("hover"),
            RetainedSegment::overlay("playhead"),
        ]);
        let mut revisions = RetainedSegmentRevisions::<3>::default();

        assert_eq!(PLAN.valid_mask(), 0b111);
        assert_eq!(PLAN.static_mask(), 0b001);
        assert_eq!(PLAN.overlay_mask(), 0b110);

        let hover = PLAN.mask_for_name("hover").expect("hover segment");
        assert!(!PLAN.requires_static_rebuild(hover));
        assert!(PLAN.requires_overlay_rebuild(hover));

        PLAN.bump_revisions(&mut revisions, hover);
        assert_eq!(revisions.revisions, [0, 1, 0]);
        assert_eq!(PLAN.mask(0b1000).bits(), 0);
    }
}
