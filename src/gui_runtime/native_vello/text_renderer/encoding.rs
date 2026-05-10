//! Primitive conversions shared by Vello scene and window integration.

use super::super::WindowIconRgba;
use crate::gui::types::{Rect as UiRect, Rgba8};
use vello::{kurbo::Rect as KurboRect, peniko::Color};
use winit::window::Icon;

pub(in crate::gui_runtime::native_vello) fn to_kurbo_rect(rect: UiRect) -> KurboRect {
    KurboRect::new(
        rect.min.x as f64,
        rect.min.y as f64,
        rect.max.x as f64,
        rect.max.y as f64,
    )
}

pub(in crate::gui_runtime::native_vello) fn color_from_rgba(color: Rgba8) -> Color {
    Color::from_rgba8(color.r, color.g, color.b, color.a)
}

pub(in crate::gui_runtime::native_vello) fn icon_from_rgba(icon: &WindowIconRgba) -> Option<Icon> {
    Icon::from_rgba(icon.rgba.clone(), icon.width, icon.height).ok()
}
