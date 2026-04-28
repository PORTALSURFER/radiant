//! Deterministic generic runtime loop for declarative Radiant surfaces.
//!
//! This controller keeps the generic host bridge, projected surface, and
//! layout output together so backends can route normalized widget input without
//! depending on the legacy Sempal shell contract.

use super::{RuntimeBridge, SurfacePaintPlan, UiSurface};
use crate::{
    gui::types::{Point, Rect, Vector2},
    layout::{LayoutOutput, layout_tree},
    theme::ThemeTokens,
    widgets::{WidgetId, WidgetInput},
};

/// Stateful generic runtime controller for message-driven Radiant hosts.
///
/// The controller preserves one-way data flow:
/// 1. project an immutable [`UiSurface`] from host state
/// 2. run public layout on that surface
/// 3. route backend-neutral [`WidgetInput`] into a widget
/// 4. map widget output into a host-defined message
/// 5. reduce that message into host state
/// 6. project the next immutable surface snapshot
pub struct SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    bridge: Bridge,
    viewport: Rect,
    surface: UiSurface<Message>,
    layout: LayoutOutput,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Build a generic runtime controller for the provided viewport.
    pub fn new(mut bridge: Bridge, viewport: Vector2) -> Self {
        let viewport = normalized_viewport(viewport);
        let surface = bridge.pull_surface();
        let layout = layout_tree(&surface.layout_node(), viewport);
        Self {
            bridge,
            viewport,
            surface,
            layout,
        }
    }

    /// Return the current projected surface snapshot.
    pub fn surface(&self) -> &UiSurface<Message> {
        &self.surface
    }

    /// Return the current layout output for the projected surface.
    pub fn layout(&self) -> &LayoutOutput {
        &self.layout
    }

    /// Project the current surface and layout into backend-neutral paint data.
    pub fn paint_plan(&self, theme: &ThemeTokens) -> SurfacePaintPlan {
        self.surface.paint_plan(&self.layout, theme)
    }

    /// Return the current logical viewport size.
    pub fn viewport(&self) -> Vector2 {
        Vector2::new(self.viewport.width(), self.viewport.height())
    }

    /// Return an immutable reference to the owned bridge.
    pub fn bridge(&self) -> &Bridge {
        &self.bridge
    }

    /// Return a mutable reference to the owned bridge.
    pub fn bridge_mut(&mut self) -> &mut Bridge {
        &mut self.bridge
    }

    /// Consume the runtime controller and return the owned bridge.
    pub fn into_bridge(self) -> Bridge {
        self.bridge
    }

    /// Replace the viewport and recompute layout for the current surface.
    pub fn set_viewport(&mut self, viewport: Vector2) {
        self.viewport = normalized_viewport(viewport);
        self.relayout();
    }

    /// Reproject the latest host state into a fresh immutable surface snapshot.
    pub fn refresh(&mut self) {
        self.surface = self.bridge.pull_surface();
        self.relayout();
    }

    /// Route one normalized widget interaction by widget id.
    ///
    /// Returns `true` when the interaction targeted a projected widget, even if
    /// that interaction did not emit a host-defined message.
    pub fn dispatch_input(&mut self, widget_id: WidgetId, input: WidgetInput) -> bool {
        let Some(bounds) = self.layout.rects.get(&widget_id).copied() else {
            return false;
        };
        let Some(output) = self.surface.dispatch_widget_input(widget_id, bounds, input) else {
            return self.surface.find_widget(widget_id).is_some();
        };
        if let Some(message) = self.surface.dispatch_widget_output(widget_id, output) {
            self.bridge.reduce_message(message);
            self.refresh();
        } else {
            self.relayout();
        }
        true
    }

    /// Return the first projected widget whose laid-out bounds contain `point`.
    pub fn widget_at(&self, point: Point) -> Option<WidgetId> {
        self.layout
            .rects
            .iter()
            .filter(|(node_id, rect)| {
                rect.contains(point) && self.surface.find_widget(**node_id).is_some()
            })
            .map(|(node_id, rect)| (*node_id, rect.width() * rect.height()))
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(node_id, _)| node_id)
    }

    /// Route one normalized widget interaction by point hit test.
    ///
    /// Returns the targeted widget id when a projected widget handled the point.
    pub fn dispatch_input_at(&mut self, point: Point, input: WidgetInput) -> Option<WidgetId> {
        let widget_id = self.widget_at(point)?;
        self.dispatch_input(widget_id, input).then_some(widget_id)
    }

    fn relayout(&mut self) {
        self.layout = layout_tree(&self.surface.layout_node(), self.viewport);
    }
}

fn normalized_viewport(viewport: Vector2) -> Rect {
    Rect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(viewport.x.max(1.0), viewport.y.max(1.0)),
    )
}
