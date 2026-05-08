use super::*;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Return the first projected widget whose laid-out bounds contain `point`.
    pub fn widget_at(&self, point: Point) -> Option<WidgetId> {
        self.widget_hit_order
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
        self.styled_container_hit_order
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

    fn widget_clip_contains_point(&self, widget_id: WidgetId, point: Point) -> bool {
        self.widget_clip_ancestors
            .get(&widget_id)
            .is_none_or(|clip_nodes| {
                clip_nodes.iter().all(|node_id| {
                    self.layout
                        .rects
                        .get(node_id)
                        .is_some_and(|rect| rect.contains(point))
                })
            })
    }

    fn container_clip_contains_point(&self, node_id: NodeId, point: Point) -> bool {
        self.container_clip_ancestors
            .get(&node_id)
            .is_none_or(|clip_nodes| {
                clip_nodes.iter().all(|clip_node| {
                    self.layout
                        .rects
                        .get(clip_node)
                        .is_some_and(|rect| rect.contains(point))
                })
            })
    }

    /// Scroll the topmost scroll container under `point`.
    ///
    /// Returns `true` when a scroll container accepted the delta.
    pub fn scroll_at(&mut self, point: Point, delta: Vector2) -> bool {
        let Some(node_id) = self.scroll_container_at(point) else {
            return false;
        };
        let current = self.layout_state.scroll_offset(node_id);
        self.layout_state.scroll_offsets.insert(
            node_id,
            Vector2::new(
                (current.x + delta.x).max(0.0),
                (current.y + delta.y).max(0.0),
            ),
        );
        self.relayout();
        true
    }

    /// Route wheel input to the topmost widget under `point`, then fall back to
    /// scrolling the topmost scroll container under the pointer.
    pub fn wheel_or_scroll_at(&mut self, point: Point, delta: Vector2) -> bool {
        if self.dispatch_wheel_at(point, delta) {
            return true;
        }
        self.scroll_at(point, delta)
    }

    /// Route wheel input but defer host-surface refresh until the caller chooses
    /// to refresh. This is intended for GPU-backed surfaces whose bounds do not
    /// change during rapid wheel updates.
    pub fn wheel_or_scroll_at_deferred_refresh(&mut self, point: Point, delta: Vector2) -> bool {
        if self.dispatch_wheel_at_with_refresh(point, delta, false) {
            return true;
        }
        self.scroll_at(point, delta)
    }

    fn dispatch_wheel_at(&mut self, point: Point, delta: Vector2) -> bool {
        self.dispatch_wheel_at_with_refresh(point, delta, true)
    }

    fn dispatch_wheel_at_with_refresh(
        &mut self,
        point: Point,
        delta: Vector2,
        refresh_after_message: bool,
    ) -> bool {
        let Some(widget_id) = self.widget_at(point) else {
            return false;
        };
        let Some(bounds) = self.layout.rects.get(&widget_id).copied() else {
            return false;
        };
        let Some(output) = self.surface.dispatch_widget_input(
            widget_id,
            bounds,
            WidgetInput::Wheel {
                position: point,
                delta,
            },
        ) else {
            self.capture_text_input_state(widget_id);
            self.capture_pointer_capture_state(widget_id);
            return false;
        };
        self.capture_text_input_state(widget_id);
        self.capture_pointer_capture_state(widget_id);
        if let Some(message) = self.surface.dispatch_widget_output(widget_id, output) {
            if refresh_after_message {
                self.dispatch_message(message);
            } else {
                let mut outcome = CommandOutcome::default();
                self.dispatch_message_inner(message, &mut outcome);
            }
        } else {
            self.relayout();
        }
        true
    }

    fn scroll_container_at(&self, point: Point) -> Option<NodeId> {
        self.scroll_hit_order.iter().rev().copied().find(|node_id| {
            self.layout
                .rects
                .get(node_id)
                .is_some_and(|rect| rect.contains(point))
                && self
                    .layout
                    .overflow_flags
                    .get(node_id)
                    .is_some_and(|overflow| {
                        overflow.policy == OverflowPolicy::Scroll && (overflow.x || overflow.y)
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
        if self.hovered_widget != hover_widget {
            if let Some(previous) = self.hovered_widget {
                let _ = self.dispatch_input(previous, WidgetInput::PointerMove { position });
            }
            self.hovered_widget = hover_widget;
        }

        let target = self.pointer_capture.or(pointer_widget)?;
        self.dispatch_input(target, WidgetInput::PointerMove { position })
            .then_some(target)
    }

    fn widget_suppresses_container_hover(&self, widget_id: Option<WidgetId>) -> bool {
        let Some(widget_id) = widget_id else {
            return false;
        };
        self.surface.find_widget(widget_id).is_some_and(|widget| {
            let object = widget.widget_object();
            if !object.common().paint.paints_state_layers {
                return false;
            }
            if object.as_any().downcast_ref::<TextWidget>().is_some()
                || object.as_any().downcast_ref::<ImageWidget>().is_some()
                || object.as_any().downcast_ref::<CanvasWidget>().is_some()
            {
                return false;
            }
            object.common().focus != FocusBehavior::None
                || object.as_any().downcast_ref::<CardWidget>().is_some()
        })
    }
}
