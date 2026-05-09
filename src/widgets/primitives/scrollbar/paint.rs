//! Scrollbar paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{PaintFillRect, PaintPrimitive, push_axis_stroke};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::scrollbar::{ScrollbarAxis, ScrollbarWidget};

pub(super) fn push_scrollbar_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    scrollbar: &ScrollbarWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = crate::widgets::resolve_widget_visual_tokens(
        theme,
        scrollbar.common.style,
        scrollbar.common.state,
    );
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: scrollbar.common.id,
        rect: bounds,
        color: theme.surface_base,
    }));
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: scrollbar.common.id,
        rect: scrollbar.thumb_rect(bounds),
        color: tokens.emphasis,
    }));
    push_axis_stroke(
        primitives,
        scrollbar.common.id,
        bounds,
        theme.grid_soft,
        scrollbar.props.axis == ScrollbarAxis::Horizontal,
    );
}
