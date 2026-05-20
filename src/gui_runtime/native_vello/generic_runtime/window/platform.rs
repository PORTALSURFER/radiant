//! Platform-specific native window attribute extensions.

use crate::gui_runtime::native_vello::*;

#[cfg(target_os = "windows")]
pub(super) fn apply_drag_and_drop_attributes(
    attrs: WindowAttributes,
    enabled: bool,
) -> WindowAttributes {
    if enabled {
        use winit::platform::windows::WindowAttributesExtWindows;
        attrs.with_drag_and_drop(true)
    } else {
        attrs
    }
}

#[cfg(not(target_os = "windows"))]
pub(super) fn apply_drag_and_drop_attributes(
    attrs: WindowAttributes,
    _enabled: bool,
) -> WindowAttributes {
    attrs
}

#[cfg(target_os = "windows")]
pub(super) fn apply_popup_attributes(
    attrs: WindowAttributes,
    popup: NativePopupOptions,
) -> WindowAttributes {
    if popup.skip_taskbar {
        use winit::platform::windows::WindowAttributesExtWindows;
        attrs.with_skip_taskbar(true)
    } else {
        attrs
    }
}

#[cfg(not(target_os = "windows"))]
pub(super) fn apply_popup_attributes(
    attrs: WindowAttributes,
    _popup: NativePopupOptions,
) -> WindowAttributes {
    attrs
}
