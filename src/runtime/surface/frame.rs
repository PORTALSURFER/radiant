use crate::{
    gui::types::Rect,
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
    /// Prepare one layout plus paint-plan frame for a host-controlled viewport.
    ///
    /// This is the direct embedding path for hosts that already own a platform
    /// surface or render pass and only need Radiant to project widgets into
    /// backend-neutral layout and paint data.
    pub fn frame(&self, viewport: Rect, theme: &ThemeTokens) -> SurfaceFrame {
        let layout = layout_tree(&self.layout_node(), viewport);
        let paint_plan = self.paint_plan(&layout, theme);
        SurfaceFrame {
            viewport,
            layout,
            paint_plan,
        }
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
        let layout =
            layout_tree_with_state(&self.layout_node(), viewport, layout_state, debug_options);
        let paint_plan = self.paint_plan(&layout, theme);
        SurfaceFrame {
            viewport,
            layout,
            paint_plan,
        }
    }
}
