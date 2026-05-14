//! Window attribute construction for the generic native Vello runtime.

use crate::gui_runtime::native_vello::*;

pub(super) fn generic_window_attributes(options: &NativeRunOptions) -> WindowAttributes {
    let mut attrs = Window::default_attributes()
        .with_title(options.title.clone())
        .with_maximized(options.maximized)
        .with_decorations(options.decorations)
        .with_visible(false);
    if let NativeWindowMode::Popup(popup) = options.window_mode {
        attrs = apply_popup_window_attributes(attrs, popup);
    }
    if let Some([w, h]) = options.inner_size {
        attrs = attrs.with_inner_size(Size::Logical(LogicalSize::new(w as f64, h as f64)));
    }
    if let Some([w, h]) = options.min_inner_size {
        attrs = attrs.with_min_inner_size(Size::Logical(LogicalSize::new(w as f64, h as f64)));
    }
    if let Some(icon) = options.icon.as_ref().and_then(icon_from_rgba) {
        attrs = attrs.with_window_icon(Some(icon));
    }
    #[cfg(target_os = "windows")]
    {
        if platform_drag_and_drop_enabled(options) {
            use winit::platform::windows::WindowAttributesExtWindows;
            attrs = attrs.with_drag_and_drop(true);
        }
    }
    attrs
}

fn apply_popup_window_attributes(
    mut attrs: WindowAttributes,
    popup: NativePopupOptions,
) -> WindowAttributes {
    attrs = attrs
        .with_decorations(false)
        .with_resizable(popup.resizable)
        .with_transparent(popup.transparent)
        .with_active(popup.initially_focused);
    if popup.always_on_top {
        attrs = attrs.with_window_level(WindowLevel::AlwaysOnTop);
    }
    if let Some([x, y]) = popup.position {
        attrs = attrs.with_position(Position::Logical(LogicalPosition::new(x as f64, y as f64)));
    }
    #[cfg(target_os = "windows")]
    {
        if popup.skip_taskbar {
            use winit::platform::windows::WindowAttributesExtWindows;
            attrs = attrs.with_skip_taskbar(true);
        }
    }
    attrs
}

pub(super) fn platform_drag_and_drop_enabled(options: &NativeRunOptions) -> bool {
    options.drag_and_drop && !options.is_popup()
}

pub(super) fn reveal_window_after_surface_setup(options: &NativeRunOptions) -> bool {
    options
        .popup_options()
        .is_none_or(|popup| popup.initially_visible)
}

pub(super) fn reveal_window_after_first_present(_options: &NativeRunOptions) -> bool {
    false
}

pub(super) fn hide_window_after_first_present(options: &NativeRunOptions) -> bool {
    options
        .popup_options()
        .is_some_and(|popup| popup.hide_after_first_present)
}
