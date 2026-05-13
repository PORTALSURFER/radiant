//! Input mapping for the generic native Vello runtime.

use crate::{
    gui::input::{KeyCode, KeyPress},
    widgets::PointerButton,
};
use winit::event::MouseButton;

/// Convert a `winit` physical key code into the local backend-agnostic key representation.
///
/// Returns `None` when a key is not currently used by app shortcuts or widget
/// controls.
pub(super) fn key_code_from_winit(key: winit::keyboard::KeyCode) -> Option<KeyCode> {
    use winit::keyboard::KeyCode as WinitKeyCode;
    Some(match key {
        WinitKeyCode::Digit0 => KeyCode::Num0,
        WinitKeyCode::Digit1 => KeyCode::Num1,
        WinitKeyCode::Digit2 => KeyCode::Num2,
        WinitKeyCode::Digit3 => KeyCode::Num3,
        WinitKeyCode::Digit4 => KeyCode::Num4,
        WinitKeyCode::Digit5 => KeyCode::Num5,
        WinitKeyCode::Digit6 => KeyCode::Num6,
        WinitKeyCode::Digit7 => KeyCode::Num7,
        WinitKeyCode::Digit8 => KeyCode::Num8,
        WinitKeyCode::Digit9 => KeyCode::Num9,
        WinitKeyCode::KeyA => KeyCode::A,
        WinitKeyCode::KeyB => KeyCode::B,
        WinitKeyCode::KeyC => KeyCode::C,
        WinitKeyCode::KeyD => KeyCode::D,
        WinitKeyCode::KeyE => KeyCode::E,
        WinitKeyCode::Enter | WinitKeyCode::NumpadEnter => KeyCode::Enter,
        WinitKeyCode::Delete => KeyCode::Delete,
        WinitKeyCode::KeyF => KeyCode::F,
        WinitKeyCode::F1 => KeyCode::F1,
        WinitKeyCode::F2 => KeyCode::F2,
        WinitKeyCode::KeyG => KeyCode::G,
        WinitKeyCode::KeyH => KeyCode::H,
        WinitKeyCode::KeyI => KeyCode::I,
        WinitKeyCode::KeyL => KeyCode::L,
        WinitKeyCode::KeyM => KeyCode::M,
        WinitKeyCode::KeyN => KeyCode::N,
        WinitKeyCode::BracketLeft => KeyCode::OpenBracket,
        WinitKeyCode::KeyO => KeyCode::O,
        WinitKeyCode::BracketRight => KeyCode::CloseBracket,
        WinitKeyCode::KeyP => KeyCode::P,
        WinitKeyCode::Quote => KeyCode::Quote,
        WinitKeyCode::KeyR => KeyCode::R,
        WinitKeyCode::KeyS => KeyCode::S,
        WinitKeyCode::Semicolon => KeyCode::Semicolon,
        WinitKeyCode::Slash => KeyCode::Slash,
        WinitKeyCode::Backslash => KeyCode::Backslash,
        WinitKeyCode::KeyT => KeyCode::T,
        WinitKeyCode::KeyU => KeyCode::U,
        WinitKeyCode::KeyV => KeyCode::V,
        WinitKeyCode::Space => KeyCode::Space,
        WinitKeyCode::KeyW => KeyCode::W,
        WinitKeyCode::KeyX => KeyCode::X,
        WinitKeyCode::KeyY => KeyCode::Y,
        WinitKeyCode::KeyZ => KeyCode::Z,
        WinitKeyCode::ArrowLeft => KeyCode::ArrowLeft,
        WinitKeyCode::ArrowRight => KeyCode::ArrowRight,
        WinitKeyCode::ArrowUp => KeyCode::ArrowUp,
        WinitKeyCode::ArrowDown => KeyCode::ArrowDown,
        WinitKeyCode::Home => KeyCode::Home,
        WinitKeyCode::End => KeyCode::End,
        _ => return None,
    })
}

pub(super) fn pointer_button_from_winit(button: MouseButton) -> Option<PointerButton> {
    Some(match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Auxiliary,
        _ => return None,
    })
}

pub(super) fn keypress_from_input(
    key: KeyCode,
    modifiers: winit::keyboard::ModifiersState,
) -> KeyPress {
    KeyPress {
        key,
        command: modifiers.control_key() || modifiers.super_key(),
        shift: modifiers.shift_key(),
        alt: modifiers.alt_key(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::KeyCode as WinitKeyCode;

    #[test]
    fn key_code_from_winit_maps_supported_codes() {
        assert_eq!(
            key_code_from_winit(WinitKeyCode::Digit1),
            Some(KeyCode::Num1)
        );
        assert_eq!(key_code_from_winit(WinitKeyCode::KeyA), Some(KeyCode::A));
        assert_eq!(key_code_from_winit(WinitKeyCode::KeyE), Some(KeyCode::E));
        assert_eq!(key_code_from_winit(WinitKeyCode::F2), Some(KeyCode::F2));
        assert_eq!(key_code_from_winit(WinitKeyCode::KeyV), Some(KeyCode::V));
        assert_eq!(
            key_code_from_winit(WinitKeyCode::Semicolon),
            Some(KeyCode::Semicolon)
        );
        assert_eq!(
            key_code_from_winit(WinitKeyCode::ArrowLeft),
            Some(KeyCode::ArrowLeft)
        );
        assert_eq!(key_code_from_winit(WinitKeyCode::Home), Some(KeyCode::Home));
        assert_eq!(
            key_code_from_winit(WinitKeyCode::NumpadEnter),
            Some(KeyCode::Enter)
        );
        assert_eq!(
            key_code_from_winit(WinitKeyCode::Delete),
            Some(KeyCode::Delete)
        );
        assert_eq!(
            key_code_from_winit(WinitKeyCode::Space),
            Some(KeyCode::Space)
        );
    }

    #[test]
    fn key_code_from_winit_returns_none_for_unsupported_code() {
        assert_eq!(key_code_from_winit(WinitKeyCode::Escape), None);
        assert_eq!(key_code_from_winit(WinitKeyCode::Tab), None);
    }
}
