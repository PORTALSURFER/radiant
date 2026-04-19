//! Sources-list hotkeys for source-row maintenance and navigation.

use crate::app::UiAction;
use crate::gui::input::KeyCode;

use super::super::{HotkeyBinding, HotkeyGesture, SOURCES_SCOPE};

pub(crate) const MOVE_SOURCE_FOCUS_UP: HotkeyBinding = HotkeyBinding {
    id: "move-source-focus-up",
    label: "Previous source",
    gesture: HotkeyGesture::new(KeyCode::ArrowUp),
    scope: SOURCES_SCOPE,
    action: UiAction::MoveSourceFocus { delta: -1 },
};
pub(crate) const MOVE_SOURCE_FOCUS_DOWN: HotkeyBinding = HotkeyBinding {
    id: "move-source-focus-down",
    label: "Next source",
    gesture: HotkeyGesture::new(KeyCode::ArrowDown),
    scope: SOURCES_SCOPE,
    action: UiAction::MoveSourceFocus { delta: 1 },
};
pub(crate) const RELOAD_FOCUSED_SOURCE: HotkeyBinding = HotkeyBinding {
    id: "reload-focused-source",
    label: "Reload source",
    gesture: HotkeyGesture::new(KeyCode::R),
    scope: SOURCES_SCOPE,
    action: UiAction::ReloadFocusedSourceRow,
};
pub(crate) const HARD_SYNC_FOCUSED_SOURCE: HotkeyBinding = HotkeyBinding {
    id: "hard-sync-focused-source",
    label: "Hard sync source",
    gesture: HotkeyGesture::new(KeyCode::H),
    scope: SOURCES_SCOPE,
    action: UiAction::HardSyncFocusedSourceRow,
};
pub(crate) const OPEN_FOCUSED_SOURCE_FOLDER: HotkeyBinding = HotkeyBinding {
    id: "open-focused-source-folder",
    label: "Open source folder",
    gesture: HotkeyGesture::new(KeyCode::O),
    scope: SOURCES_SCOPE,
    action: UiAction::OpenFocusedSourceFolder,
};
pub(crate) const REMOVE_FOCUSED_SOURCE: HotkeyBinding = HotkeyBinding {
    id: "remove-focused-source",
    label: "Remove source",
    gesture: HotkeyGesture::new(KeyCode::D),
    scope: SOURCES_SCOPE,
    action: UiAction::RemoveFocusedSourceRow,
};
