//! Input mapping for the generic native Vello runtime.

mod key_code;

use crate::{
    gui::input::{KeyCode, KeyPress},
    layout::Point,
    theme::DpiScale,
    widgets::{PointerButton, PointerModifiers},
};
use winit::dpi::PhysicalPosition;
use winit::event::MouseButton;

pub(super) use key_code::key_code_from_winit;

pub(super) fn logical_point_from_winit(
    position: PhysicalPosition<f64>,
    dpi_scale: DpiScale,
) -> Option<Point> {
    let point = Point::new(
        dpi_scale.physical_to_logical(position.x as f32),
        dpi_scale.physical_to_logical(position.y as f32),
    );
    point.is_finite().then_some(point)
}

pub(super) fn pointer_button_from_winit(button: MouseButton) -> Option<PointerButton> {
    Some(match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Auxiliary,
        _ => return None,
    })
}

pub(super) fn pointer_modifiers_from_winit(
    modifiers: winit::keyboard::ModifiersState,
) -> PointerModifiers {
    PointerModifiers {
        command: modifiers.control_key() || modifiers.super_key(),
        shift: modifiers.shift_key(),
        alt: modifiers.alt_key(),
    }
}

pub(super) fn keypress_from_input(
    key: KeyCode,
    modifiers: winit::keyboard::ModifiersState,
) -> KeyPress {
    KeyPress {
        key,
        command: command_modifier_from_winit(modifiers),
        control: control_modifier_from_winit(modifiers),
        shift: modifiers.shift_key(),
        alt: modifiers.alt_key(),
    }
}

fn command_modifier_from_winit(modifiers: winit::keyboard::ModifiersState) -> bool {
    if cfg!(target_os = "macos") {
        modifiers.super_key()
    } else {
        modifiers.control_key() || modifiers.super_key()
    }
}

fn control_modifier_from_winit(modifiers: winit::keyboard::ModifiersState) -> bool {
    cfg!(target_os = "macos") && modifiers.control_key()
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::ModifiersState;

    #[test]
    fn logical_point_from_winit_rejects_nonfinite_or_overflowing_coordinates() {
        assert_eq!(
            logical_point_from_winit(PhysicalPosition::new(25.0, 40.5), DpiScale::new(2.0)),
            Some(Point::new(12.5, 20.25))
        );
        assert_eq!(
            logical_point_from_winit(PhysicalPosition::new(f64::NAN, 20.25), DpiScale::ONE),
            None
        );
        assert_eq!(
            logical_point_from_winit(PhysicalPosition::new(f64::MAX, 20.25), DpiScale::ONE),
            None
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_keypress_keeps_control_distinct_from_command() {
        let command = keypress_from_input(KeyCode::Space, ModifiersState::SUPER);
        assert!(command.command);
        assert!(!command.control);

        let control = keypress_from_input(KeyCode::Space, ModifiersState::CONTROL);
        assert!(!control.command);
        assert!(control.control);
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn non_macos_keypress_treats_control_as_platform_command() {
        let control = keypress_from_input(KeyCode::Space, ModifiersState::CONTROL);
        assert!(control.command);
        assert!(!control.control);
    }
}
