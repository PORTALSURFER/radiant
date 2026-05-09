//! Paint projection helpers for primitive widget implementations.

mod chrome;
mod controls;
mod media;

pub(in crate::widgets::primitives) use chrome::{
    push_button_chrome, push_checkbox_chrome, push_control_chrome,
};
pub(in crate::widgets::primitives) use controls::push_text_widget_paint;
pub(in crate::widgets::primitives) use media::{push_canvas_widget_paint, push_image_widget_paint};
