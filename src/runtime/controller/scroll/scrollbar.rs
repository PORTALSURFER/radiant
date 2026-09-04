//! Scrollbar hit testing and drag routing.

use super::{
    super::{ScrollDragCapture, SurfaceRuntime},
    ScrollUpdate, ScrollUpdateMetadata,
};
use crate::runtime::controller::interaction_state::ScrollbarAxis;
use crate::runtime::{RuntimeBridge, paint::resolve_scroll_affordance};
use crate::{
    gui::types::{Point, Rect, Vector2},
    layout::NodeId,
    runtime::paint::{resolve_horizontal_scroll_affordance, scrollbar_viewport},
    widgets::PointerButton,
};

#[cfg(test)]
#[path = "scrollbar/tests.rs"]
mod tests;

const SCROLLBAR_HIT_WIDTH: f32 = 10.0;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::runtime::controller) fn start_scrollbar_drag_at(
        &mut self,
        point: Point,
        button: PointerButton,
    ) -> bool {
        let Some((node_id, grip_fraction, axis)) = self.scrollbar_drag_capture_at(point) else {
            return false;
        };
        let capture = ScrollDragCapture {
            node_id,
            grip_fraction,
            button,
            axis,
            start_offset: self.layout_state.scroll_offset(node_id),
        };
        self.interaction.pointer.scroll_drag_capture = Some(capture);
        self.interaction.hover.scroll_affordance = Some(capture.node_id);
        self.note_scroll_visibility_mutation();
        self.repaint_requested = true;
        true
    }

    pub(in crate::runtime::controller) fn drag_scrollbar_to(
        &mut self,
        point: Point,
        refresh_after_message: bool,
        metadata: ScrollUpdateMetadata,
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
            self.interaction
                .pointer
                .set_release_tombstone(capture.button, true);
            self.interaction.pointer.scroll_drag_capture = None;
            return false;
        };
        let Some(affordance) = (match capture.axis {
            ScrollbarAxis::Vertical => {
                resolve_scroll_affordance(capture.node_id, content_id, &self.layout)
            }
            ScrollbarAxis::Horizontal => {
                resolve_horizontal_scroll_affordance(capture.node_id, content_id, &self.layout)
            }
        }) else {
            self.interaction
                .pointer
                .set_release_tombstone(capture.button, true);
            self.interaction.pointer.scroll_drag_capture = None;
            return false;
        };
        let travel = match capture.axis {
            ScrollbarAxis::Vertical => {
                (affordance.track.height() - affordance.thumb.height()).max(0.0)
            }
            ScrollbarAxis::Horizontal => {
                (affordance.track.width() - affordance.thumb.width()).max(0.0)
            }
        };
        if travel <= f32::EPSILON {
            return true;
        }
        let offset_fraction = match capture.axis {
            ScrollbarAxis::Vertical => {
                let thumb_y = (point.y - affordance.thumb.height() * capture.grip_fraction)
                    .clamp(affordance.track.min.y, affordance.track.min.y + travel);
                (thumb_y - affordance.track.min.y) / travel
            }
            ScrollbarAxis::Horizontal => {
                let thumb_x = (point.x - affordance.thumb.width() * capture.grip_fraction)
                    .clamp(affordance.track.min.x, affordance.track.min.x + travel);
                (thumb_x - affordance.track.min.x) / travel
            }
        };
        let previous_offset = self.layout_state.scroll_offset(capture.node_id);
        let value = offset_fraction * affordance.max_scroll;
        let next_offset = match capture.axis {
            ScrollbarAxis::Vertical => Vector2::new(previous_offset.x, value),
            ScrollbarAxis::Horizontal => Vector2::new(value, previous_offset.y),
        };
        self.layout_state
            .scroll_offsets
            .insert(capture.node_id, next_offset);
        self.note_layout_state_mutation();
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
                    metadata,
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
            .map(|(node_id, _, _)| node_id)
    }

    pub(in crate::runtime::controller) fn scroll_viewport_at(
        &self,
        point: Point,
    ) -> Option<NodeId> {
        self.traversal
            .containers
            .scroll
            .visible()
            .iter()
            .rev()
            .copied()
            .find(|node_id| {
                let Some(policy) = self
                    .scroll_policy_for_node(*node_id)
                    .map(|container| container.scroll_policy)
                else {
                    return false;
                };
                if policy.scrollbar_visibility == crate::layout::ScrollbarVisibility::Hidden {
                    return false;
                }
                let viewport = self
                    .layout
                    .viewport_bounds
                    .get(node_id)
                    .or_else(|| self.layout.rects.get(node_id));
                viewport.is_some_and(|viewport| {
                    viewport.contains(point) && self.container_clip_contains_point(*node_id, point)
                })
            })
    }

    pub(crate) fn scrollbar_drag_active(&self) -> bool {
        self.interaction.pointer.scroll_drag_capture.is_some()
    }

    fn scrollbar_drag_capture_at(&self, point: Point) -> Option<(NodeId, f32, ScrollbarAxis)> {
        self.traversal
            .containers
            .scroll
            .visible()
            .iter()
            .rev()
            .copied()
            .find_map(|node_id| {
                let viewport = scrollbar_viewport(node_id, &self.layout)?;
                let policy = self
                    .scroll_policy_for_node(node_id)
                    .map(|c| c.scroll_policy)?;
                if policy.scrollbar_visibility == crate::layout::ScrollbarVisibility::Hidden
                    || !scrollbar_hit_viewport_contains_point(viewport, point)
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
                if policy.axes.includes_vertical()
                    && let Some(affordance) =
                        resolve_scroll_affordance(node_id, content_id, &self.layout)
                    && scrollbar_thumb_hit_rect(affordance.thumb).contains(point)
                {
                    let grip_fraction = ((point.y - affordance.thumb.min.y)
                        / affordance.thumb.height())
                    .clamp(0.0, 1.0);
                    return Some((node_id, grip_fraction, ScrollbarAxis::Vertical));
                }
                if policy.axes.includes_horizontal()
                    && let Some(affordance) =
                        resolve_horizontal_scroll_affordance(node_id, content_id, &self.layout)
                    && scrollbar_horizontal_thumb_hit_rect(affordance.thumb).contains(point)
                {
                    let grip_fraction = ((point.x - affordance.thumb.min.x)
                        / affordance.thumb.width())
                    .clamp(0.0, 1.0);
                    return Some((node_id, grip_fraction, ScrollbarAxis::Horizontal));
                }
                None
            })
    }
}

fn scrollbar_hit_viewport_contains_point(viewport: Rect, point: Point) -> bool {
    viewport.contains(point)
}

#[cfg(test)]
fn scrollbar_hit_column_contains_point(viewport: Rect, point: Point) -> bool {
    viewport.contains(point) && point.x >= viewport.max.x - SCROLLBAR_HIT_WIDTH
}

fn scrollbar_thumb_hit_rect(thumb: Rect) -> Rect {
    Rect::from_min_max(
        Point::new(thumb.max.x - SCROLLBAR_HIT_WIDTH, thumb.min.y),
        Point::new(thumb.max.x, thumb.max.y),
    )
}

fn scrollbar_horizontal_thumb_hit_rect(thumb: Rect) -> Rect {
    Rect::from_min_max(
        Point::new(thumb.min.x, thumb.max.y - SCROLLBAR_HIT_WIDTH),
        Point::new(thumb.max.x, thumb.max.y),
    )
}
