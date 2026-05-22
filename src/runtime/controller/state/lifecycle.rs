use super::super::*;

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
            external_drag_session: None,
            drag_session: None,
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
        self.clear_stale_interaction_state();
        if let Some(widget_id) = self.focused_widget {
            self.route_focus_changed(widget_id, true);
        }
    }

    fn clear_stale_interaction_state(&mut self) {
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
    }
}

fn normalized_viewport(viewport: Vector2) -> Rect {
    Rect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(viewport.x.max(1.0), viewport.y.max(1.0)),
    )
}
