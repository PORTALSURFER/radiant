//! Reusable custom canvas primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, SurfaceNode, WidgetMessageMapper};
use crate::theme::ThemeTokens;

use super::support::{WidgetCommon, push_canvas_widget_paint};
use crate::widgets::contract::{
    FocusBehavior, PaintBounds, Widget, WidgetId, WidgetProminence, WidgetSizing, WidgetStyle,
    WidgetTone,
};
use crate::widgets::interaction::{CanvasMessage, WidgetInput, WidgetOutput};

/// Public canvas/custom-paint primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Optional retained-surface metadata supplied by the host.
    pub retained: Option<RetainedSurfaceDescriptor>,
}

/// Product-neutral metadata for a host-retained custom surface.
///
/// The descriptor lets a host attach stable cache identity, revision, and dirty
/// mask information to a canvas without moving product model state into
/// Radiant. Native backends can use this to avoid unnecessary full-surface
/// recomputation while still treating the actual retained paint as host-owned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RetainedSurfaceDescriptor {
    /// Stable host-defined retained surface key.
    pub key: u64,
    /// Monotonic host-defined revision for this retained surface.
    pub revision: u64,
    /// Host-defined dirty segment bitmask for the latest projection.
    pub dirty_mask: u64,
    /// Whether the host-retained surface has dynamic paint that must be
    /// re-rendered whenever the runtime is asked to repaint it.
    pub volatile: bool,
}

impl CanvasWidget {
    /// Build a canvas descriptor for custom paint and routed pointer/keyboard input.
    pub fn new(id: WidgetId, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Keyboard;
        common.paint.bounds = PaintBounds::AllowOverflow;
        common.paint.paints_state_layers = false;
        common.style = WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Subtle,
        };
        Self {
            common,
            retained: None,
        }
    }

    /// Attach retained-surface metadata to this custom canvas.
    pub fn with_retained_surface(mut self, descriptor: RetainedSurfaceDescriptor) -> Self {
        self.retained = Some(descriptor);
        self
    }

    /// Route one backend-neutral interaction into the custom surface.
    pub fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<CanvasMessage> {
        (!self.common.state.disabled).then_some(CanvasMessage::Input { input })
    }
}

impl Widget for CanvasWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        CanvasWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        push_canvas_widget_paint(primitives, self, bounds);
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Build a canvas-message mapper.
    pub fn canvas(map: impl Fn(CanvasMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a non-emitting canvas leaf node for custom paint or routed input surfaces.
    pub fn canvas(id: WidgetId, sizing: WidgetSizing) -> Self {
        Self::static_widget(CanvasWidget::new(id, sizing))
    }

    /// Build a canvas leaf node with a custom widget-to-host message mapper.
    pub fn canvas_mapped(
        id: WidgetId,
        sizing: WidgetSizing,
        map: impl Fn(CanvasMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            CanvasWidget::new(id, sizing),
            WidgetMessageMapper::canvas(map),
        )
    }

    /// Build a custom canvas with retained-surface metadata and a host-message mapper.
    pub fn retained_canvas_mapped(
        id: WidgetId,
        sizing: WidgetSizing,
        retained: RetainedSurfaceDescriptor,
        map: impl Fn(CanvasMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            CanvasWidget::new(id, sizing).with_retained_surface(retained),
            WidgetMessageMapper::canvas(map),
        )
    }
}
