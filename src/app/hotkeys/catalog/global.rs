//! Global hotkeys that remain active regardless of focus scope.

use crate::app::{BrowserTagTarget, UiAction};
use crate::gui::input::KeyCode;

use super::super::{HotkeyBinding, HotkeyGesture, KeyPress, GLOBAL_SCOPE};

pub(crate) const UNDO_CTRL_Z: HotkeyBinding = HotkeyBinding {
    id: "undo-ctrl-z",
    label: "Undo",
    gesture: HotkeyGesture::with_command(KeyCode::Z),
    scope: GLOBAL_SCOPE,
    action: UiAction::Undo,
};
pub(crate) const UNDO_U: HotkeyBinding = HotkeyBinding {
    id: "undo-u",
    label: "Undo",
    gesture: HotkeyGesture::new(KeyCode::U),
    scope: GLOBAL_SCOPE,
    action: UiAction::Undo,
};
pub(crate) const REDO_CTRL_Y: HotkeyBinding = HotkeyBinding {
    id: "redo-ctrl-y",
    label: "Redo",
    gesture: HotkeyGesture::with_command(KeyCode::Y),
    scope: GLOBAL_SCOPE,
    action: UiAction::Redo,
};
pub(crate) const REDO_SHIFT_U: HotkeyBinding = HotkeyBinding {
    id: "redo-shift-u",
    label: "Redo",
    gesture: HotkeyGesture::with_shift(KeyCode::U),
    scope: GLOBAL_SCOPE,
    action: UiAction::Redo,
};
pub(crate) const SHOW_HOTKEYS: HotkeyBinding = HotkeyBinding {
    id: "show-hotkeys",
    label: "Show hotkeys",
    gesture: HotkeyGesture::with_command(KeyCode::Slash),
    scope: GLOBAL_SCOPE,
    action: UiAction::ToggleHotkeyOverlay,
};
pub(crate) const COPY_STATUS_LOG: HotkeyBinding = HotkeyBinding {
    id: "copy-status-log",
    label: "Copy status log",
    gesture: HotkeyGesture {
        first: KeyPress {
            key: KeyCode::L,
            command: true,
            shift: true,
            alt: false,
        },
        chord: None,
    },
    scope: GLOBAL_SCOPE,
    action: UiAction::CopyStatusLog,
};
pub(crate) const SUBMIT_GITHUB_ISSUE: HotkeyBinding = HotkeyBinding {
    id: "submit-github-issue",
    label: "Submit GitHub issue",
    gesture: HotkeyGesture::with_shift(KeyCode::F1),
    scope: GLOBAL_SCOPE,
    action: UiAction::OpenFeedbackIssuePrompt,
};
pub(crate) const FOCUS_WAVEFORM: HotkeyBinding = HotkeyBinding {
    id: "focus-waveform",
    label: "Focus waveform",
    gesture: HotkeyGesture::with_chord(KeyPress::new(KeyCode::G), KeyPress::new(KeyCode::W)),
    scope: GLOBAL_SCOPE,
    action: UiAction::FocusWaveformPanel,
};
pub(crate) const FOCUS_BROWSER: HotkeyBinding = HotkeyBinding {
    id: "focus-browser",
    label: "Focus source samples",
    gesture: HotkeyGesture::with_chord(KeyPress::new(KeyCode::G), KeyPress::new(KeyCode::B)),
    scope: GLOBAL_SCOPE,
    action: UiAction::FocusBrowserPanel,
};
pub(crate) const FOCUS_FOLDER_TREE: HotkeyBinding = HotkeyBinding {
    id: "focus-folder-tree",
    label: "Focus folder tree",
    gesture: HotkeyGesture::with_chord(KeyPress::new(KeyCode::G), KeyPress::new(KeyCode::T)),
    scope: GLOBAL_SCOPE,
    action: UiAction::FocusFolderPanel { pane: None },
};
pub(crate) const FOCUS_SOURCES_LIST: HotkeyBinding = HotkeyBinding {
    id: "focus-sources-list",
    label: "Focus sources list",
    gesture: HotkeyGesture::with_chord(KeyPress::new(KeyCode::G), KeyPress::new(KeyCode::S)),
    scope: GLOBAL_SCOPE,
    action: UiAction::FocusSourcesPanel,
};
pub(crate) const PLAY_FROM_START: HotkeyBinding = HotkeyBinding {
    id: "play-from-start",
    label: "Play from start",
    gesture: HotkeyGesture::new(KeyCode::Space),
    scope: GLOBAL_SCOPE,
    action: UiAction::PlayFromStart,
};
pub(crate) const PLAY_COMPARE_ANCHOR: HotkeyBinding = HotkeyBinding {
    id: "play-compare-anchor",
    label: "Play compare anchor",
    gesture: HotkeyGesture::with_shift(KeyCode::Space),
    scope: GLOBAL_SCOPE,
    action: UiAction::PlayCompareAnchor,
};
pub(crate) const PLAY_FROM_CURRENT_PLAYHEAD: HotkeyBinding = HotkeyBinding {
    id: "play-from-current-playhead",
    label: "Play from current playhead",
    gesture: HotkeyGesture::with_command(KeyCode::Space),
    scope: GLOBAL_SCOPE,
    action: UiAction::PlayFromCurrentPlayhead,
};
pub(crate) const TOGGLE_LOOP: HotkeyBinding = HotkeyBinding {
    id: "toggle-loop",
    label: "Toggle loop",
    gesture: HotkeyGesture::new(KeyCode::L),
    scope: GLOBAL_SCOPE,
    action: UiAction::ToggleLoopPlayback,
};
pub(crate) const TOGGLE_LOOP_LOCK: HotkeyBinding = HotkeyBinding {
    id: "toggle-loop-lock",
    label: "Cycle locked loop",
    gesture: HotkeyGesture::with_shift(KeyCode::L),
    scope: GLOBAL_SCOPE,
    action: UiAction::ToggleLoopLock,
};
pub(crate) const RATE_DECREMENT: HotkeyBinding = HotkeyBinding {
    id: "rate-decrement",
    label: "Decrement rating",
    gesture: HotkeyGesture::new(KeyCode::OpenBracket),
    scope: GLOBAL_SCOPE,
    action: UiAction::AdjustSelectedBrowserRating { delta: -1 },
};
pub(crate) const RATE_INCREMENT: HotkeyBinding = HotkeyBinding {
    id: "rate-increment",
    label: "Increment rating",
    gesture: HotkeyGesture::new(KeyCode::CloseBracket),
    scope: GLOBAL_SCOPE,
    action: UiAction::AdjustSelectedBrowserRating { delta: 1 },
};
pub(crate) const TAG_NEUTRAL: HotkeyBinding = HotkeyBinding {
    id: "tag-neutral",
    label: "Neutral sample(s)",
    gesture: HotkeyGesture::new(KeyCode::Quote),
    scope: GLOBAL_SCOPE,
    action: UiAction::TagBrowserSelection {
        target: BrowserTagTarget::Neutral,
    },
};
pub(crate) const TAG_KEEP: HotkeyBinding = HotkeyBinding {
    id: "tag-keep",
    label: "Keep sample(s)",
    gesture: HotkeyGesture::new(KeyCode::Num5),
    scope: GLOBAL_SCOPE,
    action: UiAction::TagBrowserSelection {
        target: BrowserTagTarget::Keep,
    },
};
pub(crate) const TAG_TRASH: HotkeyBinding = HotkeyBinding {
    id: "tag-trash",
    label: "Trash sample(s)",
    gesture: HotkeyGesture::new(KeyCode::Num1),
    scope: GLOBAL_SCOPE,
    action: UiAction::TagBrowserSelection {
        target: BrowserTagTarget::Trash,
    },
};
