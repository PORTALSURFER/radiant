use super::InvalidationMask;

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
