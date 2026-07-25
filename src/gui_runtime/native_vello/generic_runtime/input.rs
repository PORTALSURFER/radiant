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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NativePointerGesture {
    pub(super) button: PointerButton,
    pub(super) consume_control: bool,
}

pub(super) fn native_pointer_press_gesture(
    button: Option<PointerButton>,
    modifiers: winit::keyboard::ModifiersState,
) -> Option<NativePointerGesture> {
    native_pointer_press_gesture_for_platform(button, modifiers, cfg!(target_os = "macos"))
}

fn native_pointer_press_gesture_for_platform(
    button: Option<PointerButton>,
    modifiers: winit::keyboard::ModifiersState,
    macos: bool,
) -> Option<NativePointerGesture> {
    let button = button?;
    let consume_control = macos && button == PointerButton::Primary && modifiers.control_key();
    Some(NativePointerGesture {
        button: if consume_control {
            PointerButton::Secondary
        } else {
            button
        },
        consume_control,
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

pub(super) fn pointer_modifiers_for_native_gesture(
    modifiers: winit::keyboard::ModifiersState,
    consume_control: bool,
) -> PointerModifiers {
    let mut projected = pointer_modifiers_from_winit(modifiers);
    if consume_control && cfg!(target_os = "macos") {
        // On macOS Control is folded into the generic command projection. A
        // converted Control-click consumes only that physical modifier while
        // retaining an independently held Command key.
        projected.command = modifiers.super_key();
    }
    projected
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
    fn native_pointer_gesture_conversion_is_platform_scoped() {
        let control = ModifiersState::CONTROL;
        assert_eq!(
            native_pointer_press_gesture_for_platform(Some(PointerButton::Primary), control, true,),
            Some(NativePointerGesture {
                button: PointerButton::Secondary,
                consume_control: true,
            })
        );
        assert_eq!(
            native_pointer_press_gesture_for_platform(Some(PointerButton::Primary), control, false,),
            Some(NativePointerGesture {
                button: PointerButton::Primary,
                consume_control: false,
            })
        );
        assert_eq!(
            native_pointer_press_gesture_for_platform(
                Some(PointerButton::Secondary),
                control,
                true,
            ),
            Some(NativePointerGesture {
                button: PointerButton::Secondary,
                consume_control: false,
            })
        );
    }

    #[test]
    fn converted_control_click_preserves_independent_modifiers() {
        let projected = pointer_modifiers_for_native_gesture(
            ModifiersState::CONTROL | ModifiersState::SUPER | ModifiersState::SHIFT,
            true,
        );
        assert!(projected.command);
        assert!(projected.shift);
        assert!(!projected.alt);
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
