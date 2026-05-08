use super::*;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn capture_text_input_state(&mut self, widget_id: WidgetId) {
        let Some(widget) = self.surface.find_widget(widget_id) else {
            return;
        };
        if let Some(input) = widget
            .widget_object()
            .as_any()
            .downcast_ref::<TextInputWidget>()
        {
            self.text_input_states
                .insert(widget_id, input.state.clone());
        }
    }

    pub(super) fn restore_text_input_states(&mut self) {
        let states = self.text_input_states.clone();
        for (widget_id, state) in states {
            let Some(widget) = self.surface.find_widget_mut(widget_id) else {
                self.text_input_states.remove(&widget_id);
                continue;
            };
            let Some(input) = widget
                .widget_object_mut()
                .as_any_mut()
                .downcast_mut::<TextInputWidget>()
            else {
                self.text_input_states.remove(&widget_id);
                continue;
            };
            if input.state.value == state.value {
                input.state = state;
            }
        }
    }

    pub(super) fn capture_pointer_capture_state(&mut self, widget_id: WidgetId) {
        if self.pointer_capture != Some(widget_id) {
            return;
        }
        let Some(widget) = self.surface.find_widget(widget_id) else {
            self.pointer_capture_state = None;
            return;
        };
        self.pointer_capture_state = Some((widget_id, widget.widget_object().common().state));
        if let Some(scrollbar) = widget
            .widget_object()
            .as_any()
            .downcast_ref::<ScrollbarWidget>()
        {
            self.scrollbar_states.insert(widget_id, scrollbar.state);
        }
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

    pub(super) fn restore_scrollbar_states(&mut self) {
        let states = self.scrollbar_states.clone();
        for (widget_id, state) in states {
            let Some(widget) = self.surface.find_widget_mut(widget_id) else {
                self.scrollbar_states.remove(&widget_id);
                continue;
            };
            let Some(scrollbar) = widget
                .widget_object_mut()
                .as_any_mut()
                .downcast_mut::<ScrollbarWidget>()
            else {
                self.scrollbar_states.remove(&widget_id);
                continue;
            };
            scrollbar.state.drag_grip_fraction = state.drag_grip_fraction;
        }
    }

    pub(super) fn relayout(&mut self) {
        self.layout = layout_tree_with_state(
            &self.surface.layout_node(),
            self.viewport,
            &self.layout_state,
            LayoutDebugOptions::default(),
        );
        self.widget_hit_order = self.surface.widget_paint_order();
        self.styled_container_hit_order = self.surface.styled_container_order();
        self.scroll_hit_order = self.surface.scroll_container_order();
        self.widget_clip_ancestors = self.surface.widget_clip_ancestors();
        self.container_clip_ancestors = self.surface.container_clip_ancestors();
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
                    .get(&self.surface.scroll_content_id(diagnostic.node_id)?)?;
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
