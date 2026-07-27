use super::{NativePointerGesture, pointer_modifiers_from_winit};
use crate::widgets::{PointerButton, PointerModifiers};
use winit::keyboard::ModifiersState;

#[cfg(target_os = "macos")]
pub(super) fn native_pointer_press_gesture(
    button: Option<PointerButton>,
    modifiers: ModifiersState,
) -> Option<NativePointerGesture> {
    super::native_pointer_press_gesture_for_platform(button, modifiers, true)
}

#[cfg(not(target_os = "macos"))]
pub(super) fn native_pointer_press_gesture(
    button: Option<PointerButton>,
    modifiers: ModifiersState,
) -> Option<NativePointerGesture> {
    super::native_pointer_press_gesture_for_platform(button, modifiers, false)
}

#[cfg(target_os = "macos")]
pub(super) fn pointer_modifiers_for_native_gesture(
    modifiers: ModifiersState,
    consume_control: bool,
) -> PointerModifiers {
    let mut projected = pointer_modifiers_from_winit(modifiers);
    if consume_control {
        projected.command = modifiers.super_key();
    }
    projected
}

#[cfg(not(target_os = "macos"))]
pub(super) fn pointer_modifiers_for_native_gesture(
    modifiers: ModifiersState,
    _consume_control: bool,
) -> PointerModifiers {
    pointer_modifiers_from_winit(modifiers)
}

#[cfg(target_os = "macos")]
pub(super) fn command_modifier_from_winit(modifiers: ModifiersState) -> bool {
    modifiers.super_key()
}

#[cfg(not(target_os = "macos"))]
pub(super) fn command_modifier_from_winit(modifiers: ModifiersState) -> bool {
    modifiers.control_key() || modifiers.super_key()
}

#[cfg(target_os = "macos")]
pub(super) fn control_modifier_from_winit(modifiers: ModifiersState) -> bool {
    modifiers.control_key()
}

#[cfg(not(target_os = "macos"))]
pub(super) fn control_modifier_from_winit(_modifiers: ModifiersState) -> bool {
    false
}
