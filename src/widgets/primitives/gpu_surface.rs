//! Reusable retained GPU surface primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{
    GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceLineStyle, GpuSurfaceOverlay,
    GpuSurfaceRuntimeOverlays, PaintPrimitive, SurfaceNode,
};
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{
    FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing, WidgetStyle,
};
use crate::widgets::interaction::{GpuSurfaceMessage, WidgetInput, WidgetOutput};

mod paint;

/// Named construction inputs for a retained GPU surface widget.
///
/// This keeps the retained resource identity (`key`) and content generation
/// (`revision`) readable at call sites that construct GPU-heavy leaves.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuSurfaceParts {
    /// Stable widget id used by layout, paint, and input routing.
    pub id: WidgetId,
    /// Desired layout sizing for the retained GPU surface slot.
    pub sizing: WidgetSizing,
    /// Stable surface key used by native backends for retained GPU resources.
    pub key: u64,
    /// Monotonic content revision for retained GPU resources.
    pub revision: u64,
    /// Backend-neutral retained GPU content.
    pub content: GpuSurfaceContent,
}

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
    /// Runtime interaction capabilities requested by this GPU surface.
    pub capabilities: GpuSurfaceCapabilities,
    /// Optional lightweight overlays composited by the native GPU backend.
    pub overlays: Vec<GpuSurfaceOverlay>,
    /// Whether routed widget input should be emitted as host-mappable output.
    pub emits_input: bool,
}

impl GpuSurfaceWidget {
    /// Build a retained GPU surface widget from named construction inputs.
    pub fn from_parts(parts: GpuSurfaceParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        common.style = WidgetStyle::default();
        Self {
            common,
            key: parts.key,
            revision: parts.revision,
            content: parts.content,
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
            emits_input: false,
        }
    }

    /// Build a retained GPU surface widget.
    pub fn new(
        id: WidgetId,
        sizing: WidgetSizing,
        key: u64,
        revision: u64,
        content: GpuSurfaceContent,
    ) -> Self {
        Self::from_parts(GpuSurfaceParts {
            id,
            sizing,
            key,
            revision,
            content,
        })
    }

    /// Replace the runtime interaction capabilities for this retained GPU surface.
    pub fn with_capabilities(mut self, capabilities: GpuSurfaceCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Enable runtime-owned pointer-following vertical line updates for this retained GPU surface.
    pub fn with_runtime_pointer_line(mut self, line: GpuSurfaceLineStyle) -> Self {
        self.capabilities.runtime_overlays = GpuSurfaceRuntimeOverlays::pointer_vertical_line(line);
        self
    }

    /// Enable fast pointer-motion routing for this retained GPU surface.
    pub fn with_fast_pointer_move(mut self, enabled: bool) -> Self {
        self.capabilities.fast_pointer_move = enabled;
        self
    }

    /// Enable coalesced vertical wheel routing for this retained GPU surface.
    pub fn with_coalesced_vertical_wheel(mut self, enabled: bool) -> Self {
        self.capabilities.coalesce_vertical_wheel = enabled;
        self
    }

    /// Replace the lightweight overlays for this retained GPU surface.
    pub fn with_overlays(mut self, overlays: Vec<GpuSurfaceOverlay>) -> Self {
        self.overlays = overlays;
        self
    }

    /// Enable or disable host-mappable input events for this GPU surface.
    pub fn with_input_events(mut self, enabled: bool) -> Self {
        self.emits_input = enabled;
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

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        self.emits_input
            .then(|| WidgetOutput::typed(GpuSurfaceMessage::Input { input }))
    }

    fn accepts_wheel_input(&self) -> bool {
        self.emits_input || self.capabilities.coalesce_vertical_wheel
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        paint::push_gpu_surface_widget_paint(primitives, self, bounds);
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
        Self::static_widget(GpuSurfaceWidget::from_parts(GpuSurfaceParts {
            id,
            sizing,
            key,
            revision,
            content,
        }))
    }
}
