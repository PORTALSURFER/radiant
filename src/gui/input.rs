//! Keyboard input primitives used by hotkeys and future GUI backends.

/// Backend-agnostic key code values used by sempal hotkeys.
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
    /// Enter/Return key.
    Enter,
    /// Latin letter F.
    F,
    /// F1 function key.
    F1,
    /// Latin letter G.
    G,
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
    /// Slash key (`/`).
    Slash,
    /// Backslash key (`\\`).
    Backslash,
    /// Latin letter T.
    T,
    /// Latin letter U.
    U,
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
        WinitKeyCode::Enter | WinitKeyCode::NumpadEnter => KeyCode::Enter,
        WinitKeyCode::KeyF => KeyCode::F,
        WinitKeyCode::F1 => KeyCode::F1,
        WinitKeyCode::KeyG => KeyCode::G,
        WinitKeyCode::KeyI => KeyCode::I,
        WinitKeyCode::KeyL => KeyCode::L,
        WinitKeyCode::KeyM => KeyCode::M,
        WinitKeyCode::KeyN => KeyCode::N,
        WinitKeyCode::BracketLeft => KeyCode::OpenBracket,
        WinitKeyCode::BracketRight => KeyCode::CloseBracket,
        WinitKeyCode::KeyP => KeyCode::P,
        WinitKeyCode::Quote => KeyCode::Quote,
        WinitKeyCode::KeyR => KeyCode::R,
        WinitKeyCode::KeyS => KeyCode::S,
        WinitKeyCode::Slash => KeyCode::Slash,
        WinitKeyCode::Backslash => KeyCode::Backslash,
        WinitKeyCode::KeyT => KeyCode::T,
        WinitKeyCode::KeyU => KeyCode::U,
        WinitKeyCode::KeyW => KeyCode::W,
        WinitKeyCode::KeyX => KeyCode::X,
        WinitKeyCode::KeyY => KeyCode::Y,
        WinitKeyCode::KeyZ => KeyCode::Z,
        WinitKeyCode::ArrowLeft => KeyCode::ArrowLeft,
        WinitKeyCode::ArrowRight => KeyCode::ArrowRight,
        WinitKeyCode::ArrowUp => KeyCode::ArrowUp,
        WinitKeyCode::ArrowDown => KeyCode::ArrowDown,
        _ => return None,
    })
}
