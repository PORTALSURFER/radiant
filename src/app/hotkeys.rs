//! Shared hotkey catalog for native runtime routing and host presentation.
//!
//! The native runtime and host-side help/automation surfaces must agree on the
//! exact hotkey contract. This module keeps the bindings, gestures, scopes, and
//! `UiAction` payloads in one place so keyboard routing, overlays, and tests all
//! read from the same source of truth.

use super::{BrowserTagTarget, FocusContextModel, UiAction};
use crate::gui::input::KeyCode;

/// Logical section scope that owns a hotkey binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyScope {
    /// Binding is always active regardless of section focus.
    Global,
    /// Binding is active only when the matching section owns focus.
    Focus(FocusContextModel),
}

impl HotkeyScope {
    /// Return whether this scope is active for the provided focus context.
    pub fn matches(self, focus: FocusContextModel) -> bool {
        match self {
            Self::Global => true,
            Self::Focus(target) => target == focus,
        }
    }
}

/// One physical keypress plus modifier state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyPress {
    /// Physical key identity.
    pub key: KeyCode,
    /// Whether the platform command modifier is held.
    pub command: bool,
    /// Whether Shift is held.
    pub shift: bool,
    /// Whether Alt is held.
    pub alt: bool,
}

impl KeyPress {
    /// Build an unmodified keypress.
    pub const fn new(key: KeyCode) -> Self {
        Self {
            key,
            command: false,
            shift: false,
            alt: false,
        }
    }

    /// Build a command-modified keypress.
    pub const fn with_command(key: KeyCode) -> Self {
        Self {
            key,
            command: true,
            shift: false,
            alt: false,
        }
    }

    /// Build a shift-modified keypress.
    pub const fn with_shift(key: KeyCode) -> Self {
        Self {
            key,
            command: false,
            shift: true,
            alt: false,
        }
    }

    /// Build an alt-modified keypress.
    pub const fn with_alt(key: KeyCode) -> Self {
        Self {
            key,
            command: false,
            shift: false,
            alt: true,
        }
    }
}

/// Keyboard gesture used to trigger one binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HotkeyGesture {
    /// First keypress in the gesture.
    pub first: KeyPress,
    /// Optional chord follow-up keypress.
    pub chord: Option<KeyPress>,
}

impl HotkeyGesture {
    /// Build a single-key gesture.
    pub const fn new(key: KeyCode) -> Self {
        Self {
            first: KeyPress::new(key),
            chord: None,
        }
    }

    /// Build a single-key command gesture.
    pub const fn with_command(key: KeyCode) -> Self {
        Self {
            first: KeyPress::with_command(key),
            chord: None,
        }
    }

    /// Build a single-key shift gesture.
    pub const fn with_shift(key: KeyCode) -> Self {
        Self {
            first: KeyPress::with_shift(key),
            chord: None,
        }
    }

    /// Build a single-key alt gesture.
    pub const fn with_alt(key: KeyCode) -> Self {
        Self {
            first: KeyPress::with_alt(key),
            chord: None,
        }
    }

    /// Build a two-step chord gesture.
    pub const fn with_chord(first: KeyPress, second: KeyPress) -> Self {
        Self {
            first,
            chord: Some(second),
        }
    }
}

/// One cataloged hotkey binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotkeyBinding {
    /// Stable binding identifier for tests and overlays.
    pub id: &'static str,
    /// Human-readable label shown in help surfaces.
    pub label: &'static str,
    /// Keyboard gesture that triggers the binding.
    pub gesture: HotkeyGesture,
    /// Section scope that owns the binding.
    pub scope: HotkeyScope,
    /// Action emitted when the gesture resolves.
    pub action: UiAction,
}

impl HotkeyBinding {
    /// Return whether the binding is active for the provided focus context.
    pub fn is_active(&self, focus: FocusContextModel) -> bool {
        self.scope.matches(focus)
    }
}

/// Result of resolving one keypress against the catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HotkeyResolution {
    /// Action produced by this keypress, if any.
    pub action: Option<UiAction>,
    /// Whether the keypress was consumed by the hotkey system.
    pub handled: bool,
    /// Pending chord starter to carry into the next keypress, if any.
    pub pending_chord: Option<KeyPress>,
}

