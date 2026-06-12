use crate::{
    gui::types::{Rect, Rgba8},
    runtime::{PaintFillRectBatch, PaintPrimitive, PaintStrokeRectBatch},
    widgets::WidgetId,
};
use std::sync::Arc;

/// Push a same-color filled rectangle batch into a runtime paint primitive buffer.
pub fn push_fill_rect_batch(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    rects: Vec<Rect>,
    color: Rgba8,
) {
    if rects.is_empty() {
        return;
    }
    primitives.push(PaintPrimitive::FillRectBatch(PaintFillRectBatch {
        widget_id,
        rects: Arc::from(rects),
        color,
    }));
}

/// Push a same-color stroked rectangle batch into a runtime paint primitive buffer.
pub fn push_stroke_rect_batch(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    rects: Vec<Rect>,
    color: Rgba8,
    width: f32,
) {
    if rects.is_empty() {
        return;
    }
    primitives.push(PaintPrimitive::StrokeRectBatch(PaintStrokeRectBatch {
        widget_id,
        rects: Arc::from(rects),
        color,
        width,
    }));
}
