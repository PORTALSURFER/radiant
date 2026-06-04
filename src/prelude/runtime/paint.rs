//! Backend-neutral paint primitive prelude exports.

pub use crate::runtime::{
    PaintClipEnd, PaintClipStart, PaintFillPath, PaintFillRect, PaintFillRectBatch, PaintFillRule,
    PaintImage, PaintPath, PaintPathCommand, PaintPrimitive, PaintRectList, PaintStrokeRect,
    PaintStrokeRectBatch, PaintSvg, PaintSvgDocument, PaintTextAlign, PaintTextMetrics,
    PaintTextRun, PaintTransform, SurfacePaintPlan, SvgParseError, TransientOverlayContext,
    WidgetPaint, push_fill_polygon, push_fill_rect, push_fill_rect_batch, push_stroke_polyline,
    push_stroke_rect, push_stroke_rect_batch, push_text, push_text_run_with_metrics,
    push_visible_fill_rect,
};
