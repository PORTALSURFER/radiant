//! Platform-specific native window attribute extensions.

use crate::gui_runtime::{NativePopupOptions, NativeRunOptions};
use winit::{window::Window, window::WindowAttributes};

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

#[cfg(target_os = "macos")]
pub(super) fn apply_top_level_attributes(
    attrs: WindowAttributes,
    options: &NativeRunOptions,
) -> WindowAttributes {
    use winit::platform::macos::WindowAttributesExtMacOS;

    if options.window.behavior.integrated_titlebar {
        attrs
            .with_movable_by_window_background(false)
            .with_titlebar_transparent(true)
            .with_title_hidden(true)
            .with_fullsize_content_view(true)
    } else {
        attrs
    }
}

#[cfg(target_os = "macos")]
pub(super) fn configure_created_top_level_window(window: &Window, options: &NativeRunOptions) {
    set_integrated_titlebar_window_movable(window, options, false);
}

#[cfg(not(target_os = "macos"))]
pub(super) fn configure_created_top_level_window(_window: &Window, _options: &NativeRunOptions) {}

#[cfg(target_os = "macos")]
pub(super) fn set_integrated_titlebar_window_movable(
    window: &Window,
    options: &NativeRunOptions,
    movable: bool,
) {
    if !options.window.behavior.integrated_titlebar {
        return;
    }
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::ffi::c_void;

    type Id = *mut c_void;
    type Sel = *mut c_void;
    unsafe extern "C" {
        fn objc_msgSend();
        fn sel_registerName(name: *const std::ffi::c_char) -> Sel;
    }

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };
    let view = handle.ns_view.as_ptr();
    let window_selector = unsafe { sel_registerName(c"window".as_ptr()) };
    let get_window: unsafe extern "C" fn(Id, Sel) -> Id =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    let native_window = unsafe { get_window(view, window_selector) };
    if native_window.is_null() {
        return;
    }
    let movable_selector = unsafe { sel_registerName(c"setMovable:".as_ptr()) };
    let set_movable: unsafe extern "C" fn(Id, Sel, i8) =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { set_movable(native_window, movable_selector, i8::from(movable)) };
}

#[cfg(not(target_os = "macos"))]
pub(super) fn set_integrated_titlebar_window_movable(
    _window: &Window,
    _options: &NativeRunOptions,
    _movable: bool,
) {
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
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
