use super::{PaintPrimitive, SurfacePaintPlan};

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
                | PaintPrimitive::FillPath(_)
                | PaintPrimitive::FillPolygon(_) => stats.fills += 1,
                PaintPrimitive::Svg(_) => stats.svg_documents += 1,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::{Point, Rect, Vector2},
        runtime::{PaintFillRect, PaintImage, PaintText, PaintTextAlign, PaintTextRun},
        theme::ThemeTokens,
        widgets::TextWrap,
    };
    use std::sync::Arc;

    #[test]
    fn surface_paint_plan_stats_count_core_primitive_groups() {
        let theme = ThemeTokens::default();
        let image = crate::gui::types::ImageRgba::new(1, 1, vec![255, 255, 255, 255])
            .expect("valid test image");
        let mut plan = SurfacePaintPlan::empty(&theme);
        plan.primitives
            .push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: 1,
                rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(4.0, 4.0)),
                color: theme.accent_mint,
            }));
        plan.primitives.push(PaintPrimitive::Text(PaintTextRun {
            widget_id: 2,
            text: PaintText::from("ready"),
            rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(24.0, 12.0)),
            font_size: 12.0,
            baseline: None,
            color: theme.text_primary,
            align: PaintTextAlign::Left,
            wrap: TextWrap::None,
        }));
        plan.primitives.push(PaintPrimitive::Image(PaintImage {
            widget_id: 3,
            source_rect: None,
            rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(8.0, 8.0)),
            image: Arc::new(image),
        }));

        let stats = plan.stats();

        assert_eq!(stats.total, 3);
        assert_eq!(stats.fills, 1);
        assert_eq!(stats.text, 1);
        assert_eq!(stats.images, 1);
    }
}
