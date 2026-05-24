//! Backend-neutral paint plans emitted from generic Radiant surfaces.

mod chrome;
mod debug;
mod geometry;
mod primitives;
mod scroll;
mod text;

pub(super) use chrome::{push_container_chrome, push_overlay_panel};
pub(super) use debug::{push_clip_end, push_clip_start, push_layout_debug_overlay};
pub(crate) use geometry::{blend_color, diagonal_cut_rect_points, inset_rect, push_axis_stroke};
pub use primitives::{
    PaintClipEnd, PaintClipStart, PaintCustomSurface, PaintFillPath, PaintFillPolygon,
    PaintFillRect, PaintFillRule, PaintGpuSurface, PaintImage, PaintOverlayPanel, PaintPath,
    PaintPathCommand, PaintPointList, PaintPrimitive, PaintStrokePolygon, PaintStrokePolyline,
    PaintStrokeRect, PaintSvg, PaintSvgDocument, PaintText, PaintTextAlign, PaintTextInput,
    PaintTextRun, PaintTransform, Renderer, SurfacePaintPlan, SurfacePaintStats, SvgParseError,
    TransientOverlayContext,
};
pub(super) use scroll::{
    push_scroll_affordance, resolve_scroll_affordance, scroll_content_clip_rect,
};
pub(crate) use text::{
    button_font_size, input_font_size, optical_centered_baseline, push_text_run, text_font_size,
};
