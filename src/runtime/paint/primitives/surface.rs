use crate::{
    gui::types::{ImageRgba, Rect},
    runtime::{GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceOverlay},
    widgets::{PaintBounds, RetainedSurfaceDescriptor, WidgetId},
};
use std::sync::Arc;

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

/// Renderer-neutral retained render-canvas paint entry.
pub type PaintRenderCanvas = PaintGpuSurface;
