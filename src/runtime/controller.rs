//! Deterministic generic runtime flow for declarative Radiant surfaces.
//!
//! This controller keeps the generic host bridge, projected surface, and
//! layout output together so backends can route normalized widget input without
//! depending on host-specific shell contracts.

mod commands;
mod context;
mod events;
mod focus;
mod hit_order;
mod input;
mod pointer;
mod scratch;
mod scroll;
mod state;

pub use commands::CommandOutcome;
pub use context::{RuntimeContext, RuntimeSurfaceFrame};
pub use events::Event;
pub use scroll::ScrollUpdate;

use super::{
    ClipAncestors, Command, RuntimeBridge, SurfaceFrame, SurfacePaintPlan,
    SurfaceRuntimeProjection, SurfaceTraversalIndex, SurfaceWidget, UiSurface,
    WidgetDispatchResult, WidgetPath, estimated_paint_primitive_capacity,
};
use crate::{
    gui::{
        focus::FocusSurface,
        input::KeyPress,
        types::{Point, Rect, Vector2},
    },
    layout::{LayoutDebugOptions, LayoutEngine, LayoutOutput, LayoutState, NodeId, OverflowPolicy},
    theme::ThemeTokens,
    widgets::{WidgetId, WidgetInput, WidgetKey, WidgetState},
};
use hit_order::HitOrderIndex;
use scratch::RuntimeScratch;
use std::collections::{HashMap, HashSet};

