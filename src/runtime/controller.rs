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
    widget_hit_order: Vec<WidgetId>,
    focusable_widget_order: Vec<WidgetId>,
    focusable_widget_rank: HashMap<WidgetId, usize>,
    pointer_hit_order: Vec<WidgetId>,
    pointer_hit_rank: HashMap<WidgetId, usize>,
    visible_pointer_hit_order: Vec<WidgetId>,
    widget_paths: HashMap<WidgetId, WidgetPath>,
    previous_widget_paths: HashMap<WidgetId, WidgetPath>,
    container_hover_suppression: HashSet<WidgetId>,
    keyboard_focus_order: Vec<WidgetId>,
    keyboard_focus_rank: HashMap<WidgetId, usize>,
    wheel_hit_order: Vec<WidgetId>,
    wheel_hit_rank: HashMap<WidgetId, usize>,
    visible_wheel_hit_order: Vec<WidgetId>,
    stateful_widget_order: Vec<WidgetId>,
    styled_container_hit_order: Vec<NodeId>,
    styled_container_hit_rank: HashMap<NodeId, usize>,
    visible_styled_container_hit_order: Vec<NodeId>,
    scroll_hit_order: Vec<NodeId>,
    scroll_hit_rank: HashMap<NodeId, usize>,
    visible_scroll_hit_order: Vec<NodeId>,
    widget_clip_ancestors: HashMap<WidgetId, ClipAncestors>,
    container_clip_ancestors: HashMap<NodeId, ClipAncestors>,
    scroll_content_by_container: HashMap<NodeId, NodeId>,
    scroll_clamp_updates: Vec<(NodeId, Vector2)>,
    projection_scroll_stack: Vec<NodeId>,
    projection_child_path: Vec<usize>,
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
    runtime_messages: Vec<Message>,
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
        let layout_state = LayoutState::default();
        let mut layout_engine = LayoutEngine::default();
        let layout = layout_engine.layout_with_state(
            &layout_root,
            viewport,
            &layout_state,
            LayoutDebugOptions::default(),
        );
        let focusable_widget_rank = hit_rank(&traversal.focusable_widget_order);
        let pointer_hit_rank = hit_rank(&traversal.pointer_hit_order);
        let keyboard_focus_rank = hit_rank(&traversal.keyboard_focus_order);
        let wheel_hit_rank = hit_rank(&traversal.wheel_hit_order);
        let styled_container_hit_rank = hit_rank(&traversal.styled_container_order);
        let scroll_hit_rank = hit_rank(&traversal.scroll_container_order);
        let mut visible_pointer_hit_order = Vec::new();
        collect_visible_hit_order(
            &layout,
            &traversal.pointer_hit_order,
            &pointer_hit_rank,
            &mut visible_pointer_hit_order,
        );
        let mut visible_wheel_hit_order = Vec::new();
        collect_visible_hit_order(
            &layout,
            &traversal.wheel_hit_order,
            &wheel_hit_rank,
            &mut visible_wheel_hit_order,
        );
        let mut visible_styled_container_hit_order = Vec::new();
        collect_visible_hit_order(
            &layout,
            &traversal.styled_container_order,
            &styled_container_hit_rank,
            &mut visible_styled_container_hit_order,
        );
        let mut visible_scroll_hit_order = Vec::new();
        collect_visible_hit_order(
            &layout,
            &traversal.scroll_container_order,
            &scroll_hit_rank,
            &mut visible_scroll_hit_order,
        );
        Self {
            bridge,
            viewport,
            surface,
            layout_root,
            layout_engine,
            layout,
            layout_state,
            widget_hit_order: traversal.widget_paint_order,
            focusable_widget_order: traversal.focusable_widget_order,
            focusable_widget_rank,
            pointer_hit_order: traversal.pointer_hit_order,
            pointer_hit_rank,
            visible_pointer_hit_order,
            widget_paths: traversal.widget_paths,
            previous_widget_paths: HashMap::new(),
            container_hover_suppression: traversal.container_hover_suppression,
            keyboard_focus_order: traversal.keyboard_focus_order,
            keyboard_focus_rank,
            wheel_hit_order: traversal.wheel_hit_order,
            wheel_hit_rank,
            visible_wheel_hit_order,
            stateful_widget_order: traversal.stateful_widget_order,
            styled_container_hit_order: traversal.styled_container_order,
            styled_container_hit_rank,
            visible_styled_container_hit_order,
            scroll_hit_order: traversal.scroll_container_order,
            scroll_hit_rank,
            visible_scroll_hit_order,
            widget_clip_ancestors: traversal.widget_clip_ancestors,
            container_clip_ancestors: traversal.container_clip_ancestors,
            scroll_content_by_container: traversal.scroll_content_by_container,
            scroll_clamp_updates: Vec::new(),
            projection_scroll_stack: Vec::new(),
            projection_child_path: Vec::new(),
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
            runtime_messages: Vec::new(),
        }
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
            &mut self.projection_scroll_stack,
            &mut self.projection_child_path,
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
            .is_some_and(|widget_id| !self.focusable_widget_rank.contains_key(&widget_id))
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
            .is_some_and(|capture| !self.scroll_hit_rank.contains_key(&capture.node_id))
        {
            self.scroll_drag_capture = None;
        }
        if self
            .hovered_scroll_affordance
            .is_some_and(|node_id| !self.scroll_hit_rank.contains_key(&node_id))
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
            .is_some_and(|node_id| !self.styled_container_hit_rank.contains_key(&node_id))
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

    fn dispatch_surface_input(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<WidgetDispatchResult<Message>> {
        let Some(child_path) = self.widget_paths.get(&widget_id) else {
            return self
                .surface
                .dispatch_widget_input_message(widget_id, bounds, input);
        };
        self.surface
            .dispatch_widget_input_message_at_path(widget_id, child_path, bounds, input)
    }

    fn dispatch_raw_surface_input(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
    ) -> bool {
        let Some(child_path) = self.widget_paths.get(&widget_id) else {
            return self
                .surface
                .dispatch_widget_input(widget_id, bounds, input)
                .is_some();
        };
        self.surface
            .dispatch_widget_input_at_path(widget_id, child_path, bounds, input)
            .is_some()
    }

    fn surface_widget(&self, widget_id: WidgetId) -> Option<&SurfaceWidget<Message>> {
        self.widget_paths
            .get(&widget_id)
            .and_then(|child_path| self.surface.find_widget_at_path(widget_id, child_path))
            .or_else(|| self.surface.find_widget(widget_id))
    }

    fn surface_widget_mut(&mut self, widget_id: WidgetId) -> Option<&mut SurfaceWidget<Message>> {
        let surface = &mut self.surface;
        if let Some(child_path) = self.widget_paths.get(&widget_id) {
            return surface.find_widget_mut_at_path(widget_id, child_path);
        }
        surface.find_widget_mut(widget_id)
    }
}

