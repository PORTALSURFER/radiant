//! Shared support for primitive widget implementations.

mod common;
mod input;
mod paint;

pub use common::WidgetCommon;
pub(super) use input::{
    activate_on_keyboard, clamp_fraction, leading_arrow_for_axis, trailing_arrow_for_axis,
};
pub(super) use paint::{
    push_badge_widget_paint, push_canvas_widget_paint, push_card_widget_paint,
    push_drag_handle_widget_paint, push_image_widget_paint, push_list_item_widget_paint,
    push_scrollbar_widget_paint, push_selectable_widget_paint, push_text_input_widget_paint,
    push_text_widget_paint,
};
pub(super) use paint::{push_button_chrome, push_checkbox_chrome, push_control_chrome};
