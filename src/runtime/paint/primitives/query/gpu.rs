use super::super::{PaintGpuSurface, PaintPrimitive, SurfacePaintPlan};

impl SurfacePaintPlan {
    /// Iterate over retained GPU surface primitives in paint order.
    pub fn gpu_surfaces(&self) -> impl Iterator<Item = &PaintGpuSurface> {
        self.primitives
            .iter()
            .filter_map(PaintPrimitive::gpu_surface)
    }
}
