use crate::{
    gui::types::{Rect, Rgba8},
    runtime::{PaintFillRect, PaintPrimitive, PaintStrokeRect},
    widgets::WidgetId,
};

/// Push a filled rectangle into a runtime paint primitive buffer.
pub fn push_fill_rect(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    rect: Rect,
    color: Rgba8,
) {
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color,
    }));
}

/// Push a filled rectangle only when it has finite positive area.
///
/// Returns `true` when a primitive was appended. This is useful for dense
/// custom widgets that derive paint rects from normalized values, hit regions,
/// or clipped geometry where an empty segment should not enter the paint plan.
pub fn push_visible_fill_rect(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    rect: Rect,
    color: Rgba8,
) -> bool {
    if !rect.has_finite_positive_area() {
        return false;
    }
    push_fill_rect(primitives, widget_id, rect, color);
    true
}

/// Push a stroked rectangle into a runtime paint primitive buffer.
pub fn push_stroke_rect(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    rect: Rect,
    color: Rgba8,
    width: f32,
) {
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect,
        color,
        width,
    }));
}
