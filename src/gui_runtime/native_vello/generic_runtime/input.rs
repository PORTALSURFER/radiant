//! Input mapping for the generic native Vello runtime.

mod key_code;

use crate::{
    gui::input::{KeyCode, KeyPress},
    layout::Point,
    widgets::{PointerButton, PointerModifiers},
};
use winit::dpi::PhysicalPosition;
use winit::event::MouseButton;

pub(super) use key_code::key_code_from_winit;

pub(super) fn logical_point_from_winit(position: PhysicalPosition<f64>) -> Option<Point> {
    let point = Point::new(position.x as f32, position.y as f32);
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
        command: modifiers.control_key() || modifiers.super_key(),
        shift: modifiers.shift_key(),
        alt: modifiers.alt_key(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_point_from_winit_rejects_nonfinite_or_overflowing_coordinates() {
        assert_eq!(
            logical_point_from_winit(PhysicalPosition::new(12.5, 20.25)),
            Some(Point::new(12.5, 20.25))
        );
        assert_eq!(
            logical_point_from_winit(PhysicalPosition::new(f64::NAN, 20.25)),
            None
        );
        assert_eq!(
            logical_point_from_winit(PhysicalPosition::new(f64::MAX, 20.25)),
            None
        );
    }
}
