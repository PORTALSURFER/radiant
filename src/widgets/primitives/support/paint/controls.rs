mod badge;
mod button;
mod list_item;
mod scrollbar;
mod selectable;
mod toggle;

use super::chrome::push_control_chrome;
use crate::gui::types::{Point, Rect};
use crate::runtime::{
    PaintPrimitive, PaintStrokePolyline, PaintStrokeRect, PaintTextAlign, inset_rect,
    optical_centered_baseline, push_text_run, text_font_size,
};
use crate::theme::ThemeTokens;

use super::super::super::{card::CardWidget, drag_handle::DragHandleWidget, text::TextWidget};

pub(in crate::widgets::primitives) use badge::push_badge_widget_paint;
pub(in crate::widgets::primitives) use button::push_button_widget_paint;
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

pub(in crate::widgets::primitives) fn push_drag_handle_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    handle: &DragHandleWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    if !handle.common.paint.paints_state_layers {
        return;
    }
    let tokens = crate::widgets::resolve_widget_visual_tokens(
        theme,
        handle.common.style,
        handle.common.state,
    );
    let color = if handle.common.state.pressed {
        theme.accent_danger
    } else if handle.common.state.hovered {
        tokens.emphasis
    } else {
        theme.text_muted
    };
    let center_y = bounds.min.y + bounds.height() * 0.5;
    for y in [center_y - 5.0, center_y, center_y + 5.0] {
        primitives.push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
            widget_id: handle.common.id,
            points: vec![
                Point::new(bounds.min.x + bounds.width() * 0.25, y),
                Point::new(bounds.max.x - bounds.width() * 0.25, y),
            ],
            color,
            width: if handle.common.state.pressed {
                2.0
            } else {
                1.25
            },
        }));
    }
    if handle.common.state.hovered || handle.common.state.pressed {
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: handle.common.id,
            rect: inset_rect(bounds, 2.0, 2.0),
            color,
            width: 1.0,
        }));
    }
}

pub(in crate::widgets::primitives) fn push_card_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    card: &CardWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &card.common, bounds, theme);
}
