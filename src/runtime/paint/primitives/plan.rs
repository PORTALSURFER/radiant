use super::PaintSvg;
use super::{
    PaintClipEnd, PaintClipStart, PaintCustomSurface, PaintFillPath, PaintFillPolygon,
    PaintFillRect, PaintGpuSurface, PaintImage, PaintOverlayPanel, PaintStrokePolygon,
    PaintStrokePolyline, PaintStrokeRect, PaintTextInput, PaintTextRun,
};
use crate::{
    gui::types::{Rect, Rgba8, Vector2},
    theme::ThemeTokens,
    widgets::WidgetId,
};
use std::time::Duration;

/// One backend-neutral primitive emitted by a generic surface projection.
#[derive(Clone, Debug, PartialEq)]
pub enum PaintPrimitive {
    /// Begin a rectangular clip.
    ClipStart(PaintClipStart),
    /// End the current clip.
    ClipEnd(PaintClipEnd),
    /// Fill a rectangle.
    FillRect(PaintFillRect),
    /// Fill a bezier path.
    FillPath(PaintFillPath),
    /// Paint a retained SVG document.
    Svg(PaintSvg),
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

/// Frame-local context for lightweight transient overlay painters.
///
/// Transient overlays are painted over the latest cached surface instead of
/// refreshing the declarative surface tree. The context lets hosts anchor
/// overlays to the current paint plan, viewport, and frame time without
/// requiring another surface projection.
#[derive(Clone, Copy, Debug)]
pub struct TransientOverlayContext<'a> {
    /// Latest cached surface paint plan.
    pub plan: &'a SurfacePaintPlan,
    /// Current logical viewport.
    pub viewport: Vector2,
    /// Elapsed animation time supplied by the native runtime.
    pub animation_time: Duration,
}

