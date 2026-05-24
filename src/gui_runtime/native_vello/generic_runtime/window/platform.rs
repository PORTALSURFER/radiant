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
pub(super) fn apply_top_level_attributes(
    mut attrs: WindowAttributes,
    options: &NativeRunOptions,
) -> WindowAttributes {
    use winit::platform::windows::WindowAttributesExtWindows;
    if let Some(owner) = options.window.behavior.owner_window_handle {
        attrs = attrs.with_owner_window(owner as _);
    }
    if options.window.behavior.skip_taskbar {
        attrs = attrs.with_skip_taskbar(true);
    }
    attrs
}

#[cfg(not(target_os = "windows"))]
pub(super) fn apply_top_level_attributes(
    attrs: WindowAttributes,
    options: &NativeRunOptions,
) -> WindowAttributes {
    let _ = options;
    attrs
}

#[cfg(target_os = "windows")]
pub(super) fn owner_window_handle(window: Option<&Window>) -> Option<isize> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let handle = window?.window_handle().ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get()),
        _ => None,
    }
}

#[cfg(not(target_os = "windows"))]
pub(super) fn owner_window_handle(_window: Option<&Window>) -> Option<isize> {
    None
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
