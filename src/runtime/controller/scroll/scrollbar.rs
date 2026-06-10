//! Scrollbar hit testing and drag routing.

use super::{
    super::{ScrollDragCapture, SurfaceRuntime},
    ScrollUpdate,
};
use crate::{
    gui::types::{Point, Rect, Vector2},
    layout::NodeId,
    runtime::{RuntimeBridge, paint::resolve_scroll_affordance},
};

#[cfg(test)]
#[path = "scrollbar/tests.rs"]
mod tests;

const SCROLLBAR_HIT_WIDTH: f32 = 10.0;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::runtime::controller) fn start_scrollbar_drag_at(&mut self, point: Point) -> bool {
        let Some(capture) = self.scrollbar_drag_capture_at(point) else {
            return false;
        };
        self.interaction.pointer.scroll_drag_capture = Some(capture);
        self.interaction.hover.scroll_affordance = Some(capture.node_id);
        self.repaint_requested = true;
        true
    }

    pub(in crate::runtime::controller) fn drag_scrollbar_to(
        &mut self,
        point: Point,
        refresh_after_message: bool,
    ) -> bool {
        let Some(capture) = self.interaction.pointer.scroll_drag_capture else {
            return false;
        };
        if self.interaction.hover.scroll_affordance != Some(capture.node_id) {
            self.interaction.hover.scroll_affordance = Some(capture.node_id);
            self.repaint_requested = true;
        }
        let Some(content_id) = self
            .traversal
            .containers
            .scroll_content_by_container
            .get(&capture.node_id)
            .copied()
        else {
            self.interaction.pointer.scroll_drag_capture = None;
            return false;
        };
        let Some(affordance) = resolve_scroll_affordance(capture.node_id, content_id, &self.layout)
        else {
            self.interaction.pointer.scroll_drag_capture = None;
            return false;
        };
        let travel = (affordance.track.height() - affordance.thumb.height()).max(0.0);
        if travel <= f32::EPSILON {
            return true;
        }
        let thumb_y = (point.y - affordance.thumb.height() * capture.grip_fraction)
            .clamp(affordance.track.min.y, affordance.track.min.y + travel);
        let offset_fraction = (thumb_y - affordance.track.min.y) / travel;
        let previous_offset = self.layout_state.scroll_offset(capture.node_id);
        self.layout_state.scroll_offsets.insert(
            capture.node_id,
            Vector2::new(previous_offset.x, offset_fraction * affordance.max_scroll),
        );
        self.relayout_current_surface();
        let offset = self.layout_state.scroll_offset(capture.node_id);
        if offset != previous_offset {
            let viewport = self
                .layout
                .rects
                .get(&capture.node_id)
                .map(|rect| Vector2::new(rect.width(), rect.height()))
                .unwrap_or_default();
            self.report_scroll_update_with_refresh(
                ScrollUpdate {
                    node_id: capture.node_id,
                    position: point,
                    delta: Vector2::new(offset.x - previous_offset.x, offset.y - previous_offset.y),
                    previous_offset,
                    offset,
                    viewport,
                },
                refresh_after_message,
            );
        }
        self.repaint_requested |= refresh_after_message;
        true
    }

    pub(in crate::runtime::controller) fn scroll_affordance_at(
        &self,
        point: Point,
    ) -> Option<NodeId> {
        self.scrollbar_drag_capture_at(point)
            .map(|capture| capture.node_id)
    }

    fn scrollbar_drag_capture_at(&self, point: Point) -> Option<ScrollDragCapture> {
        self.traversal
            .containers
            .scroll
            .visible()
            .iter()
            .rev()
            .copied()
            .find_map(|node_id| {
                let viewport = self.layout.rects.get(&node_id).copied()?;
                if !scrollbar_hit_column_contains_point(viewport, point)
                    || !self.container_clip_contains_point(node_id, point)
                {
                    return None;
                }
                let content_id = self
                    .traversal
                    .containers
                    .scroll_content_by_container
                    .get(&node_id)
                    .copied()?;
                let affordance = resolve_scroll_affordance(node_id, content_id, &self.layout)?;
                if !scrollbar_thumb_hit_rect(affordance.thumb).contains(point) {
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

fn scrollbar_hit_column_contains_point(viewport: Rect, point: Point) -> bool {
    viewport.contains(point) && point.x >= viewport.max.x - SCROLLBAR_HIT_WIDTH
}

fn scrollbar_thumb_hit_rect(thumb: Rect) -> Rect {
    Rect::from_min_max(
        Point::new(thumb.max.x - SCROLLBAR_HIT_WIDTH, thumb.min.y),
        Point::new(thumb.max.x, thumb.max.y),
    )
}