const GLOBAL: HotkeyScope = HotkeyScope::Global;
const BROWSER: HotkeyScope = HotkeyScope::Focus(FocusContextModel::SampleBrowser);
const WAVEFORM: HotkeyScope = HotkeyScope::Focus(FocusContextModel::Waveform);
const FOLDERS: HotkeyScope = HotkeyScope::Focus(FocusContextModel::SourceFolders);
const SOURCES: HotkeyScope = HotkeyScope::Focus(FocusContextModel::SourcesList);

/// Shared hotkey bindings in stable presentation order.
pub const HOTKEY_BINDINGS: &[HotkeyBinding] = &[
    HotkeyBinding {
        id: "undo-ctrl-z",
        label: "Undo",
        gesture: HotkeyGesture::with_command(KeyCode::Z),
        scope: GLOBAL,
        action: UiAction::Undo,
    },
    HotkeyBinding {
        id: "undo-u",
        label: "Undo",
        gesture: HotkeyGesture::new(KeyCode::U),
        scope: GLOBAL,
        action: UiAction::Undo,
    },
    HotkeyBinding {
        id: "redo-ctrl-y",
        label: "Redo",
        gesture: HotkeyGesture::with_command(KeyCode::Y),
        scope: GLOBAL,
        action: UiAction::Redo,
    },
    HotkeyBinding {
        id: "redo-shift-u",
        label: "Redo",
        gesture: HotkeyGesture::with_shift(KeyCode::U),
        scope: GLOBAL,
        action: UiAction::Redo,
    },
    HotkeyBinding {
        id: "show-hotkeys",
        label: "Show hotkeys",
        gesture: HotkeyGesture::with_command(KeyCode::Slash),
        scope: GLOBAL,
        action: UiAction::ToggleHotkeyOverlay,
    },
    HotkeyBinding {
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
        scope: GLOBAL,
        action: UiAction::CopyStatusLog,
    },
    HotkeyBinding {
        id: "submit-github-issue",
        label: "Submit GitHub issue",
        gesture: HotkeyGesture::with_shift(KeyCode::F1),
        scope: GLOBAL,
        action: UiAction::OpenFeedbackIssuePrompt,
    },
    HotkeyBinding {
        id: "focus-waveform",
        label: "Focus waveform",
        gesture: HotkeyGesture::with_chord(KeyPress::new(KeyCode::G), KeyPress::new(KeyCode::W)),
        scope: GLOBAL,
        action: UiAction::FocusWaveformPanel,
    },
    HotkeyBinding {
        id: "focus-browser",
        label: "Focus source samples",
        gesture: HotkeyGesture::with_chord(KeyPress::new(KeyCode::G), KeyPress::new(KeyCode::B)),
        scope: GLOBAL,
        action: UiAction::FocusBrowserPanel,
    },
    HotkeyBinding {
        id: "focus-folder-tree",
        label: "Focus folder tree",
        gesture: HotkeyGesture::with_chord(KeyPress::new(KeyCode::G), KeyPress::new(KeyCode::T)),
        scope: GLOBAL,
        action: UiAction::FocusFolderPanel,
    },
    HotkeyBinding {
        id: "focus-sources-list",
        label: "Focus sources list",
        gesture: HotkeyGesture::with_chord(KeyPress::new(KeyCode::G), KeyPress::new(KeyCode::S)),
        scope: GLOBAL,
        action: UiAction::FocusSourcesPanel,
    },
    HotkeyBinding {
        id: "play-from-start",
        label: "Play from start",
        gesture: HotkeyGesture::new(KeyCode::Space),
        scope: GLOBAL,
        action: UiAction::PlayFromStart,
    },
    HotkeyBinding {
        id: "play-from-current-playhead",
        label: "Play from current playhead",
        gesture: HotkeyGesture::with_command(KeyCode::Space),
        scope: GLOBAL,
        action: UiAction::PlayFromCurrentPlayhead,
    },
    HotkeyBinding {
        id: "toggle-loop",
        label: "Toggle loop",
        gesture: HotkeyGesture::new(KeyCode::L),
        scope: GLOBAL,
        action: UiAction::ToggleLoopPlayback,
    },
    HotkeyBinding {
        id: "toggle-loop-lock",
        label: "Toggle loop lock",
        gesture: HotkeyGesture::with_shift(KeyCode::L),
        scope: GLOBAL,
        action: UiAction::ToggleLoopLock,
    },
    HotkeyBinding {
        id: "rate-decrement",
        label: "Decrement rating",
        gesture: HotkeyGesture::new(KeyCode::OpenBracket),
        scope: GLOBAL,
        action: UiAction::AdjustSelectedBrowserRating { delta: -1 },
    },
    HotkeyBinding {
        id: "rate-increment",
        label: "Increment rating",
        gesture: HotkeyGesture::new(KeyCode::CloseBracket),
        scope: GLOBAL,
        action: UiAction::AdjustSelectedBrowserRating { delta: 1 },
    },
    HotkeyBinding {
        id: "tag-neutral",
        label: "Neutral sample(s)",
        gesture: HotkeyGesture::new(KeyCode::Quote),
        scope: GLOBAL,
        action: UiAction::TagBrowserSelection {
            target: BrowserTagTarget::Neutral,
        },
    },
    HotkeyBinding {
        id: "tag-keep",
        label: "Keep sample(s)",
        gesture: HotkeyGesture::new(KeyCode::Num5),
        scope: GLOBAL,
        action: UiAction::TagBrowserSelection {
            target: BrowserTagTarget::Keep,
        },
    },
    HotkeyBinding {
        id: "tag-trash",
        label: "Trash sample(s)",
        gesture: HotkeyGesture::new(KeyCode::Num1),
        scope: GLOBAL,
        action: UiAction::TagBrowserSelection {
            target: BrowserTagTarget::Trash,
        },
    },
    HotkeyBinding {
        id: "search-browser",
        label: "Search samples",
        gesture: HotkeyGesture::with_command(KeyCode::F),
        scope: BROWSER,
        action: UiAction::FocusBrowserSearch,
    },
    HotkeyBinding {
        id: "focus-loaded-sample",
        label: "Focus loaded sample",
        gesture: HotkeyGesture::new(KeyCode::F),
        scope: BROWSER,
        action: UiAction::FocusLoadedSampleInBrowser,
    },
    HotkeyBinding {
        id: "find-similar",
        label: "Toggle find similar",
        gesture: HotkeyGesture::with_shift(KeyCode::F),
        scope: BROWSER,
        action: UiAction::ToggleFindSimilarFocusedSample,
    },
    HotkeyBinding {
        id: "toggle-random-navigation-mode",
        label: "Toggle random navigation mode",
        gesture: HotkeyGesture::with_alt(KeyCode::R),
        scope: BROWSER,
        action: UiAction::ToggleRandomNavigationMode,
    },
    HotkeyBinding {
        id: "play-random-sample",
        label: "Play random sample",
        gesture: HotkeyGesture::with_shift(KeyCode::R),
        scope: BROWSER,
        action: UiAction::PlayRandomSample,
    },
    HotkeyBinding {
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
        scope: BROWSER,
        action: UiAction::PlayPreviousRandomSample,
    },
    HotkeyBinding {
        id: "move-trashed-to-folder",
        label: "Move trashed samples to folder",
        gesture: HotkeyGesture::new(KeyCode::P),
        scope: BROWSER,
        action: UiAction::MoveTrashedSamplesToFolder,
    },
    HotkeyBinding {
        id: "move-trashed-to-folder-shift",
        label: "Move trashed samples to folder",
        gesture: HotkeyGesture::with_shift(KeyCode::P),
        scope: BROWSER,
        action: UiAction::MoveTrashedSamplesToFolder,
    },
    HotkeyBinding {
        id: "toggle-select",
        label: "Toggle selection",
        gesture: HotkeyGesture::new(KeyCode::X),
        scope: BROWSER,
        action: UiAction::ToggleFocusedBrowserRowSelection,
    },
    HotkeyBinding {
        id: "focus-history-previous",
        label: "Previous focused sample",
        gesture: HotkeyGesture::new(KeyCode::ArrowLeft),
        scope: BROWSER,
        action: UiAction::FocusPreviousBrowserHistory,
    },
    HotkeyBinding {
        id: "focus-history-next",
        label: "Next focused sample",
        gesture: HotkeyGesture::new(KeyCode::ArrowRight),
        scope: BROWSER,
        action: UiAction::FocusNextBrowserHistory,
    },
    HotkeyBinding {
        id: "rename-sample",
        label: "Rename sample",
        gesture: HotkeyGesture::new(KeyCode::R),
        scope: BROWSER,
        action: UiAction::StartBrowserRename,
    },
    HotkeyBinding {
        id: "select-all-browser",
        label: "Select all samples",
        gesture: HotkeyGesture::with_command(KeyCode::A),
        scope: BROWSER,
        action: UiAction::SelectAllBrowserRows,
    },
    HotkeyBinding {
        id: "normalize-browser",
        label: "Normalize sample",
        gesture: HotkeyGesture::new(KeyCode::N),
        scope: BROWSER,
        action: UiAction::NormalizeFocusedBrowserSample,
    },
    HotkeyBinding {
        id: "delete-browser",
        label: "Delete sample",
        gesture: HotkeyGesture::new(KeyCode::D),
        scope: BROWSER,
        action: UiAction::DeleteBrowserSelection,
    },
    HotkeyBinding {
        id: "toggle-folder-select",
        label: "Toggle folder selection",
        gesture: HotkeyGesture::new(KeyCode::X),
        scope: FOLDERS,
        action: UiAction::ToggleFocusedFolderSelection,
    },
    HotkeyBinding {
        id: "delete-folder",
        label: "Delete folder",
        gesture: HotkeyGesture::new(KeyCode::D),
        scope: FOLDERS,
        action: UiAction::DeleteFocusedFolder,
    },
    HotkeyBinding {
        id: "rename-folder",
        label: "Rename folder",
        gesture: HotkeyGesture::new(KeyCode::R),
        scope: FOLDERS,
        action: UiAction::StartFolderRename,
    },
    HotkeyBinding {
        id: "new-folder",
        label: "New folder",
        gesture: HotkeyGesture::new(KeyCode::N),
        scope: FOLDERS,
        action: UiAction::StartNewFolder,
    },
    HotkeyBinding {
        id: "search-folders",
        label: "Search folders",
        gesture: HotkeyGesture::with_command(KeyCode::F),
        scope: FOLDERS,
        action: UiAction::FocusFolderSearch,
    },
    HotkeyBinding {
        id: "move-source-focus-up",
        label: "Previous source",
        gesture: HotkeyGesture::new(KeyCode::ArrowUp),
        scope: SOURCES,
        action: UiAction::MoveSourceFocus { delta: -1 },
    },
    HotkeyBinding {
        id: "move-source-focus-down",
        label: "Next source",
        gesture: HotkeyGesture::new(KeyCode::ArrowDown),
        scope: SOURCES,
        action: UiAction::MoveSourceFocus { delta: 1 },
    },
    HotkeyBinding {
        id: "reload-focused-source",
        label: "Reload source",
        gesture: HotkeyGesture::new(KeyCode::R),
        scope: SOURCES,
        action: UiAction::ReloadFocusedSourceRow,
    },
    HotkeyBinding {
        id: "hard-sync-focused-source",
        label: "Hard sync source",
        gesture: HotkeyGesture::new(KeyCode::H),
        scope: SOURCES,
        action: UiAction::HardSyncFocusedSourceRow,
    },
    HotkeyBinding {
        id: "open-focused-source-folder",
        label: "Open source folder",
        gesture: HotkeyGesture::new(KeyCode::O),
        scope: SOURCES,
        action: UiAction::OpenFocusedSourceFolder,
    },
    HotkeyBinding {
        id: "remove-focused-source",
        label: "Remove source",
        gesture: HotkeyGesture::new(KeyCode::D),
        scope: SOURCES,
        action: UiAction::RemoveFocusedSourceRow,
    },
    HotkeyBinding {
        id: "remove-dead-links-focused-source",
        label: "Remove dead links",
        gesture: HotkeyGesture::with_shift(KeyCode::D),
        scope: SOURCES,
        action: UiAction::RemoveDeadLinksForFocusedSourceRow,
    },
    HotkeyBinding {
        id: "normalize-waveform",
        label: "Normalize selection/sample",
        gesture: HotkeyGesture::new(KeyCode::N),
        scope: WAVEFORM,
        action: UiAction::NormalizeWaveformSelectionOrSample,
    },
    HotkeyBinding {
        id: "align-waveform-start",
        label: "Set start to hover cursor",
        gesture: HotkeyGesture::new(KeyCode::S),
        scope: WAVEFORM,
        action: UiAction::AlignWaveformStartToMarker,
    },
    HotkeyBinding {
        id: "crop-selection",
        label: "Crop selection",
        gesture: HotkeyGesture::new(KeyCode::C),
        scope: WAVEFORM,
        action: UiAction::CropWaveformSelection,
    },
    HotkeyBinding {
        id: "crop-selection-new-sample",
        label: "Crop selection as new sample",
        gesture: HotkeyGesture::with_shift(KeyCode::C),
        scope: WAVEFORM,
        action: UiAction::CropWaveformSelectionToNewSample,
    },
    HotkeyBinding {
        id: "save-selection-to-browser",
        label: "Save selection/slices to browser",
        gesture: HotkeyGesture::new(KeyCode::Enter),
        scope: WAVEFORM,
        action: UiAction::SaveWaveformSelectionToBrowser,
    },
    HotkeyBinding {
        id: "trim-selection",
        label: "Trim selection",
        gesture: HotkeyGesture::new(KeyCode::T),
        scope: WAVEFORM,
        action: UiAction::TrimWaveformSelection,
    },
    HotkeyBinding {
        id: "toggle-bpm-snap",
        label: "Toggle BPM snap",
        gesture: HotkeyGesture::new(KeyCode::B),
        scope: WAVEFORM,
        action: UiAction::ToggleBpmSnap,
    },
    HotkeyBinding {
        id: "toggle-transients",
        label: "Show/hide transients",
        gesture: HotkeyGesture::new(KeyCode::I),
        scope: WAVEFORM,
        action: UiAction::ToggleTransientMarkers,
    },
    HotkeyBinding {
        id: "reverse-selection",
        label: "Reverse selection",
        gesture: HotkeyGesture::with_shift(KeyCode::R),
        scope: WAVEFORM,
        action: UiAction::ReverseWaveformSelection,
    },
    HotkeyBinding {
        id: "fade-selection-left-to-right",
        label: "Fade selection (left to right)",
        gesture: HotkeyGesture::new(KeyCode::Backslash),
        scope: WAVEFORM,
        action: UiAction::FadeWaveformSelectionLeftToRight,
    },
    HotkeyBinding {
        id: "fade-selection-right-to-left",
        label: "Fade selection (right to left)",
        gesture: HotkeyGesture::new(KeyCode::Slash),
        scope: WAVEFORM,
        action: UiAction::FadeWaveformSelectionRightToLeft,
    },
    HotkeyBinding {
        id: "delete-slice-markers",
        label: "Delete slice markers (Slice mode)",
        gesture: HotkeyGesture::with_shift(KeyCode::D),
        scope: WAVEFORM,
        action: UiAction::DeleteSelectedSliceMarkers,
    },
    HotkeyBinding {
        id: "delete-loaded-sample",
        label: "Delete loaded sample",
        gesture: HotkeyGesture::new(KeyCode::D),
        scope: WAVEFORM,
        action: UiAction::DeleteLoadedWaveformSample,
    },
    HotkeyBinding {
        id: "mute-selection",
        label: "Mute selection / Merge slices (Slice mode)",
        gesture: HotkeyGesture::new(KeyCode::M),
        scope: WAVEFORM,
        action: UiAction::MuteWaveformSelection,
    },
    HotkeyBinding {
        id: "zoom-in-selection",
        label: "Zoom to selection",
        gesture: HotkeyGesture::new(KeyCode::Z),
        scope: WAVEFORM,
        action: UiAction::ZoomWaveformToSelection,
    },
    HotkeyBinding {
        id: "zoom-out-selection",
        label: "Zoom out",
        gesture: HotkeyGesture::new(KeyCode::X),
        scope: WAVEFORM,
        action: UiAction::ZoomWaveformFull,
    },
    HotkeyBinding {
        id: "slide-selection-left",
        label: "Slide selection left",
        gesture: HotkeyGesture::new(KeyCode::ArrowLeft),
        scope: WAVEFORM,
        action: UiAction::SlideWaveformSelection {
            delta: -1,
            fine: false,
        },
    },
    HotkeyBinding {
        id: "slide-selection-right",
        label: "Slide selection right",
        gesture: HotkeyGesture::new(KeyCode::ArrowRight),
        scope: WAVEFORM,
        action: UiAction::SlideWaveformSelection {
            delta: 1,
            fine: false,
        },
    },
    HotkeyBinding {
        id: "nudge-selection-left",
        label: "Nudge selection left (fine)",
        gesture: HotkeyGesture::with_shift(KeyCode::ArrowLeft),
        scope: WAVEFORM,
        action: UiAction::SlideWaveformSelection {
            delta: -1,
            fine: true,
        },
    },
    HotkeyBinding {
        id: "nudge-selection-right",
        label: "Nudge selection right (fine)",
        gesture: HotkeyGesture::with_shift(KeyCode::ArrowRight),
        scope: WAVEFORM,
        action: UiAction::SlideWaveformSelection {
            delta: 1,
            fine: true,
        },
    },
];