fn hit_rank(order: &[NodeId]) -> HashMap<NodeId, usize> {
    let mut rank = HashMap::with_capacity(order.len());
    collect_hit_rank(order, &mut rank);
    rank
}

fn collect_hit_rank(order: &[NodeId], out: &mut HashMap<NodeId, usize>) {
    out.clear();
    if order.len() > out.capacity() {
        out.reserve(order.len());
    }
    out.extend(
        order
            .iter()
            .copied()
            .enumerate()
            .map(|(index, node_id)| (node_id, index)),
    );
}

fn collect_visible_hit_order(
    layout: &LayoutOutput,
    order: &[NodeId],
    rank: &HashMap<NodeId, usize>,
    out: &mut Vec<NodeId>,
) {
    const SPARSE_LAYOUT_SCAN_FACTOR: usize = 4;
    out.clear();
    let visible_capacity = layout.rects.len().min(order.len());
    if visible_capacity > out.capacity() {
        out.reserve(visible_capacity);
    }
    if order.len() <= layout.rects.len().saturating_mul(SPARSE_LAYOUT_SCAN_FACTOR) {
        out.extend(
            order
                .iter()
                .copied()
                .filter(|node_id| layout.rects.contains_key(node_id)),
        );
        return;
    }

    out.extend(
        layout
            .rects
            .keys()
            .filter(|node_id| rank.contains_key(node_id))
            .copied(),
    );
    out.sort_by_key(|node_id| rank.get(node_id).copied().unwrap_or(usize::MAX));
}

