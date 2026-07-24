//! Backend-neutral retained GPU surface model.

use crate::gui::types::Rgba8;

mod content;
mod signal_summary;
pub use content::{
    GpuShaderSurfaceDescriptor, GpuShaderSurfaceDescriptorParts, GpuSignalGainPreview,
    GpuSignalRenderShape, GpuSurfaceContent, GpuSurfaceContentError,
    RenderCanvasShaderSurfaceDescriptor, RenderCanvasShaderSurfaceDescriptorParts,
};
pub use signal_summary::{GpuSignalSummary, GpuSignalSummaryBucket, GpuSignalSummaryLevel};

/// Renderer-neutral retained canvas content.
///
/// The current native implementation stores this payload in the retained GPU
/// surface path, but application code should use this canvas vocabulary rather
/// than naming the active renderer.
pub type RenderCanvasContent = GpuSurfaceContent;

/// Validation error returned for invalid retained canvas content.
pub type RenderCanvasContentError = GpuSurfaceContentError;

/// Runtime interaction capabilities for retained GPU surfaces.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuSurfaceCapabilities {
    /// Whether pointer motion inside this surface can update runtime-owned overlays
    /// without refreshing the projected app surface.
    pub fast_pointer_move: bool,
    /// Whether vertical wheel deltas over this surface can be coalesced until redraw.
    pub coalesce_vertical_wheel: bool,
    /// Runtime-owned overlay policies for this surface.
    pub runtime_overlays: GpuSurfaceRuntimeOverlays,
}

/// Runtime interaction capabilities for a retained render canvas.
pub type RenderCanvasCapabilities = GpuSurfaceCapabilities;

/// Runtime-owned overlay policies for retained GPU surfaces.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuSurfaceRuntimeOverlays {
    /// Optional pointer-following vertical line style.
    pub pointer_vertical_line: Option<GpuSurfaceLineStyle>,
}

/// Runtime-owned overlay policies for retained render canvases.
pub type RenderCanvasRuntimeOverlays = GpuSurfaceRuntimeOverlays;

impl GpuSurfaceRuntimeOverlays {
    /// Build runtime overlays with a pointer-following vertical line enabled.
    pub fn pointer_vertical_line(style: GpuSurfaceLineStyle) -> Self {
        Self {
            pointer_vertical_line: Some(style),
        }
    }
}

/// Generic line styling for retained GPU-surface overlays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuSurfaceLineStyle {
    /// Line color.
    pub color: Rgba8,
    /// Line width in logical pixels.
    pub width: f32,
}

/// Generic line styling for retained render-canvas overlays.
pub type RenderCanvasLineStyle = GpuSurfaceLineStyle;

/// Lightweight GPU-surface overlay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GpuSurfaceOverlay {
    /// Vertical cursor line positioned as a 0..1 ratio inside the destination rect.
    VerticalCursor {
        /// Horizontal cursor position as a 0..1 ratio inside the destination rect.
        ratio: f32,
        /// Cursor color.
        color: Rgba8,
        /// Cursor width in logical pixels.
        width: f32,
    },
    /// Runtime-owned vertical line positioned inside the destination rect.
    RuntimeVerticalLine {
        /// Horizontal line position as a 0..1 ratio inside the destination rect.
        ratio: f32,
        /// Line color.
        color: Rgba8,
        /// Line width in logical pixels.
        width: f32,
    },
    /// Filled horizontal range positioned as 0..1 ratios inside the destination rect.
    HorizontalRange {
        /// Inclusive range start as a 0..1 ratio inside the destination rect.
        start: f32,
        /// Inclusive range end as a 0..1 ratio inside the destination rect.
        end: f32,
        /// Range fill color.
        color: Rgba8,
    },
}

/// Lightweight retained render-canvas overlay.
pub type RenderCanvasOverlay = GpuSurfaceOverlay;
