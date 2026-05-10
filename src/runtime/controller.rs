//! Deterministic generic runtime flow for declarative Radiant surfaces.
//!
//! This controller keeps the generic host bridge, projected surface, and
//! layout output together so backends can route normalized widget input without
//! depending on host-specific shell contracts.

mod commands;
mod context;
mod events;
mod focus;
mod pointer;
mod scroll;
mod state;

pub use commands::CommandOutcome;
pub use context::RuntimeContext;
pub use events::Event;

use super::{
    Command, RuntimeBridge, SurfaceFrame, SurfacePaintPlan, SurfaceTraversalIndex, UiSurface,
};
use crate::{
    gui::{
        focus::FocusSurface,
        input::KeyPress,
        types::{Point, Rect, Vector2},
    },
    layout::{
        LayoutDebugOptions, LayoutOutput, LayoutState, NodeId, OverflowPolicy,
        layout_tree_with_state,
    },
    theme::ThemeTokens,
    widgets::{FocusBehavior, WidgetId, WidgetInput, WidgetKey, WidgetState},
};
use std::collections::BTreeMap;

/// Direction for deterministic keyboard focus traversal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FocusTraversal {
    /// Move to the next keyboard-focusable widget in declarative tree order.
    Forward,
    /// Move to the previous keyboard-focusable widget in declarative tree order.
    Backward,
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
    layout_state: LayoutState,
    widget_hit_order: Vec<WidgetId>,
    styled_container_hit_order: Vec<NodeId>,
    scroll_hit_order: Vec<NodeId>,
    widget_clip_ancestors: BTreeMap<WidgetId, Vec<NodeId>>,
    container_clip_ancestors: BTreeMap<NodeId, Vec<NodeId>>,
    scroll_content_by_container: BTreeMap<NodeId, NodeId>,
    focused_widget: Option<WidgetId>,
    pending_key_chord: Option<KeyPress>,
    hovered_container: Option<NodeId>,
    hovered_widget: Option<WidgetId>,
    pointer_capture: Option<WidgetId>,
    pointer_capture_state: Option<(WidgetId, WidgetState)>,
    repaint_requested: bool,
    exit_requested: bool,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Build a generic runtime controller for the provided viewport.
    pub fn new(mut bridge: Bridge, viewport: Vector2) -> Self {
        let viewport = state::normalized_viewport(viewport);
        let surface = bridge.pull_surface();
        let layout_state = LayoutState::default();
        let layout = layout_tree_with_state(
            &surface.layout_node(),
            viewport,
            &layout_state,
            LayoutDebugOptions::default(),
        );
        let traversal = surface.runtime_traversal_index();
        Self {
            bridge,
            viewport,
            surface,
            layout,
            layout_state,
            widget_hit_order: traversal.widget_paint_order,
            styled_container_hit_order: traversal.styled_container_order,
            scroll_hit_order: traversal.scroll_container_order,
            widget_clip_ancestors: traversal.widget_clip_ancestors,
            container_clip_ancestors: traversal.container_clip_ancestors,
            scroll_content_by_container: traversal.scroll_content_by_container,
            focused_widget: None,
            pending_key_chord: None,
            hovered_container: None,
            hovered_widget: None,
            pointer_capture: None,
            pointer_capture_state: None,
            repaint_requested: false,
            exit_requested: false,
        }
    }

    /// Replace the viewport and recompute layout for the current surface.
    pub fn set_viewport(&mut self, viewport: Vector2) {
        self.viewport = state::normalized_viewport(viewport);
        self.relayout();
    }

    /// Reproject the latest host state into a fresh immutable surface snapshot.
    pub fn refresh(&mut self) {
        let mut next_surface = self.bridge.pull_surface();
        let traversal = next_surface.runtime_traversal_index();
        next_surface.synchronize_widget_state_from(&self.surface, &traversal.widget_paint_order);
        self.surface = next_surface;
        self.restore_pointer_capture_state();
        self.relayout_with_traversal(traversal);
        if self
            .focused_widget
            .is_some_and(|widget_id| !self.surface.is_focusable_widget(widget_id))
        {
            self.focused_widget = None;
        }
        if self
            .pointer_capture
            .is_some_and(|widget_id| self.surface.find_widget(widget_id).is_none())
        {
            self.pointer_capture = None;
        }
        if self
            .hovered_widget
            .is_some_and(|widget_id| self.surface.find_widget(widget_id).is_none())
        {
            self.hovered_widget = None;
        }
        if self
            .hovered_container
            .is_some_and(|node_id| !self.styled_container_hit_order.contains(&node_id))
        {
            self.hovered_container = None;
        }
        if let Some(widget_id) = self.focused_widget {
            self.route_focus_changed(widget_id, true);
        }
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
            self.capture_pointer_capture_state(widget_id);
            return self.surface.find_widget(widget_id).is_some();
        };
        self.capture_pointer_capture_state(widget_id);
        if let Some(message) = self.surface.dispatch_widget_output(widget_id, output) {
            self.dispatch_message(message);
        } else {
            self.relayout();
        }
        true
    }
}
