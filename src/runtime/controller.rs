//! Deterministic generic runtime flow for declarative Radiant surfaces.
//!
//! This controller keeps the generic host bridge, projected surface, and
//! layout output together so backends can route normalized widget input without
//! depending on host-specific shell contracts.

use super::{RuntimeBridge, SurfacePaintPlan, UiSurface};
use crate::{
    gui::types::{Point, Rect, Vector2},
    layout::{LayoutOutput, layout_tree},
    theme::ThemeTokens,
    widgets::{WidgetId, WidgetInput},
};

/// Direction for deterministic keyboard focus traversal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FocusTraversal {
    /// Move to the next keyboard-focusable widget in declarative tree order.
    Forward,
    /// Move to the previous keyboard-focusable widget in declarative tree order.
    Backward,
}

/// Borrowed runtime context for one projected Radiant surface.
///
/// This context exposes the current viewport, immutable view tree, and resolved
/// layout without giving renderers or host code ownership of the runtime
/// controller. Style remains an explicit argument to paint-plan generation so
/// hosts can swap themes without rebuilding runtime state.
pub struct RuntimeContext<'a, Message> {
    /// Current logical viewport rectangle.
    pub viewport: Rect,
    /// Current immutable declarative view snapshot.
    pub surface: &'a UiSurface<Message>,
    /// Current resolved layout output for the surface.
    pub layout: &'a LayoutOutput,
}

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
    focused_widget: Option<WidgetId>,
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
            focused_widget: None,
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

    /// Return a borrowed context view of the current runtime state.
    pub fn context(&self) -> RuntimeContext<'_, Message> {
        RuntimeContext {
            viewport: self.viewport,
            surface: &self.surface,
            layout: &self.layout,
        }
    }

    /// Project the current surface and layout into backend-neutral paint data.
    pub fn paint_plan(&self, theme: &ThemeTokens) -> SurfacePaintPlan {
        self.surface.paint_plan(&self.layout, theme)
    }

    /// Return the current logical viewport size.
    pub fn viewport(&self) -> Vector2 {
        Vector2::new(self.viewport.width(), self.viewport.height())
    }

    /// Return the widget that currently owns keyboard focus.
    pub fn focused_widget(&self) -> Option<WidgetId> {
        self.focused_widget
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
        if self
            .focused_widget
            .is_some_and(|widget_id| !self.surface.is_focusable_widget(widget_id))
        {
            self.focused_widget = None;
        }
        if let Some(widget_id) = self.focused_widget {
            self.route_focus_changed(widget_id, true);
        }
    }

    /// Give keyboard focus to one focusable widget.
    ///
    /// Returns `false` when the widget is absent or does not participate in
    /// focus. Focus changes are routed into affected widgets so their retained
    /// interaction state can update before the next paint plan.
    pub fn focus_widget(&mut self, widget_id: WidgetId) -> bool {
        if !self.surface.is_focusable_widget(widget_id) {
            return false;
        }
        if self.focused_widget == Some(widget_id) {
            return true;
        }

        if let Some(previous) = self.focused_widget {
            self.route_focus_changed(previous, false);
        }
        self.focused_widget = Some(widget_id);
        self.route_focus_changed(widget_id, true);
        true
    }

    /// Clear keyboard focus when a surface or backend loses focus ownership.
    pub fn clear_focus(&mut self) {
        if let Some(previous) = self.focused_widget.take() {
            self.route_focus_changed(previous, false);
        }
    }

    /// Move keyboard focus through the current declarative tree.
    ///
    /// Traversal uses stable tree order and wraps at either end. Returns the new
    /// focus target, or `None` when no keyboard-focusable widgets are projected.
    pub fn traverse_focus(&mut self, direction: FocusTraversal) -> Option<WidgetId> {
        let order = self.surface.keyboard_focus_order();
        let next = next_focus_target(self.focused_widget, &order, direction)?;
        self.focus_widget(next).then_some(next)
    }

    /// Route a keyboard interaction to the current focus target.
    ///
    /// Pointer events should continue to use [`SurfaceRuntime::dispatch_input_at`]
    /// or [`SurfaceRuntime::dispatch_input`], because they carry their own hit
    /// target. Keyboard events are resolved through focused widget identity.
    pub fn dispatch_focused_input(&mut self, input: WidgetInput) -> Option<WidgetId> {
        let widget_id = self.focused_widget?;
        self.dispatch_input(widget_id, input).then_some(widget_id)
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
        if matches!(input, WidgetInput::PointerPress { .. }) {
            let _ = self.focus_widget(widget_id);
        }
        self.dispatch_input(widget_id, input).then_some(widget_id)
    }

    fn relayout(&mut self) {
        self.layout = layout_tree(&self.surface.layout_node(), self.viewport);
    }

    fn route_focus_changed(&mut self, widget_id: WidgetId, focused: bool) {
        let Some(bounds) = self.layout.rects.get(&widget_id).copied() else {
            return;
        };
        let _ = self.surface.dispatch_widget_input(
            widget_id,
            bounds,
            WidgetInput::FocusChanged(focused),
        );
    }
}

fn normalized_viewport(viewport: Vector2) -> Rect {
    Rect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(viewport.x.max(1.0), viewport.y.max(1.0)),
    )
}

fn next_focus_target(
    current: Option<WidgetId>,
    order: &[WidgetId],
    direction: FocusTraversal,
) -> Option<WidgetId> {
    if order.is_empty() {
        return None;
    }
    let current_index = current.and_then(|widget_id| order.iter().position(|id| *id == widget_id));
    let next_index = match (current_index, direction) {
        (Some(index), FocusTraversal::Forward) => (index + 1) % order.len(),
        (Some(0), FocusTraversal::Backward) => order.len() - 1,
        (Some(index), FocusTraversal::Backward) => index - 1,
        (None, FocusTraversal::Forward) => 0,
        (None, FocusTraversal::Backward) => order.len() - 1,
    };
    Some(order[next_index])
}
