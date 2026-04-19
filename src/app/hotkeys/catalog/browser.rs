//! Sample-browser hotkeys and related row-navigation gestures.

use crate::app::UiAction;
use crate::gui::input::KeyCode;

use super::super::{HotkeyBinding, HotkeyGesture, KeyPress, BROWSER_SCOPE};

pub(crate) const SEARCH_BROWSER: HotkeyBinding = HotkeyBinding {
    id: "search-browser",
    label: "Search samples",
    gesture: HotkeyGesture::with_command(KeyCode::F),
    scope: BROWSER_SCOPE,
    action: UiAction::FocusBrowserSearch,
};
pub(crate) const FOCUS_LOADED_SAMPLE: HotkeyBinding = HotkeyBinding {
    id: "focus-loaded-sample",
    label: "Focus loaded sample",
    gesture: HotkeyGesture::new(KeyCode::F),
    scope: BROWSER_SCOPE,
    action: UiAction::FocusLoadedSampleInBrowser,
};
pub(crate) const COPY_BROWSER_SELECTION: HotkeyBinding = HotkeyBinding {
    id: "copy-browser-selection",
    label: "Copy sample file(s)",
    gesture: HotkeyGesture::with_command(KeyCode::C),
    scope: BROWSER_SCOPE,
    action: UiAction::CopySelectionToClipboard,
};
pub(crate) const SET_COMPARE_ANCHOR: HotkeyBinding = HotkeyBinding {
    id: "set-compare-anchor",
    label: "Set compare anchor",
    gesture: HotkeyGesture::new(KeyCode::C),
    scope: BROWSER_SCOPE,
    action: UiAction::SetCompareAnchorFromFocusedBrowserSample,
};
pub(crate) const FIND_SIMILAR: HotkeyBinding = HotkeyBinding {
    id: "find-similar",
    label: "Toggle find similar",
    gesture: HotkeyGesture::new(KeyCode::S),
    scope: BROWSER_SCOPE,
    action: UiAction::ToggleFindSimilarFocusedSample,
};
pub(crate) const TOGGLE_RANDOM_NAVIGATION_MODE: HotkeyBinding = HotkeyBinding {
    id: "toggle-random-navigation-mode",
    label: "Toggle random navigation mode",
    gesture: HotkeyGesture::with_alt(KeyCode::R),
    scope: BROWSER_SCOPE,
    action: UiAction::ToggleRandomNavigationMode,
};
pub(crate) const PLAY_RANDOM_SAMPLE: HotkeyBinding = HotkeyBinding {
    id: "play-random-sample",
    label: "Play random sample",
    gesture: HotkeyGesture::with_shift(KeyCode::R),
    scope: BROWSER_SCOPE,
    action: UiAction::PlayRandomSample,
};
pub(crate) const PLAY_PREVIOUS_RANDOM_SAMPLE: HotkeyBinding = HotkeyBinding {
    id: "play-previous-random-sample",
    label: "Play previous random sample",
    gesture: HotkeyGesture {
        first: KeyPress {
            key: KeyCode::R,
            command: true,
            shift: true,
            alt: false,
        },
        chord: None,
    },
    scope: BROWSER_SCOPE,
    action: UiAction::PlayPreviousRandomSample,
};
pub(crate) const MOVE_TRASHED_TO_FOLDER: HotkeyBinding = HotkeyBinding {
    id: "move-trashed-to-folder",
    label: "Move trashed samples to folder",
    gesture: HotkeyGesture::new(KeyCode::P),
    scope: BROWSER_SCOPE,
    action: UiAction::MoveTrashedSamplesToFolder,
};
pub(crate) const MOVE_TRASHED_TO_FOLDER_SHIFT: HotkeyBinding = HotkeyBinding {
    id: "move-trashed-to-folder-shift",
    label: "Move trashed samples to folder",
    gesture: HotkeyGesture::with_shift(KeyCode::P),
    scope: BROWSER_SCOPE,
    action: UiAction::MoveTrashedSamplesToFolder,
};
pub(crate) const TOGGLE_SELECT: HotkeyBinding = HotkeyBinding {
    id: "toggle-select",
    label: "Toggle selection",
    gesture: HotkeyGesture::new(KeyCode::X),
    scope: BROWSER_SCOPE,
    action: UiAction::ToggleFocusedBrowserRowSelection,
};
pub(crate) const TOGGLE_BROWSER_SAMPLE_MARK: HotkeyBinding = HotkeyBinding {
    id: "toggle-browser-sample-mark",
    label: "Toggle sample mark",
    gesture: HotkeyGesture::new(KeyCode::Semicolon),
    scope: BROWSER_SCOPE,
    action: UiAction::ToggleBrowserSampleMark,
};
pub(crate) const MOVE_BROWSER_FOCUS_UP: HotkeyBinding = HotkeyBinding {
    id: "move-browser-focus-up",
    label: "Move focus up",
    gesture: HotkeyGesture::new(KeyCode::ArrowUp),
    scope: BROWSER_SCOPE,
    action: UiAction::MoveBrowserFocus { delta: -1 },
};
pub(crate) const MOVE_BROWSER_FOCUS_DOWN: HotkeyBinding = HotkeyBinding {
    id: "move-browser-focus-down",
    label: "Move focus down",
    gesture: HotkeyGesture::new(KeyCode::ArrowDown),
    scope: BROWSER_SCOPE,
    action: UiAction::MoveBrowserFocus { delta: 1 },
};
pub(crate) const FOCUS_HISTORY_PREVIOUS: HotkeyBinding = HotkeyBinding {
    id: "focus-history-previous",
    label: "Previous focused sample",
    gesture: HotkeyGesture::new(KeyCode::ArrowLeft),
    scope: BROWSER_SCOPE,
    action: UiAction::FocusPreviousBrowserHistory,
};
pub(crate) const FOCUS_HISTORY_NEXT: HotkeyBinding = HotkeyBinding {
    id: "focus-history-next",
    label: "Next focused sample",
    gesture: HotkeyGesture::new(KeyCode::ArrowRight),
    scope: BROWSER_SCOPE,
    action: UiAction::FocusNextBrowserHistory,
};
pub(crate) const RENAME_SAMPLE: HotkeyBinding = HotkeyBinding {
    id: "rename-sample",
    label: "Rename sample",
    gesture: HotkeyGesture::new(KeyCode::R),
    scope: BROWSER_SCOPE,
    action: UiAction::StartBrowserRename,
};
pub(crate) const SELECT_ALL_BROWSER: HotkeyBinding = HotkeyBinding {
    id: "select-all-browser",
    label: "Select all samples",
    gesture: HotkeyGesture::with_command(KeyCode::A),
    scope: BROWSER_SCOPE,
    action: UiAction::SelectAllBrowserRows,
};
pub(crate) const NORMALIZE_BROWSER: HotkeyBinding = HotkeyBinding {
    id: "normalize-browser",
    label: "Normalize sample",
    gesture: HotkeyGesture::new(KeyCode::N),
    scope: BROWSER_SCOPE,
    action: UiAction::NormalizeFocusedBrowserSample,
};
pub(crate) const DELETE_BROWSER: HotkeyBinding = HotkeyBinding {
    id: "delete-browser",
    label: "Delete sample",
    gesture: HotkeyGesture::new(KeyCode::D),
    scope: BROWSER_SCOPE,
    action: UiAction::DeleteBrowserSelection,
};
