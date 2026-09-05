//! Preserve native logical identity separately from positional key codes.
use crate::application::{CommandInput, CommandKey, CommandModifiers};
use winit::keyboard::{Key, ModifiersState, PhysicalKey};

pub(super) fn command_input(
    logical: &Key,
    physical: PhysicalKey,
    modifiers: ModifiersState,
    repeat: bool,
    composing: bool,
) -> CommandInput {
    CommandInput {
        logical: match logical {
            Key::Character(text) => Some(CommandKey::Character(text.to_string())),
            Key::Named(key) => Some(CommandKey::Named(format!("{key:?}"))),
            Key::Dead(_) | Key::Unidentified(_) => None,
        },
        // Winit code variant names follow UI Events positional code names. Do not
        // derive these from logical characters or from the legacy subset KeyCode.
        physical: match physical {
            PhysicalKey::Code(code) => Some(format!("{code:?}")),
            PhysicalKey::Unidentified(_) => None,
        },
        modifiers: CommandModifiers {
            primary: false,
            control: modifiers.control_key(),
            shift: modifiers.shift_key(),
            alt: modifiers.alt_key(),
            meta: modifiers.super_key(),
        },
        platform: super::super::input::native_shortcut_platform(),
        repeat,
        composing: composing || matches!(logical, Key::Dead(_)),
        text_consumed: false,
        platform_reserved: false,
    }
}

/// Required operations of the current single-line text adapter. Failed clipboard
/// access must not turn an editing key into an unrelated semantic command.
pub(super) fn required_text_key(
    key: Option<crate::gui::input::KeyCode>,
    text: Option<&str>,
    modifiers: ModifiersState,
) -> bool {
    use crate::gui::input::KeyCode;
    let primary = modifiers.control_key() || modifiers.super_key();
    matches!(
        key,
        Some(KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::Home | KeyCode::End)
    ) || (primary
        && matches!(
            key,
            Some(
                KeyCode::A
                    | KeyCode::C
                    | KeyCode::X
                    | KeyCode::V
                    | KeyCode::Backspace
                    | KeyCode::Delete
            )
        ))
        || (!primary
            && !modifiers.alt_key()
            && (text.is_some_and(|text| text.chars().any(|character| !character.is_control()))
                || matches!(
                    key,
                    Some(
                        KeyCode::Enter
                            | KeyCode::Tab
                            | KeyCode::Backspace
                            | KeyCode::Delete
                            | KeyCode::Space
                    )
                )))
}

/// Logical editing identity is sufficient for text operations when the platform
/// cannot report a physical code. It never supplies a positional command binding.
pub(super) fn logical_editing_key(key: &Key) -> Option<crate::gui::input::KeyCode> {
    use crate::gui::input::KeyCode;
    use winit::keyboard::NamedKey;
    Some(match key {
        Key::Character(text) if text.eq_ignore_ascii_case("a") => KeyCode::A,
        Key::Character(text) if text.eq_ignore_ascii_case("c") => KeyCode::C,
        Key::Character(text) if text.eq_ignore_ascii_case("x") => KeyCode::X,
        Key::Character(text) if text.eq_ignore_ascii_case("v") => KeyCode::V,
        Key::Named(NamedKey::Backspace) => KeyCode::Backspace,
        Key::Named(NamedKey::Delete) => KeyCode::Delete,
        Key::Named(NamedKey::Enter) => KeyCode::Enter,
        Key::Named(NamedKey::Tab) => KeyCode::Tab,
        Key::Named(NamedKey::Space) => KeyCode::Space,
        Key::Named(NamedKey::ArrowLeft) => KeyCode::ArrowLeft,
        Key::Named(NamedKey::ArrowRight) => KeyCode::ArrowRight,
        Key::Named(NamedKey::Home) => KeyCode::Home,
        Key::Named(NamedKey::End) => KeyCode::End,
        _ => return None,
    })
}

#[cfg(test)]
mod tests;
