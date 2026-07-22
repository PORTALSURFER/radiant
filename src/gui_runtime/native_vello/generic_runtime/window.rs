//! Window attribute construction for the generic native Vello runtime.

use crate::gui_runtime::native_vello::icon_from_rgba;
use crate::gui_runtime::{NativePopupOptions, NativeRunOptions, NativeWindowMode};
use winit::{
    dpi::{LogicalPosition, LogicalSize, Position, Size},
    window::{Window, WindowAttributes, WindowLevel},
};

mod platform;

pub(super) fn generic_window_attributes(options: &NativeRunOptions) -> WindowAttributes {
    let drag_and_drop_enabled = platform_drag_and_drop_enabled(options);
    tracing::info!(
        drag_and_drop_enabled,
        requested = options.window.behavior.drag_and_drop,
        popup = options.is_popup(),
        "radiant generic native vello: configuring native file drag-and-drop"
    );
    let mut attrs = Window::default_attributes()
        .with_title(options.window.title.clone())
        .with_maximized(options.window.behavior.maximized)
        .with_decorations(options.window.behavior.decorations)
        .with_visible(false);
    if let NativeWindowMode::Popup(popup) = options.window.behavior.mode {
        attrs = apply_popup_window_attributes(attrs, popup);
    }
    if let Some([w, h]) = options.window.geometry.inner_size {
        attrs = attrs.with_inner_size(Size::Logical(LogicalSize::new(w as f64, h as f64)));
    }
    if let Some([x, y]) = options.window.geometry.position {
        attrs = attrs.with_position(Position::Logical(LogicalPosition::new(x as f64, y as f64)));
    }
    if let Some([w, h]) = options.window.geometry.min_inner_size {
        attrs = attrs.with_min_inner_size(Size::Logical(LogicalSize::new(w as f64, h as f64)));
    }
    if let Some(icon) = options.window.icon.as_ref().and_then(icon_from_rgba) {
        attrs = attrs.with_window_icon(Some(icon));
    }
    let attrs = platform::apply_drag_and_drop_attributes(attrs, drag_and_drop_enabled);
    platform::apply_top_level_attributes(attrs, options)
}

pub(super) fn configure_created_top_level_window(window: &Window, options: &NativeRunOptions) {
    platform::configure_created_top_level_window(window, options);
}

pub(super) fn drag_app_owned_window(
    window: &Window,
    options: &NativeRunOptions,
) -> Result<(), winit::error::ExternalError> {
    platform::set_integrated_titlebar_window_movable(window, options, true);
    let result = window.drag_window();
    platform::set_integrated_titlebar_window_movable(window, options, false);
    result
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
    platform::apply_popup_attributes(attrs, popup)
}

pub(super) fn platform_drag_and_drop_enabled(options: &NativeRunOptions) -> bool {
    options.window.behavior.drag_and_drop && !options.is_popup()
}

pub(super) fn reveal_window_after_surface_setup(options: &NativeRunOptions) -> bool {
    options.window.behavior.reveal_after_surface_setup
        && options
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

pub(super) fn owner_window_handle(window: Option<&Window>) -> Option<isize> {
    platform::owner_window_handle(window)
}
