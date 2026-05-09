//! Deterministic generic runtime flow for declarative Radiant surfaces.
//!
//! This controller keeps the generic host bridge, projected surface, and
//! layout output together so backends can route normalized widget input without
//! depending on host-specific shell contracts.

mod commands;
mod events;
mod focus;
mod pointer;
mod scroll;
mod state;

pub use commands::CommandOutcome;
pub use events::Event;

use super::{Command, RuntimeBridge, SurfacePaintPlan, UiSurface};
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
    widgets::{
        FocusBehavior, ScrollbarState, ScrollbarWidget, TextInputState, TextInputWidget, WidgetId,
        WidgetInput, WidgetKey, WidgetState,
    },
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
    layout_state: LayoutState,
    widget_hit_order: Vec<WidgetId>,
    styled_container_hit_order: Vec<NodeId>,
    scroll_hit_order: Vec<NodeId>,
    widget_clip_ancestors: BTreeMap<WidgetId, Vec<NodeId>>,
    container_clip_ancestors: BTreeMap<NodeId, Vec<NodeId>>,
    focused_widget: Option<WidgetId>,
    pending_key_chord: Option<KeyPress>,
    hovered_container: Option<NodeId>,
    hovered_widget: Option<WidgetId>,
    pointer_capture: Option<WidgetId>,
    pointer_capture_state: Option<(WidgetId, WidgetState)>,
    scrollbar_states: BTreeMap<WidgetId, ScrollbarState>,
    text_input_states: BTreeMap<WidgetId, TextInputState>,
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
        let widget_hit_order = surface.widget_paint_order();
        let styled_container_hit_order = surface.styled_container_order();
        let scroll_hit_order = surface.scroll_container_order();
        let widget_clip_ancestors = surface.widget_clip_ancestors();
        let container_clip_ancestors = surface.container_clip_ancestors();
        Self {
            bridge,
            viewport,
            surface,
            layout,
            layout_state,
            widget_hit_order,
            styled_container_hit_order,
            scroll_hit_order,
            widget_clip_ancestors,
            container_clip_ancestors,
            focused_widget: None,
            pending_key_chord: None,
            hovered_container: None,
            hovered_widget: None,
            pointer_capture: None,
            pointer_capture_state: None,
            scrollbar_states: BTreeMap::new(),
            text_input_states: BTreeMap::new(),
            repaint_requested: false,
            exit_requested: false,
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
        self.surface
            .paint_plan_with_hover(&self.layout, theme, self.hovered_container)
    }

    /// Return the current logical viewport size.
    pub fn viewport(&self) -> Vector2 {
        Vector2::new(self.viewport.width(), self.viewport.height())
    }

    /// Return the widget that currently owns keyboard focus.
    pub fn focused_widget(&self) -> Option<WidgetId> {
        self.focused_widget
    }

    /// Return the widget that currently owns pointer capture.
    pub fn pointer_capture(&self) -> Option<WidgetId> {
        self.pointer_capture
    }

    /// Return the widget currently receiving hover state.
    pub fn hovered_widget(&self) -> Option<WidgetId> {
        self.hovered_widget
    }

    /// Return the styled container currently receiving hover chrome.
    pub fn hovered_container(&self) -> Option<NodeId> {
        self.hovered_container
    }

    /// Return whether the host update flow requested another repaint.
    pub fn repaint_requested(&self) -> bool {
        self.repaint_requested
    }

    /// Return and clear the current repaint request flag.
    pub fn take_repaint_requested(&mut self) -> bool {
        let repaint_requested = self.repaint_requested;
        self.repaint_requested = false;
        repaint_requested
    }

    /// Return and clear the current runtime-exit request flag.
    pub fn take_exit_requested(&mut self) -> bool {
        let exit_requested = self.exit_requested;
        self.exit_requested = false;
        exit_requested
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
        self.viewport = state::normalized_viewport(viewport);
        self.relayout();
    }

    /// Reproject the latest host state into a fresh immutable surface snapshot.
    pub fn refresh(&mut self) {
        self.surface = self.bridge.pull_surface();
        self.restore_text_input_states();
        self.restore_scrollbar_states();
        self.restore_pointer_capture_state();
        self.relayout();
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
            self.capture_text_input_state(widget_id);
            self.capture_pointer_capture_state(widget_id);
            return self.surface.find_widget(widget_id).is_some();
        };
        self.capture_text_input_state(widget_id);
        self.capture_pointer_capture_state(widget_id);
        if let Some(message) = self.surface.dispatch_widget_output(widget_id, output) {
            self.dispatch_message(message);
        } else {
            self.relayout();
        }
        true
    }
}
