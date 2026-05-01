//! Keyboard input primitives used by hotkeys and future GUI backends.

/// Backend-agnostic key code values used by host hotkeys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// Number row 0.
    Num0,
    /// Number row 1.
    Num1,
    /// Number row 2.
    Num2,
    /// Number row 3.
    Num3,
    /// Number row 4.
    Num4,
    /// Number row 5.
    Num5,
    /// Number row 6.
    Num6,
    /// Number row 7.
    Num7,
    /// Number row 8.
    Num8,
    /// Number row 9.
    Num9,
    /// Latin letter A.
    A,
    /// Latin letter B.
    B,
    /// Latin letter C.
    C,
    /// Latin letter D.
    D,
    /// Latin letter E.
    E,
    /// Enter/Return key.
    Enter,
    /// Latin letter F.
    F,
    /// F1 function key.
    F1,
    /// Latin letter G.
    G,
    /// Latin letter H.
    H,
    /// Latin letter I.
    I,
    /// Latin letter L.
    L,
    /// Latin letter M.
    M,
    /// Latin letter N.
    N,
    /// Open bracket (`[`).
    OpenBracket,
    /// Latin letter O.
    O,
    /// Close bracket (`]`).
    CloseBracket,
    /// Latin letter P.
    P,
    /// Quote key (`'`).
    Quote,
    /// Latin letter R.
    R,
    /// Latin letter S.
    S,
    /// Semicolon key (`;`).
    Semicolon,
    /// Slash key (`/`).
    Slash,
    /// Backslash key (`\\`).
    Backslash,
    /// Latin letter T.
    T,
    /// Latin letter U.
    U,
    /// Latin letter V.
    V,
    /// Space key.
    Space,
    /// Latin letter W.
    W,
    /// Latin letter X.
    X,
    /// Latin letter Y.
    Y,
    /// Latin letter Z.
    Z,
    /// Left arrow key.
    ArrowLeft,
    /// Right arrow key.
    ArrowRight,
    /// Up arrow key.
    ArrowUp,
    /// Down arrow key.
    ArrowDown,
    /// Home key.
    Home,
    /// End key.
    End,
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

/// Convert a `winit` physical key code into the local backend-agnostic key representation.
///
/// Returns `None` when a key is not currently used by app shortcuts or shell
/// controls.
pub fn key_code_from_winit(key: winit::keyboard::KeyCode) -> Option<KeyCode> {
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
        WinitKeyCode::KeyF => KeyCode::F,
        WinitKeyCode::F1 => KeyCode::F1,
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
            key_code_from_winit(WinitKeyCode::Space),
            Some(KeyCode::Space)
        );
    }

    #[test]
    fn key_code_from_winit_returns_none_for_unsupported_code() {
        assert_eq!(key_code_from_winit(WinitKeyCode::Escape), None);
        assert_eq!(key_code_from_winit(WinitKeyCode::Tab), None);
    }

    #[test]
    fn keypress_constructors_preserve_modifier_state() {
        assert_eq!(
            KeyPress::new(KeyCode::G),
            KeyPress {
                key: KeyCode::G,
                command: false,
                shift: false,
                alt: false,
            }
        );
        assert_eq!(
            KeyPress::with_command(KeyCode::G),
            KeyPress {
                key: KeyCode::G,
                command: true,
                shift: false,
                alt: false,
            }
        );
        assert_eq!(
            KeyPress::with_shift(KeyCode::G),
            KeyPress {
                key: KeyCode::G,
                command: false,
                shift: true,
                alt: false,
            }
        );
        assert_eq!(
            KeyPress::with_alt(KeyCode::G),
            KeyPress {
                key: KeyCode::G,
                command: false,
                shift: false,
                alt: true,
            }
        );
    }
}
