//! Source-folder pane hotkeys and folder-tree navigation gestures.

use crate::app::UiAction;
use crate::gui::input::KeyCode;

use super::super::{FOLDERS_SCOPE, HotkeyBinding, HotkeyGesture};

pub(crate) const TOGGLE_FOLDER_SELECT: HotkeyBinding = HotkeyBinding {
    id: "toggle-folder-select",
    label: "Toggle folder selection",
    gesture: HotkeyGesture::new(KeyCode::X),
    scope: FOLDERS_SCOPE,
    action: UiAction::ToggleFocusedFolderSelection,
};
pub(crate) const MOVE_FOLDER_FOCUS_UP: HotkeyBinding = HotkeyBinding {
    id: "move-folder-focus-up",
    label: "Move focus up",
    gesture: HotkeyGesture::new(KeyCode::ArrowUp),
    scope: FOLDERS_SCOPE,
    action: UiAction::MoveFolderFocus { delta: -1 },
};
pub(crate) const MOVE_FOLDER_FOCUS_DOWN: HotkeyBinding = HotkeyBinding {
    id: "move-folder-focus-down",
    label: "Move focus down",
    gesture: HotkeyGesture::new(KeyCode::ArrowDown),
    scope: FOLDERS_SCOPE,
    action: UiAction::MoveFolderFocus { delta: 1 },
};
pub(crate) const COLLAPSE_FOCUSED_FOLDER: HotkeyBinding = HotkeyBinding {
    id: "collapse-focused-folder",
    label: "Collapse folder",
    gesture: HotkeyGesture::new(KeyCode::ArrowLeft),
    scope: FOLDERS_SCOPE,
    action: UiAction::CollapseFocusedFolder,
};
pub(crate) const EXPAND_FOCUSED_FOLDER: HotkeyBinding = HotkeyBinding {
    id: "expand-focused-folder",
    label: "Expand folder",
    gesture: HotkeyGesture::new(KeyCode::ArrowRight),
    scope: FOLDERS_SCOPE,
    action: UiAction::ExpandFocusedFolder,
};
pub(crate) const DELETE_FOLDER: HotkeyBinding = HotkeyBinding {
    id: "delete-folder",
    label: "Delete folder",
    gesture: HotkeyGesture::new(KeyCode::D),
    scope: FOLDERS_SCOPE,
    action: UiAction::DeleteFocusedFolder,
};
pub(crate) const RENAME_FOLDER: HotkeyBinding = HotkeyBinding {
    id: "rename-folder",
    label: "Rename folder",
    gesture: HotkeyGesture::new(KeyCode::R),
    scope: FOLDERS_SCOPE,
    action: UiAction::StartFolderRename,
};
pub(crate) const NEW_FOLDER: HotkeyBinding = HotkeyBinding {
    id: "new-folder",
    label: "New folder",
    gesture: HotkeyGesture::new(KeyCode::N),
    scope: FOLDERS_SCOPE,
    action: UiAction::StartNewFolder,
};
pub(crate) const SEARCH_FOLDERS: HotkeyBinding = HotkeyBinding {
    id: "search-folders",
    label: "Search folders",
    gesture: HotkeyGesture::with_command(KeyCode::F),
    scope: FOLDERS_SCOPE,
    action: UiAction::FocusFolderSearch { pane: None },
};
