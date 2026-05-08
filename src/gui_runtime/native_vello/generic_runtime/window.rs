//! Window attribute construction for the generic native Vello runtime.

use crate::gui_runtime::native_vello::*;

pub(super) fn generic_window_attributes(options: &NativeRunOptions) -> WindowAttributes {
    let mut attrs = Window::default_attributes()
        .with_title(options.title.clone())
        .with_maximized(options.maximized)
        .with_decorations(options.decorations)
        .with_visible(false);
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
        use winit::platform::windows::WindowAttributesExtWindows;
        attrs = attrs.with_drag_and_drop(true);
    }
    attrs
}
