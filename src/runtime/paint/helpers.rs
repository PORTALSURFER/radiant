use super::{
    PaintFillRect, PaintFillRectBatch, PaintPrimitive, PaintStrokeRect, PaintStrokeRectBatch,
    PaintText, PaintTextAlign, PaintTextRun,
};
use crate::{
    gui::types::{Rect, Rgba8},
    widgets::{TextWrap, WidgetId},
};
use std::sync::Arc;

/// Text metrics used by paint-helper text runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintTextMetrics {
    /// Font size in logical pixels.
    pub font_size: f32,
    /// Optional baseline offset in logical pixels.
    pub baseline: Option<f32>,
}

impl PaintTextMetrics {
    /// Create explicit text metrics.
    pub const fn new(font_size: f32, baseline: Option<f32>) -> Self {
        Self {
            font_size,
            baseline,
        }
    }
}

impl Default for PaintTextMetrics {
    fn default() -> Self {
        Self {
            font_size: 12.0,
            baseline: Some(16.0),
        }
    }
}

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

/// Push a single-line text run with explicit metrics into a runtime paint primitive buffer.
pub fn push_text_run_with_metrics(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    text: impl Into<String>,
    rect: Rect,
    color: Rgba8,
    align: PaintTextAlign,
    metrics: PaintTextMetrics,
) {
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text: PaintText::from(text.into()),
        rect,
        font_size: metrics.font_size,
        baseline: metrics.baseline,
        color,
        align,
        wrap: TextWrap::None,
    }));
}

/// Push a compact default single-line text run into a runtime paint primitive buffer.
pub fn push_text(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    text: impl Into<String>,
    rect: Rect,
    color: Rgba8,
    align: PaintTextAlign,
) {
    push_text_run_with_metrics(
        primitives,
        widget_id,
        text,
        rect,
        color,
        align,
        PaintTextMetrics::default(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::{Point, Vector2};

    #[test]
    fn empty_rect_batches_do_not_emit_primitives() {
        let mut primitives = Vec::new();
        push_fill_rect_batch(&mut primitives, 7, Vec::new(), Rgba8::new(1, 2, 3, 4));
        push_stroke_rect_batch(&mut primitives, 7, Vec::new(), Rgba8::new(1, 2, 3, 4), 1.0);
        assert!(primitives.is_empty());
    }

    #[test]
    fn text_helper_uses_default_single_line_metrics() {
        let mut primitives = Vec::new();
        push_text(
            &mut primitives,
            9,
            "Label",
            Rect::from_min_size(Point::new(1.0, 2.0), Vector2::new(30.0, 18.0)),
            Rgba8::new(4, 5, 6, 255),
            PaintTextAlign::Left,
        );

        let [PaintPrimitive::Text(text)] = primitives.as_slice() else {
            panic!("expected text primitive");
        };
        assert_eq!(text.widget_id, 9);
        assert_eq!(text.text.as_str(), "Label");
        assert_eq!(text.font_size, 12.0);
        assert_eq!(text.baseline, Some(16.0));
        assert_eq!(text.wrap, TextWrap::None);
    }
}
