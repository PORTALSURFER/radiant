//! Backend-neutral interaction and state prelude exports.

pub use crate::gui::{
    input::{KeyCode, KeyPress},
    invalidation::{
        InvalidationMask, RetainedSegment, RetainedSegmentKind, RetainedSegmentMask,
        RetainedSegmentPlan, RetainedSegmentRevisions, RevisionCounter,
    },
    selection::SelectionSet,
    shortcuts::{
        ShortcutGesture, ShortcutLayer, ShortcutModifier, ShortcutResolution, ShortcutStack,
    },
    undo::{UndoCheckpoint, UndoHistory, UndoRedoIntent, UndoTransition},
};