#[cfg(test)]
fn visible_hit_order(
    layout: &LayoutOutput,
    order: &[NodeId],
    rank: &HashMap<NodeId, usize>,
) -> Vec<NodeId> {
    let mut visible = Vec::new();
    collect_visible_hit_order(layout, order, rank, &mut visible);
    visible
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_visible_hit_order_preserves_traversal_order() {
        let mut layout = LayoutOutput::default();
        for node_id in [100, 50, 2] {
            layout.rects.insert(
                node_id,
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(10.0, 10.0)),
            );
        }
        let order = vec![100, 200, 201, 202, 203, 204, 205, 206, 207, 50, 208, 209, 2];
        let rank = hit_rank(&order);

        assert_eq!(visible_hit_order(&layout, &order, &rank), vec![100, 50, 2]);
    }

    #[test]
    fn dense_visible_hit_order_reuses_output_buffer() {
        let mut layout = LayoutOutput::default();
        for node_id in [100, 50, 2] {
            layout.rects.insert(
                node_id,
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(10.0, 10.0)),
            );
        }
        let order = vec![100, 200, 201, 202, 203, 204, 205, 206, 207, 50, 208, 209, 2];
        let rank = hit_rank(&order);
        let mut visible = Vec::with_capacity(8);
        visible.push(999);
        let capacity = visible.capacity();

        collect_visible_hit_order(&layout, &order, &rank, &mut visible);

        assert_eq!(visible, vec![100, 50, 2]);
        assert_eq!(visible.capacity(), capacity);
    }

    #[test]
    fn visible_hit_order_presizes_empty_output_buffer() {
        let mut layout = LayoutOutput::default();
        for node_id in [100, 50, 2] {
            layout.rects.insert(
                node_id,
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(10.0, 10.0)),
            );
        }
        let order = vec![100, 200, 201, 202, 203, 204, 205, 206, 207, 50, 208, 209, 2];
        let rank = hit_rank(&order);
        let mut visible = Vec::new();

        collect_visible_hit_order(&layout, &order, &rank, &mut visible);

        assert_eq!(visible, vec![100, 50, 2]);
        assert!(visible.capacity() >= 3);
    }

    #[test]
    fn visible_hit_order_grows_reused_output_buffer_to_visible_capacity() {
        let mut layout = LayoutOutput::default();
        for node_id in 0..64 {
            layout.rects.insert(
                node_id,
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(10.0, 10.0)),
            );
        }
        let order = (0..128).collect::<Vec<_>>();
        let rank = hit_rank(&order);
        let mut visible = Vec::with_capacity(8);

        collect_visible_hit_order(&layout, &order, &rank, &mut visible);

        assert_eq!(visible.len(), 64);
        assert!(visible.capacity() >= 64);
    }

    #[test]
    fn hit_rank_reuses_output_map() {
        let mut rank = HashMap::with_capacity(8);
        rank.insert(999, 999);
        let capacity = rank.capacity();

        collect_hit_rank(&[5, 1, 9], &mut rank);

        assert_eq!(rank.get(&5), Some(&0));
        assert_eq!(rank.get(&1), Some(&1));
        assert_eq!(rank.get(&9), Some(&2));
        assert!(!rank.contains_key(&999));
        assert!(rank.capacity() >= capacity);
    }

    #[test]
    fn hit_rank_presizes_reused_map_for_growth() {
        let mut rank = HashMap::with_capacity(4);
        let order = (0..96).collect::<Vec<_>>();

        collect_hit_rank(&order, &mut rank);

        assert_eq!(rank.len(), 96);
        assert!(rank.capacity() >= 96);
        assert_eq!(rank.get(&95), Some(&95));
    }
}
