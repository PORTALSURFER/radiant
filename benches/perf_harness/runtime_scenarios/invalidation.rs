//! Runtime invalidation performance scenarios.

use radiant::gui::invalidation::{RetainedSegment, RetainedSegmentPlan, RetainedSegmentRevisions};
use std::hint::black_box;

const INVALIDATION_STEPS: usize = 1_024;
const PLAN: RetainedSegmentPlan<8> = RetainedSegmentPlan::new([
    RetainedSegment::static_segment("base"),
    RetainedSegment::static_segment("grid"),
    RetainedSegment::static_segment("labels"),
    RetainedSegment::overlay("hover"),
    RetainedSegment::overlay("selection"),
    RetainedSegment::overlay("cursor"),
    RetainedSegment::overlay("drag"),
    RetainedSegment::overlay("playhead"),
]);

pub(super) fn retained_segment_invalidation_1k() -> impl FnMut() {
    let mut bench = RetainedSegmentInvalidationBench::new();
    move || bench.step()
}

struct RetainedSegmentInvalidationBench {
    revisions: RetainedSegmentRevisions<8>,
    masks: Vec<u16>,
    next: usize,
}

impl RetainedSegmentInvalidationBench {
    fn new() -> Self {
        Self {
            revisions: RetainedSegmentRevisions::default(),
            masks: invalidation_masks(INVALIDATION_STEPS),
            next: 0,
        }
    }

    fn step(&mut self) {
        let mut static_rebuilds = 0_u64;
        let mut overlay_rebuilds = 0_u64;
        for offset in 0..self.masks.len() {
            let mask = PLAN.mask(self.masks[(self.next + offset) % self.masks.len()]);
            if PLAN.requires_static_rebuild(mask) {
                static_rebuilds += 1;
            }
            if PLAN.requires_overlay_rebuild(mask) {
                overlay_rebuilds += 1;
            }
            PLAN.bump_revisions(&mut self.revisions, mask);
        }
        self.next = self.next.wrapping_add(1);
        assert!(self.revisions.has_revisions());
        assert!(static_rebuilds > 0);
        assert!(overlay_rebuilds > 0);
        black_box((self.revisions, static_rebuilds, overlay_rebuilds, self.next));
    }
}

fn invalidation_masks(count: usize) -> Vec<u16> {
    (0..count)
        .map(|index| {
            let bit = 1u16 << (index % 8);
            let neighbor = 1u16 << ((index.wrapping_mul(5).wrapping_add(3)) % 8);
            let invalid = if index % 11 == 0 { 1u16 << 12 } else { 0 };
            bit | neighbor | invalid
        })
        .collect()
}
