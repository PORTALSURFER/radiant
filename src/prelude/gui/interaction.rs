//! Backend-neutral interaction and state prelude exports.

pub use crate::gui::{
    input::{KeyCode, KeyPress},
    invalidation::RevisionCounter,
    selection::SelectionSet,
    shortcuts::{
        ShortcutCatalog, ShortcutGesture, ShortcutLayer, ShortcutModifier, ShortcutResolution,
        ShortcutStack,
    },
    undo::{UndoCheckpoint, UndoHistory, UndoRedoIntent, UndoTransition},
};
