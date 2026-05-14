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
    /// Escape key.
    Escape,
    /// Delete key.
    Delete,
    /// Latin letter F.
    F,
    /// F1 function key.
    F1,
    /// F2 function key.
    F2,
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

#[cfg(test)]
mod tests {
    use super::*;

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
