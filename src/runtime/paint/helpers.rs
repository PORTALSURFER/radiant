use super::{
    PaintFillPolygon, PaintFillRect, PaintFillRectBatch, PaintPrimitive, PaintStrokePolyline,
    PaintStrokeRect, PaintStrokeRectBatch, PaintText, PaintTextAlign, PaintTextRun,
};
use crate::{
    gui::types::{Point, Rect, Rgba8},
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

/// Push a filled polygon from generated or caller-owned points.
pub fn push_fill_polygon(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    points: impl IntoIterator<Item = Point>,
    color: Rgba8,
) {
    let points = points.into_iter().collect::<Vec<_>>();
    if points.len() < 3 {
        return;
    }
    primitives.push(PaintPrimitive::FillPolygon(PaintFillPolygon {
        widget_id,
        points: Arc::from(points),
        color,
    }));
}

/// Push an open stroked polyline from generated or caller-owned points.
pub fn push_stroke_polyline(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    points: impl IntoIterator<Item = Point>,
    color: Rgba8,
    width: f32,
) {
    let points = points.into_iter().collect::<Vec<_>>();
    if points.len() < 2 {
        return;
    }
    primitives.push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
        widget_id,
        points: Arc::from(points),
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
    fn polygon_and_polyline_helpers_skip_degenerate_point_lists() {
        let mut primitives = Vec::new();

        push_fill_polygon(
            &mut primitives,
            7,
            [Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
            Rgba8::new(1, 2, 3, 4),
        );
        push_stroke_polyline(
            &mut primitives,
            7,
            [Point::new(0.0, 0.0)],
            Rgba8::new(1, 2, 3, 4),
            1.0,
        );

        assert!(primitives.is_empty());
    }

    #[test]
    fn polygon_and_polyline_helpers_emit_shared_point_lists() {
        let mut primitives = Vec::new();

        push_fill_polygon(
            &mut primitives,
            7,
            [
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0),
            ],
            Rgba8::new(1, 2, 3, 4),
        );
        push_stroke_polyline(
            &mut primitives,
            9,
            [Point::new(1.0, 2.0), Point::new(3.0, 4.0)],
            Rgba8::new(5, 6, 7, 8),
            2.0,
        );

        let [
            PaintPrimitive::FillPolygon(fill),
            PaintPrimitive::StrokePolyline(stroke),
        ] = primitives.as_slice()
        else {
            panic!("expected polygon and polyline primitives");
        };
        assert_eq!(fill.widget_id, 7);
        assert_eq!(fill.points.len(), 3);
        assert_eq!(stroke.widget_id, 9);
        assert_eq!(stroke.points.len(), 2);
        assert_eq!(stroke.width, 2.0);
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
