mod text;

pub use text::{PaintOverlayPanel, PaintTextAlign, PaintTextInput, PaintTextRun};

use crate::{
    gui::types::{ImageRgba, Point, Rect, Rgba8},
    layout::NodeId,
    runtime::{GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceOverlay},
    theme::ThemeTokens,
    widgets::{PaintBounds, RetainedSurfaceDescriptor, WidgetId},
};
use std::sync::Arc;

/// Filled rectangle primitive in logical surface coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintFillRect {
    /// Widget that produced this primitive.
    pub widget_id: WidgetId,
    /// Rectangle to fill.
    pub rect: Rect,
    /// Fill color.
    pub color: Rgba8,
}

/// Stroked rectangle primitive in logical surface coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintStrokeRect {
    /// Widget that produced this primitive.
    pub widget_id: WidgetId,
    /// Rectangle to stroke.
    pub rect: Rect,
    /// Stroke color.
    pub color: Rgba8,
    /// Stroke width in logical pixels.
    pub width: f32,
}

/// Filled polygon primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintFillPolygon {
    /// Widget or node that produced this primitive.
    pub widget_id: WidgetId,
    /// Polygon points in clockwise or counter-clockwise order.
    pub points: Vec<Point>,
    /// Fill color.
    pub color: Rgba8,
}

/// Stroked polygon primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintStrokePolygon {
    /// Widget or node that produced this primitive.
    pub widget_id: WidgetId,
    /// Polygon points in clockwise or counter-clockwise order.
    pub points: Vec<Point>,
    /// Stroke color.
    pub color: Rgba8,
    /// Stroke width in logical pixels.
    pub width: f32,
}

/// Stroked open polyline primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintStrokePolyline {
    /// Widget or node that produced this primitive.
    pub widget_id: WidgetId,
    /// Connected line points in paint order.
    pub points: Vec<Point>,
    /// Stroke color.
    pub color: Rgba8,
    /// Stroke width in logical pixels.
    pub width: f32,
}

/// Placeholder primitive for widgets whose paint is intentionally host-defined.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintCustomSurface {
    /// Widget that owns this custom surface.
    pub widget_id: WidgetId,
    /// Assigned widget rectangle.
    pub rect: Rect,
    /// Whether the custom paint is clipped to the assigned rectangle.
    pub bounds: PaintBounds,
    /// Optional retained-surface metadata supplied by the host.
    pub retained: Option<RetainedSurfaceDescriptor>,
}

/// Textured RGBA image primitive in logical surface coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintImage {
    /// Widget that produced this image primitive.
    pub widget_id: WidgetId,
    /// Optional source rectangle in image-pixel coordinates.
    ///
    /// When omitted, the full image is stretched into `rect`.
    pub source_rect: Option<Rect>,
    /// Destination rectangle.
    pub rect: Rect,
    /// Shared RGBA image payload.
    pub image: Arc<ImageRgba>,
}

/// Retained GPU surface drawn by native GPU backends.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintGpuSurface {
    /// Widget that produced this GPU surface.
    pub widget_id: WidgetId,
    /// Stable surface key used to retain GPU resources across frames.
    pub key: u64,
    /// Monotonic content revision for retained GPU resources.
    pub revision: u64,
    /// Destination rectangle in logical surface coordinates.
    pub rect: Rect,
    /// Backend-neutral retained content payload.
    pub content: GpuSurfaceContent,
    /// Runtime interaction capabilities requested by this GPU surface.
    pub capabilities: GpuSurfaceCapabilities,
    /// Optional lightweight overlays composited by the native GPU backend.
    pub overlays: Vec<GpuSurfaceOverlay>,
}

/// Begin clipping subsequent paint primitives to a rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintClipStart {
    /// Container node that owns this clip.
    pub node_id: NodeId,
    /// Clip rectangle in logical surface coordinates.
    pub rect: Rect,
}

/// End the most recent paint clip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintClipEnd {
    /// Container node that owns the matching clip.
    pub node_id: NodeId,
}

