//! GPU surface paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{PaintGpuSurface, PaintPrimitive};
use crate::widgets::primitives::gpu_surface::GpuSurfaceWidget;

pub(super) fn push_gpu_surface_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    surface: &GpuSurfaceWidget,
    bounds: Rect,
) {
    primitives.push(PaintPrimitive::GpuSurface(PaintGpuSurface {
        widget_id: surface.common.id,
        key: surface.key,
        revision: surface.revision,
        rect: bounds,
        content: surface.content.clone(),
        capabilities: surface.capabilities,
        overlays: surface.overlays.clone(),
    }));
}
