//! Backend-neutral paint plans emitted from generic Radiant surfaces.

mod chrome;
mod debug;
mod geometry;
mod helpers;
mod primitives;
mod scroll;
mod segments;
mod text;

pub(super) use chrome::push_container_chrome;
pub(crate) use chrome::{push_overlay_panel_with_environment, push_tooltip_panel_with_environment};
pub(super) use debug::{push_clip_end, push_clip_start, push_layout_debug_overlay_for_node};
pub(crate) use geometry::{blend_color, diagonal_cut_rect_points, inset_rect, push_axis_stroke};
pub use helpers::{
    PaintTextMetrics, WidgetPaint, push_fill_polygon, push_fill_rect, push_fill_rect_batch,
    push_stroke_polyline, push_stroke_rect, push_stroke_rect_batch, push_text,
    push_text_run_with_metrics, push_visible_fill_rect,
};
pub use primitives::{
    PaintBrush, PaintClipEnd, PaintClipStart, PaintCustomSurface, PaintFillPath, PaintFillPolygon,
    PaintFillRect, PaintFillRectBatch, PaintFillRule, PaintGpuSurface, PaintImage,
    PaintLinearGradient, PaintOverlayPanel, PaintPath, PaintPathCommand, PaintPointList,
    PaintPrimitive, PaintRectList, PaintRenderCanvas, PaintStrokePolygon, PaintStrokePolyline,
    PaintStrokeRect, PaintStrokeRectBatch, PaintSvg, PaintSvgDocument, PaintText, PaintTextAlign,
    PaintTextInput, PaintTextRun, PaintTransform, Renderer, SurfacePaintPlan, SurfacePaintStats,
    SvgParseError, TransientOverlayContext,
};
pub(super) use scroll::{
    push_horizontal_scroll_affordance, push_scroll_affordance,
    resolve_horizontal_scroll_affordance, resolve_scroll_affordance, scroll_content_clip_rect,
    scrollbar_viewport,
};
pub(crate) use segments::{
    MAX_PAINT_SEGMENTS, PaintSegmentIdentity, PaintSegmentObservation, PaintSegmentObserver,
    PaintSegmentSpan, collect_segment_spans,
};
#[cfg(test)]
pub(crate) use segments::{PaintSegment, PaintSegmentAnchor};
pub(crate) use text::{
    button_font_size, input_font_size_for_height, optical_centered_baseline, push_text_run,
    text_font_size, text_font_size_for_height,
};
