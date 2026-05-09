mod badge;
mod button;
mod drag_handle;
mod list_item;
mod scrollbar;
mod selectable;
mod toggle;

use super::chrome::push_control_chrome;
use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextAlign, optical_centered_baseline, push_text_run, text_font_size,
};
use crate::theme::ThemeTokens;

use super::super::super::{card::CardWidget, text::TextWidget};

pub(in crate::widgets::primitives) use badge::push_badge_widget_paint;
pub(in crate::widgets::primitives) use button::push_button_widget_paint;
pub(in crate::widgets::primitives) use drag_handle::push_drag_handle_widget_paint;
pub(in crate::widgets::primitives) use list_item::push_list_item_widget_paint;
pub(in crate::widgets::primitives) use scrollbar::push_scrollbar_widget_paint;
pub(in crate::widgets::primitives) use selectable::push_selectable_widget_paint;
pub(in crate::widgets::primitives) use toggle::push_toggle_widget_paint;

pub(in crate::widgets::primitives) fn push_text_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    text: &TextWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_text_run(
        primitives,
        text.common.id,
        text.text.clone(),
        bounds,
        optical_centered_baseline(bounds, text_font_size(bounds)),
        theme.text_primary,
        PaintTextAlign::Left,
        text.wrap,
        text_font_size(bounds),
    );
}

pub(in crate::widgets::primitives) fn push_card_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    card: &CardWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &card.common, bounds, theme);
}
