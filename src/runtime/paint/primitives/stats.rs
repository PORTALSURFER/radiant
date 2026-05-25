use super::{PaintPrimitive, SurfacePaintPlan};

#[cfg(test)]
#[path = "stats/tests.rs"]
mod tests;

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
    /// Retained SVG document primitives.
    pub svg_documents: usize,
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

impl SurfacePaintPlan {
    /// Count primitive categories in this paint plan.
    pub fn stats(&self) -> SurfacePaintStats {
        let mut stats = SurfacePaintStats {
            total: self.primitives.len(),
            ..SurfacePaintStats::default()
        };
        for primitive in &self.primitives {
            match primitive {
                PaintPrimitive::ClipStart(_) | PaintPrimitive::ClipEnd(_) => stats.clips += 1,
                PaintPrimitive::FillRect(_)
                | PaintPrimitive::FillRectBatch(_)
                | PaintPrimitive::FillPath(_)
                | PaintPrimitive::FillPolygon(_) => stats.fills += 1,
                PaintPrimitive::Svg(_) => stats.svg_documents += 1,
                PaintPrimitive::StrokeRect(_)
                | PaintPrimitive::StrokeRectBatch(_)
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
