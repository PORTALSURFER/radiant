use super::{
    PaintFillPolygon, PaintFillRect, PaintFillRectBatch, PaintPrimitive, PaintStrokePolyline,
    PaintStrokeRect, PaintStrokeRectBatch, PaintText, PaintTextAlign, PaintTextRun,
};
use crate::{
    gui::types::{Point, Rect, Rgba8},
    widgets::{TextWrap, WidgetId},
};
use std::sync::Arc;

/// Paint primitive sink bound to one widget id.
///
/// Custom widgets can use this to append several primitives without passing the
/// same `Vec<PaintPrimitive>` and `WidgetId` through every helper call.
pub struct WidgetPaint<'a> {
    primitives: &'a mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
}

impl<'a> WidgetPaint<'a> {
    /// Create a paint sink for primitives emitted by `widget_id`.
    pub fn new(primitives: &'a mut Vec<PaintPrimitive>, widget_id: WidgetId) -> Self {
        Self {
            primitives,
            widget_id,
        }
    }

    /// Return the widget id attached to primitives emitted by this sink.
    pub const fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// Return the underlying primitive buffer for specialized helpers.
    pub fn primitives_mut(&mut self) -> &mut Vec<PaintPrimitive> {
        self.primitives
    }

    /// Push a filled rectangle into this widget's paint primitive buffer.
    pub fn push_fill_rect(&mut self, rect: Rect, color: Rgba8) {
        push_fill_rect(self.primitives, self.widget_id, rect, color);
    }

    /// Push a filled rectangle when it has finite positive area.
    pub fn push_visible_fill_rect(&mut self, rect: Rect, color: Rgba8) -> bool {
        push_visible_fill_rect(self.primitives, self.widget_id, rect, color)
    }

    /// Push a stroked rectangle into this widget's paint primitive buffer.
    pub fn push_stroke_rect(&mut self, rect: Rect, color: Rgba8, width: f32) {
        push_stroke_rect(self.primitives, self.widget_id, rect, color, width);
    }

    /// Push a compact default single-line text run for this widget.
    pub fn push_text(
        &mut self,
        text: impl Into<String>,
        rect: Rect,
        color: Rgba8,
        align: PaintTextAlign,
    ) {
        push_text(self.primitives, self.widget_id, text, rect, color, align);
    }
}

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
    fn push_visible_fill_rect_skips_empty_or_invalid_rects() {
        let mut primitives = Vec::new();
        let color = Rgba8::new(1, 2, 3, 4);

        assert!(!push_visible_fill_rect(
            &mut primitives,
            7,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(0.0, 12.0)),
            color,
        ));
        assert!(!push_visible_fill_rect(
            &mut primitives,
            7,
            Rect::from_min_size(Point::new(f32::NAN, 0.0), Vector2::new(12.0, 12.0)),
            color,
        ));
        assert!(primitives.is_empty());
    }

    #[test]
    fn push_visible_fill_rect_appends_positive_rects() {
        let mut primitives = Vec::new();
        let color = Rgba8::new(1, 2, 3, 4);
        let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(12.0, 8.0));

        assert!(push_visible_fill_rect(&mut primitives, 7, rect, color));
        assert_eq!(primitives.len(), 1);
        assert!(matches!(
            &primitives[0],
            PaintPrimitive::FillRect(fill)
                if fill.widget_id == 7 && fill.rect == rect && fill.color == color
        ));
    }

    #[test]
    fn widget_paint_binds_primitives_to_one_widget_id() {
        let mut primitives = Vec::new();
        let color = Rgba8::new(1, 2, 3, 4);
        let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(12.0, 8.0));

        let mut paint = WidgetPaint::new(&mut primitives, 11);
        assert_eq!(paint.widget_id(), 11);
        assert!(paint.push_visible_fill_rect(rect, color));
        paint.push_stroke_rect(rect, color, 2.0);

        assert_eq!(primitives.len(), 2);
        assert!(matches!(
            &primitives[0],
            PaintPrimitive::FillRect(fill)
                if fill.widget_id == 11 && fill.rect == rect && fill.color == color
        ));
        assert!(matches!(
            &primitives[1],
            PaintPrimitive::StrokeRect(stroke)
                if stroke.widget_id == 11 && stroke.rect == rect && stroke.color == color
        ));
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
