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
    WidgetDispatchResult,
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
use std::collections::{BTreeMap, BTreeSet, HashMap};

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
    layout_root: crate::layout::LayoutNode,
    layout_engine: LayoutEngine,
    layout: LayoutOutput,
    layout_state: LayoutState,
    widget_hit_order: Vec<WidgetId>,
    focusable_widget_order: Vec<WidgetId>,
    pointer_hit_order: Vec<WidgetId>,
    pointer_hit_rank: HashMap<WidgetId, usize>,
    visible_pointer_hit_order: Vec<WidgetId>,
    widget_paths: HashMap<WidgetId, Vec<usize>>,
    container_hover_suppression: BTreeSet<WidgetId>,
    keyboard_focus_order: Vec<WidgetId>,
    wheel_hit_order: Vec<WidgetId>,
    wheel_hit_rank: HashMap<WidgetId, usize>,
    visible_wheel_hit_order: Vec<WidgetId>,
    styled_container_hit_order: Vec<NodeId>,
    styled_container_hit_rank: HashMap<NodeId, usize>,
    visible_styled_container_hit_order: Vec<NodeId>,
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
        let layout_root = surface.layout_node();
        let layout_state = LayoutState::default();
        let mut layout_engine = LayoutEngine::default();
        let layout = layout_engine.layout_with_state(
            &layout_root,
            viewport,
            &layout_state,
            LayoutDebugOptions::default(),
        );
        let traversal = surface.runtime_traversal_index();
        let pointer_hit_rank = hit_rank(&traversal.pointer_hit_order);
        let wheel_hit_rank = hit_rank(&traversal.wheel_hit_order);
        let styled_container_hit_rank = hit_rank(&traversal.styled_container_order);
        let visible_pointer_hit_order =
            visible_hit_order(&layout, &traversal.pointer_hit_order, &pointer_hit_rank);
        let visible_wheel_hit_order =
            visible_hit_order(&layout, &traversal.wheel_hit_order, &wheel_hit_rank);
        let visible_styled_container_hit_order = visible_hit_order(
            &layout,
            &traversal.styled_container_order,
            &styled_container_hit_rank,
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
            pointer_hit_order: traversal.pointer_hit_order,
            pointer_hit_rank,
            visible_pointer_hit_order,
            widget_paths: traversal.widget_paths,
            container_hover_suppression: traversal.container_hover_suppression,
            keyboard_focus_order: traversal.keyboard_focus_order,
            wheel_hit_order: traversal.wheel_hit_order,
            wheel_hit_rank,
            visible_wheel_hit_order,
            styled_container_hit_order: traversal.styled_container_order,
            styled_container_hit_rank,
            visible_styled_container_hit_order,
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
        next_surface.synchronize_widget_state_from(&self.surface);
        self.surface = next_surface;
        self.layout_root = self.surface.layout_node();
        self.restore_pointer_capture_state();
        self.relayout_with_traversal(traversal);
        if self
            .focused_widget
            .is_some_and(|widget_id| !self.focusable_widget_order.contains(&widget_id))
        {
            self.focused_widget = None;
        }
        if self
            .pointer_capture
            .is_some_and(|widget_id| !self.widget_hit_order.contains(&widget_id))
        {
            self.pointer_capture = None;
        }
        if self
            .hovered_widget
            .is_some_and(|widget_id| !self.widget_hit_order.contains(&widget_id))
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
}

fn hit_rank(order: &[NodeId]) -> HashMap<NodeId, usize> {
    order
        .iter()
        .copied()
        .enumerate()
        .map(|(index, node_id)| (node_id, index))
        .collect()
}

fn visible_hit_order(
    layout: &LayoutOutput,
    order: &[NodeId],
    rank: &HashMap<NodeId, usize>,
) -> Vec<NodeId> {
    const SPARSE_LAYOUT_SCAN_FACTOR: usize = 4;
    if order.len() <= layout.rects.len().saturating_mul(SPARSE_LAYOUT_SCAN_FACTOR) {
        return order
            .iter()
            .copied()
            .filter(|node_id| layout.rects.contains_key(node_id))
            .collect();
    }

    let mut visible = layout
        .rects
        .keys()
        .filter_map(|node_id| rank.get(node_id).map(|rank| (*rank, *node_id)))
        .collect::<Vec<_>>();
    visible.sort_by_key(|(rank, _)| *rank);
    visible.into_iter().map(|(_, node_id)| node_id).collect()
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
}
