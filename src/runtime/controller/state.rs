use super::*;

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
        };
        runtime.relayout_with_traversal(traversal);
        runtime
    }

    /// Replace the viewport and recompute layout for the current surface.
    pub fn set_viewport(&mut self, viewport: Vector2) {
        self.viewport = normalized_viewport(viewport);
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

    pub(super) fn capture_pointer_capture_state(&mut self, widget_id: WidgetId) {
        if self.pointer_capture != Some(widget_id) {
            return;
        }
        let Some(widget) = self.surface_widget(widget_id) else {
            self.pointer_capture_state = None;
            return;
        };
        self.pointer_capture_state = Some((widget_id, widget.widget_object().common().state));
    }

    pub(super) fn restore_pointer_capture_state(&mut self) {
        let Some((widget_id, state)) = self.pointer_capture_state else {
            return;
        };
        if self.pointer_capture != Some(widget_id) {
            self.pointer_capture_state = None;
            return;
        }
        let Some(widget) = self.surface_widget_mut(widget_id) else {
            self.pointer_capture_state = None;
            return;
        };
        widget.widget_object_mut().common_mut().state = state;
    }

    pub(super) fn relayout(&mut self) {
        let mut traversal = self.take_reusable_traversal_index(true);
        self.layout_root = self.surface.runtime_projection_reusing_with_scratch(
            &mut traversal,
            &mut self.scratch.projection_scroll_stack,
            &mut self.scratch.projection_child_path,
        );
        self.relayout_with_traversal(traversal);
    }

    pub(super) fn relayout_current_surface(&mut self) {
        self.layout = self.layout_engine.layout_with_state(
            &self.layout_root,
            self.viewport,
            &self.layout_state,
            self.layout_debug_options,
        );
        self.refresh_visible_hit_orders();
        self.sync_scroll_offsets();
    }

    pub(super) fn relayout_with_traversal(&mut self, traversal: SurfaceTraversalIndex) {
        self.layout = self.layout_engine.layout_with_state(
            &self.layout_root,
            self.viewport,
            &self.layout_state,
            self.layout_debug_options,
        );
        self.widget_hit_order = traversal.widget_paint_order;
        self.widget_paths = traversal.widget_paths;
        self.focusable_widgets
            .set_order(traversal.focusable_widget_order);
        self.pointer_widgets.set_order(traversal.pointer_hit_order);
        self.pointer_widgets.refresh_visible(&self.layout);
        self.container_hover_suppression = traversal.container_hover_suppression;
        self.keyboard_focus_widgets
            .set_order(traversal.keyboard_focus_order);
        self.wheel_widgets.set_order(traversal.wheel_hit_order);
        self.stateful_widget_order = traversal.stateful_widget_order;
        self.wheel_widgets.refresh_visible(&self.layout);
        self.styled_containers
            .set_order(traversal.styled_container_order);
        self.styled_containers.refresh_visible(&self.layout);
        self.scroll_containers
            .set_order(traversal.scroll_container_order);
        self.scroll_containers.refresh_visible(&self.layout);
        self.widget_clip_ancestors = traversal.widget_clip_ancestors;
        self.container_clip_ancestors = traversal.container_clip_ancestors;
        self.scroll_content_by_container = traversal.scroll_content_by_container;
        self.sync_scroll_offsets();
    }

    pub(super) fn refresh_visible_hit_orders(&mut self) {
        self.pointer_widgets.refresh_visible(&self.layout);
        self.wheel_widgets.refresh_visible(&self.layout);
        self.styled_containers.refresh_visible(&self.layout);
        self.scroll_containers.refresh_visible(&self.layout);
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

    fn sync_scroll_offsets(&mut self) {
        self.scratch.scroll_clamp_updates.clear();
        self.scratch.scroll_clamp_updates.extend(
            self.layout
                .diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.code
                        == crate::layout::LayoutDiagnosticCode::InvalidScrollOffsetClamped
                })
                .filter_map(|diagnostic| {
                    let child_rect = self
                        .layout
                        .rects
                        .get(self.scroll_content_by_container.get(&diagnostic.node_id)?)?;
                    let viewport_rect = self.layout.rects.get(&diagnostic.node_id)?;
                    Some((
                        diagnostic.node_id,
                        Vector2::new(
                            self.layout_state
                                .scroll_offset(diagnostic.node_id)
                                .x
                                .min((child_rect.width() - viewport_rect.width()).max(0.0)),
                            self.layout_state
                                .scroll_offset(diagnostic.node_id)
                                .y
                                .min((child_rect.height() - viewport_rect.height()).max(0.0)),
                        ),
                    ))
                }),
        );
        for (node_id, offset) in self.scratch.scroll_clamp_updates.drain(..) {
            self.layout_state.scroll_offsets.insert(node_id, offset);
        }
    }

    pub(super) fn take_reusable_traversal_index(
        &mut self,
        reuse_widget_paths: bool,
    ) -> SurfaceTraversalIndex {
        SurfaceTraversalIndex {
            widget_paint_order: std::mem::take(&mut self.widget_hit_order),
            focusable_widget_order: self.focusable_widgets.take_order(),
            keyboard_focus_order: self.keyboard_focus_widgets.take_order(),
            pointer_hit_order: self.pointer_widgets.take_order(),
            wheel_hit_order: self.wheel_widgets.take_order(),
            stateful_widget_order: std::mem::take(&mut self.stateful_widget_order),
            widget_paths: if reuse_widget_paths {
                std::mem::take(&mut self.widget_paths)
            } else {
                Default::default()
            },
            container_hover_suppression: std::mem::take(&mut self.container_hover_suppression),
            styled_container_order: self.styled_containers.take_order(),
            scroll_container_order: self.scroll_containers.take_order(),
            widget_clip_ancestors: std::mem::take(&mut self.widget_clip_ancestors),
            container_clip_ancestors: std::mem::take(&mut self.container_clip_ancestors),
            scroll_content_by_container: std::mem::take(&mut self.scroll_content_by_container),
        }
    }
}

pub(super) fn normalized_viewport(viewport: Vector2) -> Rect {
    Rect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(viewport.x.max(1.0), viewport.y.max(1.0)),
    )
}
