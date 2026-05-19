use super::KeyCode;

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
