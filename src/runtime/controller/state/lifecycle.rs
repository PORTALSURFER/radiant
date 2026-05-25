use super::super::{
    RuntimeInteractionState, RuntimeScratch, RuntimeTraversalState, RuntimeWorkQueues,
    SurfaceRuntime,
};
use crate::{
    gui::types::{Point, Rect, Vector2},
    layout::{LayoutDebugOptions, LayoutEngine, LayoutOutput, LayoutState},
    runtime::{RuntimeBridge, SurfaceRuntimeProjection},
};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Build a generic runtime controller for the provided viewport.
    pub fn new(mut bridge: Bridge, viewport: Vector2) -> Self {
        let viewport = normalized_viewport(viewport);
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
            traversal: RuntimeTraversalState::default(),
            scratch: RuntimeScratch::default(),
            interaction: RuntimeInteractionState::default(),
            repaint_requested: false,
            exit_requested: false,
            runtime_work: RuntimeWorkQueues::default(),
        };
        runtime.relayout_with_traversal(traversal);
        runtime
    }

    /// Replace the viewport and recompute layout for the current surface.
    pub fn set_viewport(&mut self, viewport: Vector2) {
        let viewport = normalized_viewport(viewport);
        if self.viewport == viewport {
            return;
        }
        self.viewport = viewport;
        self.relayout_current_surface();
    }

    /// Reproject the latest host state into a fresh immutable surface snapshot.
    pub fn refresh(&mut self) {
        let mut next_surface = self.bridge.pull_surface();
        std::mem::swap(
            &mut self.traversal.widgets.paths.previous,
            &mut self.traversal.widgets.paths.current,
        );
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
            &self.traversal.widgets.paths.previous,
        );
        self.surface = next_surface;
        self.layout_root = layout_root;
        self.restore_pointer_capture_state();
        self.relayout_with_traversal(traversal);
        self.clear_stale_interaction_state();
        if let Some(widget_id) = self.interaction.focus.focused_widget {
            self.route_focus_changed(widget_id, true);
        }
    }

    fn clear_stale_interaction_state(&mut self) {
        if self
            .interaction
            .focus
            .focused_widget
            .is_some_and(|widget_id| !self.traversal.widgets.focusable.contains(widget_id))
        {
            self.interaction.focus.focused_widget = None;
        }
        if self.interaction.pointer.capture.is_some_and(|widget_id| {
            !self
                .traversal
                .widgets
                .paths
                .current
                .contains_key(&widget_id)
        }) {
            self.interaction.pointer.capture = None;
        }
        if self
            .interaction
            .pointer
            .scroll_drag_capture
            .is_some_and(|capture| !self.traversal.containers.scroll.contains(capture.node_id))
        {
            self.interaction.pointer.scroll_drag_capture = None;
        }
        if self
            .interaction
            .hover
            .scroll_affordance
            .is_some_and(|node_id| !self.traversal.containers.scroll.contains(node_id))
        {
            self.interaction.hover.scroll_affordance = None;
        }
        if self.interaction.hover.widget.is_some_and(|widget_id| {
            !self
                .traversal
                .widgets
                .paths
                .current
                .contains_key(&widget_id)
        }) {
            self.interaction.hover.widget = None;
        }
        if self
            .interaction
            .hover
            .container
            .is_some_and(|node_id| !self.traversal.containers.styled.contains(node_id))
        {
            self.interaction.hover.container = None;
        }
    }
}

fn normalized_viewport(viewport: Vector2) -> Rect {
    Rect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(viewport.x.max(1.0), viewport.y.max(1.0)),
    )
}
