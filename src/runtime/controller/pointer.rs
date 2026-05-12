use super::*;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Return the first projected widget whose laid-out bounds contain `point`.
    pub fn widget_at(&self, point: Point) -> Option<WidgetId> {
        self.visible_pointer_hit_order
            .iter()
            .rev()
            .copied()
            .find(|widget_id| {
                self.layout
                    .rects
                    .get(widget_id)
                    .is_some_and(|rect| rect.contains(point))
                    && self.widget_clip_contains_point(*widget_id, point)
            })
    }

    pub(super) fn styled_container_at(&self, point: Point) -> Option<NodeId> {
        self.visible_styled_container_hit_order
            .iter()
            .rev()
            .copied()
            .find(|node_id| {
                self.layout
                    .rects
                    .get(node_id)
                    .is_some_and(|rect| rect.contains(point))
                    && self.container_clip_contains_point(*node_id, point)
            })
    }

    pub(super) fn widget_clip_contains_point(&self, widget_id: WidgetId, point: Point) -> bool {
        self.widget_clip_ancestors
            .get(&widget_id)
            .is_none_or(|clip_nodes| {
                clip_nodes.as_slice().iter().all(|node_id| {
                    self.layout
                        .rects
                        .get(node_id)
                        .is_some_and(|rect| rect.contains(point))
                })
            })
    }

    pub(super) fn container_clip_contains_point(&self, node_id: NodeId, point: Point) -> bool {
        self.container_clip_ancestors
            .get(&node_id)
            .is_none_or(|clip_nodes| {
                clip_nodes.as_slice().iter().all(|clip_node| {
                    self.layout
                        .rects
                        .get(clip_node)
                        .is_some_and(|rect| rect.contains(point))
                })
            })
    }

    /// Route one normalized widget interaction by point hit test.
    ///
    /// Returns the targeted widget id when a projected widget handled the point.
    pub fn dispatch_input_at(&mut self, point: Point, input: WidgetInput) -> Option<WidgetId> {
        let widget_id = self.widget_at(point)?;
        if matches!(input, WidgetInput::PointerPress { .. }) {
            let _ = self.focus_widget(widget_id);
        }
        self.dispatch_input(widget_id, input).then_some(widget_id)
    }

    pub(super) fn dispatch_pointer_move(&mut self, position: Point) -> Option<WidgetId> {
        if self.drag_scrollbar_to(position) {
            return None;
        }
        let hovered_scroll_affordance = self.scroll_affordance_at(position);
        if self.hovered_scroll_affordance != hovered_scroll_affordance {
            self.hovered_scroll_affordance = hovered_scroll_affordance;
            self.repaint_requested = true;
        }
        let pointer_widget = self.widget_at(position);
        let hover_container = if self.widget_suppresses_container_hover(pointer_widget) {
            None
        } else {
            self.styled_container_at(position)
        };
        if self.hovered_container != hover_container {
            self.hovered_container = hover_container;
            self.repaint_requested = true;
        }
        let hover_widget = self
            .pointer_capture
            .filter(|widget_id| {
                self.layout
                    .rects
                    .get(widget_id)
                    .is_some_and(|rect| rect.contains(position))
            })
            .or_else(|| {
                self.pointer_capture
                    .is_none()
                    .then_some(pointer_widget)
                    .flatten()
            });
        let hover_changed = self.hovered_widget != hover_widget;
        if hover_changed {
            if let Some(previous) = self.hovered_widget {
                let _ = self.dispatch_input(previous, WidgetInput::PointerMove { position });
            }
            self.hovered_widget = hover_widget;
        }

        let target = self.pointer_capture.or(pointer_widget)?;
        let accepts_stable_pointer_move = self.widget_accepts_stable_pointer_move(target);
        if !hover_changed && self.pointer_capture.is_none() && !accepts_stable_pointer_move {
            return Some(target);
        }
        let routed = self.dispatch_input(target, WidgetInput::PointerMove { position });
        if routed && accepts_stable_pointer_move && self.pointer_capture.is_none() {
            // Stable pointer-move widgets may update local paint-only hover
            // state without emitting host messages. Request repaint here so
            // cursor and handle previews stay responsive without reducer churn.
            self.repaint_requested = true;
        }
        routed.then_some(target)
    }

    fn widget_suppresses_container_hover(&self, widget_id: Option<WidgetId>) -> bool {
        let Some(widget_id) = widget_id else {
            return false;
        };
        self.container_hover_suppression.contains(&widget_id)
    }

    fn widget_accepts_stable_pointer_move(&self, widget_id: WidgetId) -> bool {
        self.surface_widget(widget_id)
            .is_some_and(SurfaceWidget::accepts_pointer_move)
    }
}
