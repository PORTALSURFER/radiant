use super::*;
use crate::runtime::paint::resolve_scroll_affordance;

const SCROLLBAR_HIT_WIDTH: f32 = 10.0;

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
        self.relayout_current_surface();
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
        let Some(widget_id) = self.wheel_widget_at(point) else {
            return false;
        };
        let Some(bounds) = self.layout.rects.get(&widget_id).copied() else {
            return false;
        };
        let Some(result) = self.dispatch_surface_input(
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

    fn wheel_widget_at(&self, point: Point) -> Option<WidgetId> {
        self.visible_wheel_hit_order
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
                && self.container_clip_contains_point(*node_id, point)
        })
    }

    pub(super) fn start_scrollbar_drag_at(&mut self, point: Point) -> bool {
        let Some(capture) = self.scrollbar_drag_capture_at(point) else {
            return false;
        };
        self.scroll_drag_capture = Some(capture);
        self.repaint_requested = true;
        true
    }

    pub(super) fn drag_scrollbar_to(&mut self, point: Point) -> bool {
        let Some(capture) = self.scroll_drag_capture else {
            return false;
        };
        let Some(content_id) = self
            .scroll_content_by_container
            .get(&capture.node_id)
            .copied()
        else {
            self.scroll_drag_capture = None;
            return false;
        };
        let Some(affordance) = resolve_scroll_affordance(capture.node_id, content_id, &self.layout)
        else {
            self.scroll_drag_capture = None;
            return false;
        };
        let travel = (affordance.track.height() - affordance.thumb.height()).max(0.0);
        if travel <= f32::EPSILON {
            return true;
        }
        let thumb_y = (point.y - affordance.thumb.height() * capture.grip_fraction)
            .clamp(affordance.track.min.y, affordance.track.min.y + travel);
        let offset_fraction = (thumb_y - affordance.track.min.y) / travel;
        let current = self.layout_state.scroll_offset(capture.node_id);
        self.layout_state.scroll_offsets.insert(
            capture.node_id,
            Vector2::new(current.x, offset_fraction * affordance.max_scroll),
        );
        self.relayout_current_surface();
        self.repaint_requested = true;
        true
    }

    fn scrollbar_drag_capture_at(&self, point: Point) -> Option<ScrollDragCapture> {
        self.scroll_hit_order
            .iter()
            .rev()
            .copied()
            .find_map(|node_id| {
                let content_id = self.scroll_content_by_container.get(&node_id).copied()?;
                let affordance = resolve_scroll_affordance(node_id, content_id, &self.layout)?;
                if !scrollbar_thumb_hit_rect(affordance.thumb).contains(point)
                    || !self.container_clip_contains_point(node_id, point)
                {
                    return None;
                }
                let grip_fraction = ((point.y - affordance.thumb.min.y)
                    / affordance.thumb.height())
                .clamp(0.0, 1.0);
                Some(ScrollDragCapture {
                    node_id,
                    grip_fraction,
                })
            })
    }
}

fn scrollbar_thumb_hit_rect(thumb: Rect) -> Rect {
    Rect::from_min_max(
        Point::new(thumb.max.x - SCROLLBAR_HIT_WIDTH, thumb.min.y),
        Point::new(thumb.max.x, thumb.max.y),
    )
}
