use crate::gui::types::{Point, Rect, Vector2};
use crate::runtime::{
    PaintFillPolygon, PaintFillRect, PaintPrimitive, PaintStrokePolygon, PaintStrokePolyline,
    PaintStrokeRect, diagonal_cut_rect_points, inset_rect,
};
use crate::theme::ThemeTokens;
use crate::widgets::contract::{WidgetId, WidgetState};
use crate::widgets::primitives::WidgetCommon;

pub(in crate::widgets::primitives) fn push_button_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    common: &WidgetCommon,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = crate::widgets::resolve_widget_visual_tokens(theme, common.style, common.state);
    let points = diagonal_cut_rect_points(bounds);
    primitives.push(PaintPrimitive::FillPolygon(PaintFillPolygon {
        widget_id: common.id,
        points: std::sync::Arc::clone(&points),
        color: tokens.fill,
    }));
    primitives.push(PaintPrimitive::StrokePolygon(PaintStrokePolygon {
        widget_id: common.id,
        points,
        color: tokens.border,
        width: 1.0,
    }));
    if common.state.focused && common.paint.paints_focus {
        primitives.push(PaintPrimitive::StrokePolygon(PaintStrokePolygon {
            widget_id: common.id,
            points: diagonal_cut_rect_points(inset_rect(bounds, -1.0, -1.0)),
            color: tokens.emphasis,
            width: 1.0,
        }));
    }
}

pub(in crate::widgets::primitives) fn push_control_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    common: &WidgetCommon,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = crate::widgets::resolve_widget_visual_tokens(theme, common.style, common.state);
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: common.id,
        rect: bounds,
        color: tokens.fill,
    }));
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id: common.id,
        rect: bounds,
        color: tokens.border,
        width: 1.0,
    }));
    if common.state.focused && common.paint.paints_focus {
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: common.id,
            rect: inset_rect(bounds, -1.0, -1.0),
            color: tokens.emphasis,
            width: 1.0,
        }));
    }
}

pub(in crate::widgets::primitives) fn push_checkbox_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    theme: &ThemeTokens,
    state: WidgetState,
    checked: bool,
) {
    let side = bounds.width().min(bounds.height()).max(0.0);
    let bounds = Rect::from_min_size(
        Point::new(
            bounds.min.x + (bounds.width() - side) * 0.5,
            bounds.min.y + (bounds.height() - side) * 0.5,
        ),
        Vector2::new(side, side),
    );
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect: bounds,
        color: if state.pressed {
            theme.bg_tertiary
        } else if state.hovered {
            theme.surface_raised
        } else {
            theme.surface_base
        },
    }));
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect: bounds,
        color: if checked || state.pressed || state.hovered {
            theme.accent_danger
        } else {
            theme.border_emphasis
        },
        width: 1.0,
    }));
    if checked {
        primitives.push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
            widget_id,
            points: [
                Point::new(bounds.min.x + side * 0.25, bounds.min.y + side * 0.55),
                Point::new(bounds.min.x + side * 0.43, bounds.min.y + side * 0.72),
                Point::new(bounds.min.x + side * 0.76, bounds.min.y + side * 0.30),
            ]
            .into(),
            color: theme.accent_danger,
            width: 2.0,
        }));
    }
}
