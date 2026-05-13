//! Scrollbar paint command generation.

use crate::gui::types::{Rect, Rgba8};
use crate::runtime::{PaintFillRect, PaintPrimitive, blend_color, push_axis_stroke};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::scrollbar::{ScrollbarAxis, ScrollbarWidget};

pub(super) fn push_scrollbar_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    scrollbar: &ScrollbarWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: scrollbar.common.id,
        rect: bounds,
        color: theme.bg_primary,
    }));
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: scrollbar.common.id,
        rect: scrollbar.thumb_rect(bounds),
        color: thumb_color(scrollbar, theme),
    }));
    push_axis_stroke(
        primitives,
        scrollbar.common.id,
        bounds,
        theme.grid_soft,
        scrollbar.props.axis == ScrollbarAxis::Horizontal,
    );
}

fn thumb_color(scrollbar: &ScrollbarWidget, theme: &ThemeTokens) -> Rgba8 {
    if scrollbar.common.state.pressed {
        return blend_color(theme.border_emphasis, theme.text_muted, 0.30);
    }
    if scrollbar.common.state.hovered {
        return blend_color(theme.grid_strong, theme.border_emphasis, 0.45);
    }
    theme.grid_strong
}
