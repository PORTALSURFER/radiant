//! Paint projection helpers for primitive widget implementations.

mod chrome;
mod controls;
mod media;
mod text_input;

pub(in crate::widgets::primitives) use chrome::{
    push_button_chrome, push_checkbox_chrome, push_control_chrome,
};
pub(in crate::widgets::primitives) use controls::{
    push_badge_widget_paint, push_card_widget_paint, push_drag_handle_widget_paint,
    push_list_item_widget_paint, push_scrollbar_widget_paint, push_selectable_widget_paint,
    push_text_widget_paint,
};
pub(in crate::widgets::primitives) use media::{push_canvas_widget_paint, push_image_widget_paint};
pub(in crate::widgets::primitives) use text_input::push_text_input_widget_paint;
