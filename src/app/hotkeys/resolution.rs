//! Hotkey gesture resolution helpers for the shared catalog.
//!
//! Keeping routing here isolates keypress/chord semantics from catalog assembly
//! so tests can lock down overloaded gesture behavior without depending on how
//! the bindings are declared.

use super::{FocusContextModel, HOTKEY_BINDINGS, KeyPress};

/// Result of resolving one keypress against the catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HotkeyResolution {
    /// Action produced by this keypress, if any.
    pub action: Option<super::UiAction>,
    /// Whether the keypress was consumed by the hotkey system.
    pub handled: bool,
    /// Pending chord starter to carry into the next keypress, if any.
    pub pending_chord: Option<KeyPress>,
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
    use crate::gui::input::KeyCode;
    use crate::sempal_app::{FocusContextModel, UiAction};

    use super::{HotkeyResolution, KeyPress, resolve_hotkey_press};

    const COPY_CASES: &[(FocusContextModel, Option<UiAction>)] = &[
        (
            FocusContextModel::SampleBrowser,
            Some(UiAction::CopySelectionToClipboard),
        ),
        (
            FocusContextModel::Waveform,
            Some(UiAction::CopySelectionToClipboard),
        ),
    ];

    const SEMICOLON_CASES: &[(FocusContextModel, Option<UiAction>, bool)] = &[
        (
            FocusContextModel::SampleBrowser,
            Some(UiAction::ToggleBrowserSampleMark),
            true,
        ),
        (FocusContextModel::Waveform, None, false),
    ];

    const PLAIN_KEY_CASES: &[(KeyPress, FocusContextModel, Option<UiAction>)] = &[
        (
            KeyPress::new(KeyCode::C),
            FocusContextModel::SampleBrowser,
            Some(UiAction::SetCompareAnchorFromFocusedBrowserSample),
        ),
        (
            KeyPress::new(KeyCode::C),
            FocusContextModel::Waveform,
            Some(UiAction::CropWaveformSelection),
        ),
        (
            KeyPress::new(KeyCode::C),
            FocusContextModel::SourceFolders,
            None,
        ),
        (
            KeyPress::new(KeyCode::D),
            FocusContextModel::SampleBrowser,
            Some(UiAction::DeleteBrowserSelection),
        ),
        (
            KeyPress::new(KeyCode::D),
            FocusContextModel::SourceFolders,
            Some(UiAction::DeleteFocusedFolder),
        ),
        (
            KeyPress::new(KeyCode::D),
            FocusContextModel::SourcesList,
            Some(UiAction::RemoveFocusedSourceRow),
        ),
        (
            KeyPress::new(KeyCode::D),
            FocusContextModel::Waveform,
            Some(UiAction::DeleteLoadedWaveformSample),
        ),
        (
            KeyPress::new(KeyCode::N),
            FocusContextModel::SampleBrowser,
            Some(UiAction::NormalizeFocusedBrowserSample),
        ),
        (
            KeyPress::new(KeyCode::N),
            FocusContextModel::SourceFolders,
            Some(UiAction::StartNewFolder),
        ),
        (
            KeyPress::new(KeyCode::N),
            FocusContextModel::Waveform,
            Some(UiAction::NormalizeWaveformSelectionOrSample),
        ),
        (
            KeyPress::new(KeyCode::N),
            FocusContextModel::SourcesList,
            None,
        ),
        (
            KeyPress::new(KeyCode::R),
            FocusContextModel::SampleBrowser,
            Some(UiAction::StartBrowserRename),
        ),
        (
            KeyPress::new(KeyCode::R),
            FocusContextModel::SourceFolders,
            Some(UiAction::StartFolderRename),
        ),
        (
            KeyPress::new(KeyCode::R),
            FocusContextModel::SourcesList,
            Some(UiAction::ReloadFocusedSourceRow),
        ),
        (KeyPress::new(KeyCode::R), FocusContextModel::Waveform, None),
    ];

    const ARROW_CASES: &[(KeyPress, FocusContextModel, Option<UiAction>)] = &[
        (
            KeyPress::new(KeyCode::ArrowUp),
            FocusContextModel::SampleBrowser,
            Some(UiAction::MoveBrowserFocus { delta: -1 }),
        ),
        (
            KeyPress::new(KeyCode::ArrowDown),
            FocusContextModel::SampleBrowser,
            Some(UiAction::MoveBrowserFocus { delta: 1 }),
        ),
        (
            KeyPress::new(KeyCode::ArrowUp),
            FocusContextModel::SourceFolders,
            Some(UiAction::MoveFolderFocus { delta: -1 }),
        ),
        (
            KeyPress::new(KeyCode::ArrowDown),
            FocusContextModel::SourceFolders,
            Some(UiAction::MoveFolderFocus { delta: 1 }),
        ),
        (
            KeyPress::new(KeyCode::ArrowUp),
            FocusContextModel::SourcesList,
            Some(UiAction::MoveSourceFocus { delta: -1 }),
        ),
        (
            KeyPress::new(KeyCode::ArrowDown),
            FocusContextModel::SourcesList,
            Some(UiAction::MoveSourceFocus { delta: 1 }),
        ),
        (
            KeyPress::new(KeyCode::ArrowLeft),
            FocusContextModel::SampleBrowser,
            Some(UiAction::FocusPreviousBrowserHistory),
        ),
        (
            KeyPress::new(KeyCode::ArrowRight),
            FocusContextModel::SampleBrowser,
            Some(UiAction::FocusNextBrowserHistory),
        ),
        (
            KeyPress::new(KeyCode::ArrowLeft),
            FocusContextModel::SourceFolders,
            Some(UiAction::CollapseFocusedFolder),
        ),
        (
            KeyPress::new(KeyCode::ArrowRight),
            FocusContextModel::SourceFolders,
            Some(UiAction::ExpandFocusedFolder),
        ),
        (
            KeyPress::new(KeyCode::ArrowLeft),
            FocusContextModel::Waveform,
            Some(UiAction::MoveWaveformSliceFocus { delta: -1 }),
        ),
        (
            KeyPress::new(KeyCode::ArrowRight),
            FocusContextModel::Waveform,
            Some(UiAction::MoveWaveformSliceFocus { delta: 1 }),
        ),
        (
            KeyPress::new(KeyCode::ArrowLeft),
            FocusContextModel::SourcesList,
            None,
        ),
    ];

    fn resolved_action(press: KeyPress, focus: FocusContextModel) -> Option<UiAction> {
        resolve_hotkey_press(None, press, focus).action
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

    #[test]
    fn invalid_second_chord_press_does_not_fall_through_to_plain_binding() {
        let focus = FocusContextModel::SampleBrowser;
        let first = resolve_hotkey_press(None, KeyPress::new(KeyCode::G), focus);
        let second = resolve_hotkey_press(first.pending_chord, KeyPress::new(KeyCode::C), focus);
        assert_eq!(
            second,
            HotkeyResolution {
                action: None,
                handled: true,
                pending_chord: None,
            }
        );
    }

    #[test]
    fn copy_hotkey_resolves_in_browser_and_waveform_scopes() {
        for (focus, expected) in COPY_CASES {
            let resolution = resolve_hotkey_press(None, KeyPress::with_command(KeyCode::C), *focus);
            assert_eq!(resolution.action, *expected, "focus: {focus:?}");
            assert!(resolution.handled, "focus: {focus:?}");
        }
    }

    #[test]
    fn semicolon_hotkey_routes_browser_mark_without_conflicting_with_waveform_shortcuts() {
        for (focus, expected, handled) in SEMICOLON_CASES {
            let resolution = resolve_hotkey_press(None, KeyPress::new(KeyCode::Semicolon), *focus);
            assert_eq!(resolution.action, *expected, "focus: {focus:?}");
            assert_eq!(resolution.handled, *handled, "focus: {focus:?}");
        }
    }

    #[test]
    fn overloaded_plain_key_families_route_by_focus_context() {
        for (press, focus, expected) in PLAIN_KEY_CASES {
            assert_eq!(
                resolved_action(*press, *focus),
                *expected,
                "focus: {focus:?}"
            );
        }
    }

    #[test]
    fn overloaded_arrow_keys_route_by_focus_context() {
        for (press, focus, expected) in ARROW_CASES {
            assert_eq!(
                resolved_action(*press, *focus),
                *expected,
                "focus: {focus:?}"
            );
        }
    }
}
