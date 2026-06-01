use crate::{
    gui::types::{Point, Rect},
    layout::{LayoutDebugOptions, LayoutOutput, LayoutState, layout_tree, layout_tree_with_state},
    runtime::SurfacePaintPlan,
    theme::ThemeTokens,
};

use super::UiSurface;

/// One host-controlled rendering frame prepared from a declarative surface.
///
/// `SurfaceFrame` packages the logical viewport, resolved layout, and
/// backend-neutral paint plan that a host renderer needs to draw a projected
/// [`UiSurface`]. It is intended for embedded or custom-host integrations that
/// own the surrounding window, native surface, or render pass.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFrame {
    /// Logical viewport rectangle supplied by the host.
    pub viewport: Rect,
    /// Resolved layout for the projected surface.
    pub layout: LayoutOutput,
    /// Backend-neutral paint plan for the resolved layout.
    pub paint_plan: SurfacePaintPlan,
}

impl<Message> UiSurface<Message> {
    /// Resolve this surface into layout rectangles for a host-controlled viewport.
    ///
    /// This is the layout-only counterpart to [`Self::frame`] for hosts that
    /// project declarative Radiant surfaces into an existing renderer or
    /// compatibility layer and only need geometry.
    pub fn layout(&self, viewport: Rect) -> LayoutOutput {
        layout_tree(&self.layout_node(), viewport)
    }

    /// Resolve this surface into layout rectangles for an origin-based viewport.
    ///
    /// This is a convenience for tests, automation, plugin previews, and
    /// embedded hosts that render a surface into a logical size rather than an
    /// already-positioned viewport rectangle.
    pub fn layout_at_size(&self, size: crate::layout::Vector2) -> LayoutOutput {
        self.layout(Rect::from_min_size(Point::default(), size))
    }

    /// Resolve this surface into layout rectangles with explicit state/options.
    ///
    /// Use this variant when a host needs scroll offsets, virtualization state,
    /// or debug primitives/diagnostics without also building a paint plan.
    pub fn layout_with_options(
        &self,
        viewport: Rect,
        layout_state: &LayoutState,
        debug_options: LayoutDebugOptions,
    ) -> LayoutOutput {
        layout_tree_with_state(&self.layout_node(), viewport, layout_state, debug_options)
    }

    /// Prepare one layout plus paint-plan frame for a host-controlled viewport.
    ///
    /// This is the direct embedding path for hosts that already own a platform
    /// surface or render pass and only need Radiant to project widgets into
    /// backend-neutral layout and paint data.
    pub fn frame(&self, viewport: Rect, theme: &ThemeTokens) -> SurfaceFrame {
        let layout = self.layout(viewport);
        let paint_plan = self.paint_plan(&layout, theme);
        SurfaceFrame {
            viewport,
            layout,
            paint_plan,
        }
    }

    /// Prepare one frame for an origin-based viewport.
    ///
    /// Use this when a host or test only cares about rendering into a logical
    /// size and does not need to supply a non-zero viewport origin.
    pub fn frame_at_size(&self, size: crate::layout::Vector2, theme: &ThemeTokens) -> SurfaceFrame {
        self.frame(Rect::from_min_size(Point::default(), size), theme)
    }

    /// Prepare one frame with Radiant's default theme.
    ///
    /// This is intended for smoke tests, automation, examples, and embedded
    /// previews where custom theme tokens are not part of the behavior under
    /// test.
    pub fn frame_with_default_theme(&self, viewport: Rect) -> SurfaceFrame {
        self.frame(viewport, &ThemeTokens::default())
    }

    /// Prepare one origin-based frame with Radiant's default theme.
    ///
    /// This combines [`Self::frame_at_size`] with the default theme for common
    /// GUI smoke tests and examples.
    pub fn frame_at_size_with_default_theme(&self, size: crate::layout::Vector2) -> SurfaceFrame {
        self.frame_at_size(size, &ThemeTokens::default())
    }

    /// Prepare one host-controlled frame with explicit layout state and diagnostics.
    ///
    /// Use this variant when a host needs scroll offsets, virtualization state,
    /// or debug primitives/diagnostics in the returned layout output.
    pub fn frame_with_layout_options(
        &self,
        viewport: Rect,
        theme: &ThemeTokens,
        layout_state: &LayoutState,
        debug_options: LayoutDebugOptions,
    ) -> SurfaceFrame {
        let layout = self.layout_with_options(viewport, layout_state, debug_options);
        let paint_plan = self.paint_plan(&layout, theme);
        SurfaceFrame {
            viewport,
            layout,
            paint_plan,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        layout::Vector2,
        prelude::{IntoView, text},
        runtime::UiSurface,
    };

    #[test]
    fn frame_at_size_uses_origin_viewport() {
        let theme = crate::theme::ThemeTokens::default();
        let frame = UiSurface::<()>::new(text("Status").into_node())
            .frame_at_size(Vector2::new(120.0, 40.0), &theme);

        assert_eq!(frame.viewport.min, crate::gui::types::Point::default());
        assert_eq!(frame.viewport.width(), 120.0);
        assert_eq!(frame.viewport.height(), 40.0);
        assert!(frame.paint_plan.contains_text("Status"));
    }

    #[test]
    fn frame_at_size_with_default_theme_builds_paint_plan() {
        let frame = UiSurface::<()>::new(text("Ready").into_node())
            .frame_at_size_with_default_theme(Vector2::new(120.0, 40.0));

        assert_eq!(frame.viewport.min, crate::gui::types::Point::default());
        assert!(frame.paint_plan.contains_text("Ready"));
    }
}
