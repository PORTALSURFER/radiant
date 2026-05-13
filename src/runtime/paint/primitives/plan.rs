use super::PaintSvg;
use super::{
    PaintClipEnd, PaintClipStart, PaintCustomSurface, PaintFillPath, PaintFillPolygon,
    PaintFillRect, PaintGpuSurface, PaintImage, PaintOverlayPanel, PaintStrokePolygon,
    PaintStrokePolyline, PaintStrokeRect, PaintTextInput, PaintTextRun,
};
use crate::{
    gui::types::{Rgba8, Vector2},
    theme::ThemeTokens,
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
}
