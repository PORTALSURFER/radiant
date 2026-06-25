use super::*;

#[test]
fn keypress_constructors_preserve_modifier_state() {
    assert_eq!(
        KeyPress::new(KeyCode::G),
        KeyPress {
            key: KeyCode::G,
            command: false,
            control: false,
            shift: false,
            alt: false,
        }
    );
    assert_eq!(
        KeyPress::with_command(KeyCode::G),
        KeyPress {
            key: KeyCode::G,
            command: true,
            control: false,
            shift: false,
            alt: false,
        }
    );
    assert_eq!(
        KeyPress::with_control(KeyCode::G),
        KeyPress {
            key: KeyCode::G,
            command: false,
            control: true,
            shift: false,
            alt: false,
        }
    );
    assert_eq!(
        KeyPress::with_shift(KeyCode::G),
        KeyPress {
            key: KeyCode::G,
            command: false,
            control: false,
            shift: true,
            alt: false,
        }
    );
    assert_eq!(
        KeyPress::with_alt(KeyCode::G),
        KeyPress {
            key: KeyCode::G,
            command: false,
            control: false,
            shift: false,
            alt: true,
        }
    );
}
