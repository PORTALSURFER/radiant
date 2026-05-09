//! Serializable visual snapshot primitives for deterministic GUI fixtures.

mod convert;
mod types;

pub use convert::visual_snapshot_from_paint_frame;
pub use types::{
    SnapshotColor, SnapshotPoint, SnapshotPrimitive, SnapshotRect, SnapshotTextAlign,
    SnapshotTextRun, VisualSnapshot,
};

#[cfg(test)]
mod tests;
