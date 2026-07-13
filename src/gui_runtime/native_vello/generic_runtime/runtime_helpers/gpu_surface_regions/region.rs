use crate::{
    layout::{Point, Rect},
    runtime::{GpuSurfaceRuntimeOverlays, PaintGpuSurface},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct GpuSurfaceInteractionRegion {
    pub(in crate::gui_runtime::native_vello) primitive_index: usize,
    pub(in crate::gui_runtime::native_vello) widget_id: crate::widgets::WidgetId,
    pub(in crate::gui_runtime::native_vello) rect: Rect,
    pub(in crate::gui_runtime::native_vello) fast_pointer_move: bool,
    pub(in crate::gui_runtime::native_vello) coalesce_vertical_wheel: bool,
    pub(in crate::gui_runtime::native_vello) runtime_overlays: GpuSurfaceRuntimeOverlays,
}

impl GpuSurfaceInteractionRegion {
    pub(in crate::gui_runtime::native_vello) fn from_gpu_surface(
        primitive_index: usize,
        surface: &PaintGpuSurface,
    ) -> Option<Self> {
        if !surface.rect.has_finite_positive_area() || !surface.content.is_retained_renderable() {
            return None;
        }
        if !surface.capabilities.fast_pointer_move
            && !surface.capabilities.coalesce_vertical_wheel
            && surface
                .capabilities
                .runtime_overlays
                .pointer_vertical_line
                .is_none()
        {
            return None;
        }
        Some(Self {
            primitive_index,
            widget_id: surface.widget_id,
            rect: surface.rect,
            fast_pointer_move: surface.capabilities.fast_pointer_move,
            coalesce_vertical_wheel: surface.capabilities.coalesce_vertical_wheel,
            runtime_overlays: surface.capabilities.runtime_overlays,
        })
    }

    pub(in crate::gui_runtime::native_vello) fn contains(self, point: Point) -> bool {
        point.is_finite() && self.rect.contains(point)
    }
}
