//! Reusable retained GPU surface primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{
    GpuSurfaceContent, GpuSurfaceOverlay, PaintGpuSurface, PaintPrimitive, SurfaceNode,
};
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{
    FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing, WidgetStyle,
};
use crate::widgets::interaction::{WidgetInput, WidgetOutput};

/// Reusable widget that reserves a retained native GPU surface.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuSurfaceWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Stable surface key used by native backends for retained GPU resources.
    pub key: u64,
    /// Monotonic content revision for retained GPU resources.
    pub revision: u64,
    /// Backend-neutral retained GPU content.
    pub content: GpuSurfaceContent,
    /// Optional lightweight overlays composited by the native GPU backend.
    pub overlays: Vec<GpuSurfaceOverlay>,
}

impl GpuSurfaceWidget {
    /// Build a retained GPU surface widget.
    pub fn new(
        id: WidgetId,
        sizing: WidgetSizing,
        key: u64,
        revision: u64,
        content: GpuSurfaceContent,
    ) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        common.style = WidgetStyle::default();
        Self {
            common,
            key,
            revision,
            content,
            overlays: Vec::new(),
        }
    }

    /// Replace the lightweight overlays for this retained GPU surface.
    pub fn with_overlays(mut self, overlays: Vec<GpuSurfaceOverlay>) -> Self {
        self.overlays = overlays;
        self
    }
}

impl Widget for GpuSurfaceWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        primitives.push(PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: self.common.id,
            key: self.key,
            revision: self.revision,
            rect: bounds,
            content: self.content.clone(),
            overlays: self.overlays.clone(),
        }));
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a non-emitting retained GPU surface leaf node.
    pub fn gpu_surface(
        id: WidgetId,
        sizing: WidgetSizing,
        key: u64,
        revision: u64,
        content: GpuSurfaceContent,
    ) -> Self {
        Self::static_widget(GpuSurfaceWidget::new(id, sizing, key, revision, content))
    }
}
