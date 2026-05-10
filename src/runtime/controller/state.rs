use super::*;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn capture_pointer_capture_state(&mut self, widget_id: WidgetId) {
        if self.pointer_capture != Some(widget_id) {
            return;
        }
        let Some(widget) = self.surface.find_widget(widget_id) else {
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
        let Some(widget) = self.surface.find_widget_mut(widget_id) else {
            self.pointer_capture_state = None;
            return;
        };
        widget.widget_object_mut().common_mut().state = state;
    }

    pub(super) fn relayout(&mut self) {
        let traversal = self.surface.runtime_traversal_index();
        self.relayout_with_traversal(traversal);
    }

    pub(super) fn relayout_with_traversal(&mut self, traversal: SurfaceTraversalIndex) {
        self.layout = layout_tree_with_state(
            &self.surface.layout_node(),
            self.viewport,
            &self.layout_state,
            LayoutDebugOptions::default(),
        );
        self.widget_hit_order = traversal.widget_paint_order;
        self.focusable_widget_order = traversal.focusable_widget_order;
        self.pointer_hit_order = traversal.pointer_hit_order;
        self.keyboard_focus_order = traversal.keyboard_focus_order;
        self.styled_container_hit_order = traversal.styled_container_order;
        self.scroll_hit_order = traversal.scroll_container_order;
        self.widget_clip_ancestors = traversal.widget_clip_ancestors;
        self.container_clip_ancestors = traversal.container_clip_ancestors;
        self.scroll_content_by_container = traversal.scroll_content_by_container;
        self.sync_scroll_offsets();
    }

    fn sync_scroll_offsets(&mut self) {
        let clamped: Vec<(NodeId, Vector2)> = self
            .layout
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == crate::layout::LayoutDiagnosticCode::InvalidScrollOffsetClamped
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
            })
            .collect();
        for (node_id, offset) in clamped {
            self.layout_state.scroll_offsets.insert(node_id, offset);
        }
    }
}

pub(super) fn normalized_viewport(viewport: Vector2) -> Rect {
    Rect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(viewport.x.max(1.0), viewport.y.max(1.0)),
    )
}
