//! Drag handle paint command generation.

use crate::gui::types::{Point, Rect};
use crate::runtime::{
    PaintFillRect, PaintPrimitive, PaintStrokePolyline, PaintStrokeRect, inset_rect,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::drag_handle::DragHandleWidget;

pub(super) fn push_drag_handle_widget_paint(
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
    let active_color = if handle.common.state.pressed {
        theme.accent_danger
    } else {
        tokens.emphasis
    };
    let hover_highlight_visible = handle.hover_highlight_visible();
    if let Some(width) = handle.trailing_rail_width {
        let width = width.min(bounds.width());
        let rail = Rect::from_min_size(
            Point::new(bounds.max.x - width, bounds.min.y),
            crate::gui::types::Vector2::new(width, bounds.height()),
        );
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: handle.common.id,
            rect: rail,
            color: if hover_highlight_visible || handle.common.state.pressed {
                active_color
            } else {
                theme.border_emphasis
            },
        }));
        return;
    }
    if handle.full_height_rail {
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: handle.common.id,
            rect: bounds,
            color: theme.border_emphasis,
        }));
    }
    if handle.hover_chrome_only
        && !hover_highlight_visible
        && !handle.common.state.pressed
        && !handle.common.state.focused
    {
        return;
    }
    let color = if handle.common.state.pressed {
        theme.accent_danger
    } else if hover_highlight_visible {
        tokens.emphasis
    } else {
        theme.text_muted
    };
    let center_y = bounds.min.y + bounds.height() * 0.5;
    for y in [center_y - 5.0, center_y, center_y + 5.0] {
        primitives.push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
            widget_id: handle.common.id,
            points: [
                Point::new(bounds.min.x + bounds.width() * 0.25, y),
                Point::new(bounds.max.x - bounds.width() * 0.25, y),
            ]
            .into(),
            color,
            width: if handle.common.state.pressed {
                2.0
            } else {
                1.25
            },
        }));
    }
    if hover_highlight_visible || handle.common.state.pressed {
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: handle.common.id,
            rect: inset_rect(bounds, 2.0, 2.0),
            color,
            width: 1.0,
        }));
    }
}