/// Direction for deterministic keyboard focus traversal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FocusTraversal {
    /// Move to the next keyboard-focusable widget in declarative tree order.
    Forward,
    /// Move to the previous keyboard-focusable widget in declarative tree order.
    Backward,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ScrollDragCapture {
    node_id: NodeId,
    grip_fraction: f32,
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
    layout_root: crate::layout::LayoutNode,
    layout_engine: LayoutEngine,
    layout: LayoutOutput,
    layout_state: LayoutState,
    layout_debug_options: LayoutDebugOptions,
    widget_hit_order: Vec<WidgetId>,
    focusable_widgets: HitOrderIndex,
    pointer_widgets: HitOrderIndex,
    widget_paths: HashMap<WidgetId, WidgetPath>,
    previous_widget_paths: HashMap<WidgetId, WidgetPath>,
    container_hover_suppression: HashSet<WidgetId>,
    keyboard_focus_widgets: HitOrderIndex,
    wheel_widgets: HitOrderIndex,
    stateful_widget_order: Vec<WidgetId>,
    styled_containers: HitOrderIndex,
    scroll_containers: HitOrderIndex,
    widget_clip_ancestors: HashMap<WidgetId, ClipAncestors>,
    container_clip_ancestors: HashMap<NodeId, ClipAncestors>,
    scroll_content_by_container: HashMap<NodeId, NodeId>,
    scratch: RuntimeScratch,
    focused_widget: Option<WidgetId>,
    pending_key_chord: Option<KeyPress>,
    hovered_container: Option<NodeId>,
    hovered_widget: Option<WidgetId>,
    pointer_capture: Option<WidgetId>,
    pointer_capture_state: Option<(WidgetId, WidgetState)>,
    hovered_scroll_affordance: Option<NodeId>,
    scroll_drag_capture: Option<ScrollDragCapture>,
    repaint_requested: bool,
    exit_requested: bool,
    runtime_commands: Vec<Command<Message>>,
    runtime_command_batch: Vec<Command<Message>>,
    runtime_messages: Vec<Message>,
    runtime_message_batch: Vec<Message>,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Build a generic runtime controller for the provided viewport.
    pub fn new(mut bridge: Bridge, viewport: Vector2) -> Self {
        let viewport = state::normalized_viewport(viewport);
        let surface = bridge.pull_surface();
        let SurfaceRuntimeProjection {
            layout_root,
            traversal,
        } = surface.runtime_projection();
        let mut runtime = Self {
            bridge,
            viewport,
            surface,
            layout_root,
            layout_engine: LayoutEngine::default(),
            layout: LayoutOutput::default(),
            layout_state: LayoutState::default(),
            layout_debug_options: LayoutDebugOptions::default(),
            widget_hit_order: Vec::new(),
            focusable_widgets: HitOrderIndex::default(),
            pointer_widgets: HitOrderIndex::default(),
            widget_paths: HashMap::new(),
            previous_widget_paths: HashMap::new(),
            container_hover_suppression: HashSet::new(),
            keyboard_focus_widgets: HitOrderIndex::default(),
            wheel_widgets: HitOrderIndex::default(),
            stateful_widget_order: Vec::new(),
            styled_containers: HitOrderIndex::default(),
            scroll_containers: HitOrderIndex::default(),
            widget_clip_ancestors: HashMap::new(),
            container_clip_ancestors: HashMap::new(),
            scroll_content_by_container: HashMap::new(),
            scratch: RuntimeScratch::default(),
            focused_widget: None,
            pending_key_chord: None,
            hovered_container: None,
            hovered_widget: None,
            pointer_capture: None,
            pointer_capture_state: None,
            hovered_scroll_affordance: None,
            scroll_drag_capture: None,
            repaint_requested: false,
            exit_requested: false,
            runtime_commands: Vec::new(),
            runtime_command_batch: Vec::new(),
            runtime_messages: Vec::new(),
            runtime_message_batch: Vec::new(),
        };
        runtime.relayout_with_traversal(traversal);
        runtime
    }

    /// Replace the viewport and recompute layout for the current surface.
    pub fn set_viewport(&mut self, viewport: Vector2) {
        self.viewport = state::normalized_viewport(viewport);
        self.relayout_current_surface();
    }

    /// Reproject the latest host state into a fresh immutable surface snapshot.
    pub fn refresh(&mut self) {
        let mut next_surface = self.bridge.pull_surface();
        std::mem::swap(&mut self.previous_widget_paths, &mut self.widget_paths);
        let mut traversal = self.take_reusable_traversal_index(true);
        let layout_root = next_surface.runtime_projection_reusing_with_scratch(
            &mut traversal,
            &mut self.scratch.projection_scroll_stack,
            &mut self.scratch.projection_child_path,
        );
        next_surface.synchronize_widget_state_from_paths(
            &self.surface,
            &traversal.stateful_widget_order,
            &traversal.widget_paths,
            &self.previous_widget_paths,
        );
        self.surface = next_surface;
        self.layout_root = layout_root;
        self.restore_pointer_capture_state();
        self.relayout_with_traversal(traversal);
        if self
            .focused_widget
            .is_some_and(|widget_id| !self.focusable_widgets.contains(widget_id))
        {
            self.focused_widget = None;
        }
        if self
            .pointer_capture
            .is_some_and(|widget_id| !self.widget_paths.contains_key(&widget_id))
        {
            self.pointer_capture = None;
        }
        if self
            .scroll_drag_capture
            .is_some_and(|capture| !self.scroll_containers.contains(capture.node_id))
        {
            self.scroll_drag_capture = None;
        }
        if self
            .hovered_scroll_affordance
            .is_some_and(|node_id| !self.scroll_containers.contains(node_id))
        {
            self.hovered_scroll_affordance = None;
        }
        if self
            .hovered_widget
            .is_some_and(|widget_id| !self.widget_paths.contains_key(&widget_id))
        {
            self.hovered_widget = None;
        }
        if self
            .hovered_container
            .is_some_and(|node_id| !self.styled_containers.contains(node_id))
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
        let Some(result) = self.dispatch_surface_input(widget_id, bounds, input) else {
            return false;
        };
        self.capture_pointer_capture_state(widget_id);
        match result {
            WidgetDispatchResult::Message(message) => {
                self.dispatch_message(message);
            }
            WidgetDispatchResult::UnmappedOutput => self.relayout(),
            WidgetDispatchResult::NoOutput => {}
        }
        true
    }
}
