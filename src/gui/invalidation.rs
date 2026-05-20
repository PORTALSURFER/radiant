//! Domain-neutral invalidation masks for retained UI rebuild decisions.

mod mask;
mod retained_mask;
mod segment;

#[cfg(test)]
#[path = "invalidation/tests.rs"]
mod tests;

pub use mask::InvalidationMask;
pub use retained_mask::RetainedSegmentMask;
pub use segment::{
    RetainedSegment, RetainedSegmentKind, RetainedSegmentPlan, RetainedSegmentRevisions,
};
