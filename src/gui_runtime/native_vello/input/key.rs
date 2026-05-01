use super::*;
use crate::compat_app_contract::{HotkeyResolution, KeyPress};

pub(super) fn keypress_from_input(key: KeyCode, modifiers: ModifiersState) -> KeyPress {
    KeyPress {
        key,
        command: modifiers.control_key() || modifiers.super_key(),
        shift: modifiers.shift_key(),
        alt: modifiers.alt_key(),
    }
}

pub(super) fn action_from_key(
    key: KeyCode,
    modifiers: ModifiersState,
    model: &AppModel,
    pending_chord: Option<KeyPress>,
    mut resolve_hotkey: impl FnMut(
        Option<KeyPress>,
        KeyPress,
        crate::compat_app_contract::FocusContextModel,
    ) -> HotkeyResolution,
) -> HotkeyResolution {
    if model.confirm_prompt.visible {
        let confirm_enabled = model
            .confirm_prompt
            .input_error
            .as_ref()
            .is_none_or(|error| error.trim().is_empty());
        return match key {
            KeyCode::Enter if confirm_enabled => HotkeyResolution {
                action: Some(UiAction::ConfirmPrompt),
                handled: true,
                pending_chord: None,
            },
            KeyCode::C => HotkeyResolution {
                action: Some(UiAction::CancelPrompt),
                handled: true,
                pending_chord: None,
            },
            _ => HotkeyResolution {
                action: None,
                handled: false,
                pending_chord: None,
            },
        };
    }
    if model.options_panel.visible {
        return HotkeyResolution {
            action: None,
            handled: false,
            pending_chord: None,
        };
    }
    if matches!(key, KeyCode::P) && model.progress_overlay.cancelable {
        return HotkeyResolution {
            action: Some(UiAction::CancelProgress),
            handled: true,
            pending_chord: None,
        };
    }
    resolve_hotkey(
        pending_chord,
        keypress_from_input(key, modifiers),
        model.focus_context,
    )
}
