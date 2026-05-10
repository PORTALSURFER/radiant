use super::*;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
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
        let Some(result) = self.surface.dispatch_widget_input_message(
            widget_id,
            bounds,
            WidgetInput::Wheel {
                position: point,
                delta,
            },
        ) else {
            return false;
        };
        self.capture_pointer_capture_state(widget_id);
        match result {
            WidgetDispatchResult::Message(message) => {
                if refresh_after_message {
                    self.dispatch_message(message);
                } else {
                    let mut outcome = CommandOutcome::default();
                    self.dispatch_message_inner(message, &mut outcome);
                }
            }
            WidgetDispatchResult::UnmappedOutput => self.relayout(),
            WidgetDispatchResult::NoOutput => return false,
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
}