/// One backend-neutral primitive emitted by a generic surface projection.
#[derive(Clone, Debug, PartialEq)]
pub enum PaintPrimitive {
    /// Begin a rectangular clip.
    ClipStart(PaintClipStart),
    /// End the current clip.
    ClipEnd(PaintClipEnd),
    /// Fill a rectangle.
    FillRect(PaintFillRect),
    /// Stroke a rectangle.
    StrokeRect(PaintStrokeRect),
    /// Fill a polygon.
    FillPolygon(PaintFillPolygon),
    /// Stroke a polygon.
    StrokePolygon(PaintStrokePolygon),
    /// Stroke an open polyline.
    StrokePolyline(PaintStrokePolyline),
    /// Paint one text run.
    Text(PaintTextRun),
    /// Paint a floating overlay panel above normal layout content.
    OverlayPanel(PaintOverlayPanel),
    /// Paint a single-line text input value, selection, and caret.
    TextInput(PaintTextInput),
    /// Paint an RGBA image stretched into one destination rectangle.
    Image(PaintImage),
    /// Paint a retained generic GPU surface using native GPU resources when available.
    GpuSurface(PaintGpuSurface),
    /// Reserve a host-painted custom surface.
    CustomSurface(PaintCustomSurface),
}

/// Deterministic backend-neutral paint output for a generic [`crate::runtime::UiSurface`].
#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePaintPlan {
    /// Clear color a backend may use before replaying primitives.
    pub clear_color: Rgba8,
    /// Primitives in declarative surface tree order.
    pub primitives: Vec<PaintPrimitive>,
}

/// Primitive counts for one backend-neutral [`SurfacePaintPlan`].
///
/// These stats are intended for diagnostics, benchmarks, and host renderers
/// that need to inspect the shape of a frame without duplicating primitive
/// matching logic.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SurfacePaintStats {
    /// Total number of paint primitives.
    pub total: usize,
    /// Filled rectangle or polygon primitives.
    pub fills: usize,
    /// Stroked rectangle, polygon, or polyline primitives.
    pub strokes: usize,
    /// Text-bearing primitives, including text input paint.
    pub text: usize,
    /// Clip start/end primitives.
    pub clips: usize,
    /// Image primitives.
    pub images: usize,
    /// Floating overlay panel primitives.
    pub overlay_panels: usize,
    /// Host-painted custom surface placeholders.
    pub custom_surfaces: usize,
    /// Retained GPU surface primitives.
    pub gpu_surfaces: usize,
}

/// Backend-neutral renderer contract for generic Radiant paint plans.
///
/// A renderer consumes the deterministic [`SurfacePaintPlan`] emitted by a
/// [`crate::runtime::View`] or [`crate::runtime::SurfaceRuntime`]. Renderer
/// implementations own backend resources and frame submission policy; Radiant
/// only defines the replayable paint-plan boundary.
pub trait Renderer {
    /// Backend-specific error type.
    type Error;

    /// Render one backend-neutral paint plan.
    fn render(&mut self, plan: &SurfacePaintPlan) -> Result<(), Self::Error>;
}

impl SurfacePaintPlan {
    /// Build an empty paint plan for the provided theme.
    pub fn empty(theme: &ThemeTokens) -> Self {
        Self {
            clear_color: theme.clear_color,
            primitives: Vec::new(),
        }
    }

    /// Count primitive categories in this paint plan.
    pub fn stats(&self) -> SurfacePaintStats {
        let mut stats = SurfacePaintStats {
            total: self.primitives.len(),
            ..SurfacePaintStats::default()
        };
        for primitive in &self.primitives {
            match primitive {
                PaintPrimitive::ClipStart(_) | PaintPrimitive::ClipEnd(_) => stats.clips += 1,
                PaintPrimitive::FillRect(_) | PaintPrimitive::FillPolygon(_) => stats.fills += 1,
                PaintPrimitive::StrokeRect(_)
                | PaintPrimitive::StrokePolygon(_)
                | PaintPrimitive::StrokePolyline(_) => stats.strokes += 1,
                PaintPrimitive::Text(_) | PaintPrimitive::TextInput(_) => stats.text += 1,
                PaintPrimitive::OverlayPanel(_) => stats.overlay_panels += 1,
                PaintPrimitive::Image(_) => stats.images += 1,
                PaintPrimitive::CustomSurface(_) => stats.custom_surfaces += 1,
                PaintPrimitive::GpuSurface(_) => stats.gpu_surfaces += 1,
            }
        }
        stats
    }
}