impl<'a> TransientOverlayContext<'a> {
    /// Build transient overlay context for one presentation frame.
    pub const fn new(
        plan: &'a SurfacePaintPlan,
        viewport: Vector2,
        animation_time: Duration,
    ) -> Self {
        Self {
            plan,
            viewport,
            animation_time,
        }
    }
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
        Self::empty_with_capacity(theme, 0)
    }

    pub(crate) fn empty_with_capacity(theme: &ThemeTokens, primitive_capacity: usize) -> Self {
        Self {
            clear_color: theme.clear_color,
            primitives: Vec::with_capacity(primitive_capacity),
        }
    }

    pub(crate) fn clear_for_theme_with_capacity(
        &mut self,
        theme: &ThemeTokens,
        primitive_capacity: usize,
    ) {
        self.clear_color = theme.clear_color;
        self.primitives.clear();
        if primitive_capacity > self.primitives.capacity() {
            self.primitives.reserve(primitive_capacity);
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

    /// Return the first rectangular paint region emitted by `widget_id`.
    ///
    /// Transient overlays can use this to anchor lightweight frame-time paint
    /// to the cached surface plan without matching individual primitive
    /// variants in their per-frame animation path. This returns the first
    /// rectangle-like primitive for the widget in paint order, which matches
    /// retained GPU surfaces, custom surfaces, images, text, input fields,
    /// overlay panels, and rectangular fills/strokes.
    pub fn first_widget_rect(&self, widget_id: WidgetId) -> Option<Rect> {
        self.primitives.iter().find_map(|primitive| {
            (primitive.widget_id() == Some(widget_id))
                .then(|| primitive.rect())
                .flatten()
        })
    }
}

impl PaintPrimitive {
    /// Return the widget that emitted this primitive when the primitive belongs
    /// to a widget rather than a container clip.
    pub fn widget_id(&self) -> Option<WidgetId> {
        match self {
            Self::ClipStart(_) | Self::ClipEnd(_) => None,
            Self::FillRect(fill) => Some(fill.widget_id),
            Self::FillPath(fill) => Some(fill.widget_id),
            Self::Svg(svg) => Some(svg.widget_id),
            Self::StrokeRect(stroke) => Some(stroke.widget_id),
            Self::FillPolygon(fill) => Some(fill.widget_id),
            Self::StrokePolygon(stroke) => Some(stroke.widget_id),
            Self::StrokePolyline(stroke) => Some(stroke.widget_id),
            Self::Text(text) => Some(text.widget_id),
            Self::OverlayPanel(panel) => Some(panel.widget_id),
            Self::TextInput(input) => Some(input.widget_id),
            Self::Image(image) => Some(image.widget_id),
            Self::GpuSurface(surface) => Some(surface.widget_id),
            Self::CustomSurface(surface) => Some(surface.widget_id),
        }
    }

    /// Return the rectangular region directly carried by this primitive.
    ///
    /// Vector paths and point-list primitives can still be inspected through
    /// their own fields. This helper intentionally stays allocation-free and
    /// covers the rectangle-bearing primitives used as overlay anchors.
    pub fn rect(&self) -> Option<Rect> {
        match self {
            Self::ClipStart(clip) => Some(clip.rect),
            Self::ClipEnd(_) => None,
            Self::FillRect(fill) => Some(fill.rect),
            Self::FillPath(_) => None,
            Self::Svg(svg) => Some(svg.rect),
            Self::StrokeRect(stroke) => Some(stroke.rect),
            Self::FillPolygon(_) | Self::StrokePolygon(_) | Self::StrokePolyline(_) => None,
            Self::Text(text) => Some(text.rect),
            Self::OverlayPanel(panel) => Some(panel.rect),
            Self::TextInput(input) => Some(input.rect),
            Self::Image(image) => Some(image.rect),
            Self::GpuSurface(surface) => Some(surface.rect),
            Self::CustomSurface(surface) => Some(surface.rect),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_with_capacity_presizes_primitive_storage() {
        let theme = ThemeTokens::default();
        let plan = SurfacePaintPlan::empty_with_capacity(&theme, 128);

        assert_eq!(plan.clear_color, theme.clear_color);
        assert!(plan.primitives.is_empty());
        assert!(plan.primitives.capacity() >= 128);
    }

    #[test]
    fn clear_for_theme_with_capacity_reuses_primitive_storage() {
        let theme = ThemeTokens::default();
        let mut plan = SurfacePaintPlan::empty_with_capacity(&theme, 128);
        plan.primitives
            .push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: 1,
                rect: Default::default(),
                color: theme.accent_copper,
            }));
        let capacity = plan.primitives.capacity();

        plan.clear_for_theme_with_capacity(&theme, 16);

        assert!(plan.primitives.is_empty());
        assert_eq!(plan.primitives.capacity(), capacity);
    }

    #[test]
    fn clear_for_theme_with_capacity_grows_to_requested_capacity() {
        let theme = ThemeTokens::default();
        let mut plan = SurfacePaintPlan::empty_with_capacity(&theme, 32);

        plan.clear_for_theme_with_capacity(&theme, 96);

        assert!(plan.primitives.capacity() >= 96);
    }

    #[test]
    fn first_widget_rect_returns_first_rectangle_anchor_in_paint_order() {
        let theme = ThemeTokens::default();
        let mut plan = SurfacePaintPlan::empty(&theme);
        plan.primitives
            .push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: 7,
                rect: Rect::from_min_size(Default::default(), Vector2::new(8.0, 9.0)),
                color: theme.accent_mint,
            }));
        plan.primitives
            .push(PaintPrimitive::StrokeRect(PaintStrokeRect {
                widget_id: 7,
                rect: Rect::from_min_size(Default::default(), Vector2::new(10.0, 11.0)),
                color: theme.accent_mint,
                width: 1.0,
            }));

        assert_eq!(
            plan.first_widget_rect(7),
            Some(Rect::from_min_size(
                Default::default(),
                Vector2::new(8.0, 9.0)
            ))
        );
        assert_eq!(plan.first_widget_rect(404), None);
    }

    #[test]
    fn paint_primitive_reports_widget_id_and_rect_for_anchor_primitives() {
        let atlas = crate::gui::types::ImageRgba::new(1, 1, vec![255, 255, 255, 255])
            .expect("valid test atlas");
        let primitive = PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: 42,
            key: 1,
            revision: 0,
            rect: Rect::from_min_size(Default::default(), Vector2::new(64.0, 32.0)),
            content: crate::runtime::GpuSurfaceContent::RgbaAtlas {
                atlas: std::sync::Arc::new(atlas),
                source_rect: Rect::from_min_size(Default::default(), Vector2::new(1.0, 1.0)),
            },
            capabilities: Default::default(),
            overlays: Vec::new(),
        });

        assert_eq!(primitive.widget_id(), Some(42));
        assert_eq!(
            primitive.rect(),
            Some(Rect::from_min_size(
                Default::default(),
                Vector2::new(64.0, 32.0)
            ))
        );
    }
}