/// Iterate over every shared hotkey binding.
pub fn iter_hotkey_bindings() -> impl Iterator<Item = &'static HotkeyBinding> {
    HOTKEY_BINDINGS.iter()
}

/// Resolve one keypress against the shared hotkey catalog.
pub(crate) fn resolve_hotkey_press(
    pending_chord: Option<KeyPress>,
    press: KeyPress,
    focus: FocusContextModel,
) -> HotkeyResolution {
    if let Some(first) = pending_chord {
        if let Some(binding) = HOTKEY_BINDINGS.iter().find(|binding| {
            binding.is_active(focus)
                && binding.gesture.first == first
                && binding.gesture.chord == Some(press)
        }) {
            return HotkeyResolution {
                action: Some(binding.action.clone()),
                handled: true,
                pending_chord: None,
            };
        }
        if HOTKEY_BINDINGS
            .iter()
            .any(|binding| binding.gesture.first == press && binding.gesture.chord.is_some())
        {
            return HotkeyResolution {
                action: None,
                handled: true,
                pending_chord: Some(press),
            };
        }
        return HotkeyResolution {
            action: None,
            handled: true,
            pending_chord: None,
        };
    }
    if let Some(binding) = HOTKEY_BINDINGS
        .iter()
        .find(|binding| binding.is_active(focus) && binding.gesture.first == press)
    {
        if binding.gesture.chord.is_none() {
            return HotkeyResolution {
                action: Some(binding.action.clone()),
                handled: true,
                pending_chord: None,
            };
        }
    }
    if HOTKEY_BINDINGS.iter().any(|binding| {
        binding.is_active(focus)
            && binding.gesture.first == press
            && binding.gesture.chord.is_some()
    }) {
        return HotkeyResolution {
            action: None,
            handled: true,
            pending_chord: Some(press),
        };
    }
    HotkeyResolution {
        action: None,
        handled: false,
        pending_chord: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkey_ids_are_unique() {
        for (index, binding) in HOTKEY_BINDINGS.iter().enumerate() {
            let duplicate = HOTKEY_BINDINGS
                .iter()
                .skip(index + 1)
                .find(|candidate| candidate.id == binding.id);
            assert!(duplicate.is_none(), "duplicate hotkey id: {}", binding.id);
        }
    }

    #[test]
    fn hotkey_gestures_are_unique_within_scope() {
        for (index, binding) in HOTKEY_BINDINGS.iter().enumerate() {
            let duplicate = HOTKEY_BINDINGS.iter().skip(index + 1).find(|candidate| {
                candidate.scope == binding.scope && candidate.gesture == binding.gesture
            });
            assert!(
                duplicate.is_none(),
                "duplicate scoped hotkey gesture for {:?}: {:?}",
                binding.scope,
                binding.gesture
            );
        }
    }

    #[test]
    fn source_list_actions_are_scoped_to_sources_focus() {
        let source_actions: Vec<_> = iter_hotkey_bindings()
            .filter(|binding| binding.scope == SOURCES)
            .collect();
        assert!(!source_actions.is_empty());
        assert!(
            source_actions
                .iter()
                .any(|binding| matches!(binding.action, UiAction::ReloadFocusedSourceRow))
        );
        assert!(
            source_actions
                .iter()
                .any(|binding| matches!(binding.action, UiAction::MoveSourceFocus { delta: -1 }))
        );
    }

    #[test]
    fn g_chord_focus_binding_resolves() {
        let focus = FocusContextModel::SampleBrowser;
        let first = resolve_hotkey_press(None, KeyPress::new(KeyCode::G), focus);
        assert_eq!(first.pending_chord, Some(KeyPress::new(KeyCode::G)));
        assert!(first.action.is_none());

        let second = resolve_hotkey_press(first.pending_chord, KeyPress::new(KeyCode::W), focus);
        assert_eq!(second.action, Some(UiAction::FocusWaveformPanel));
        assert!(second.handled);
        assert!(second.pending_chord.is_none());
    }
}
