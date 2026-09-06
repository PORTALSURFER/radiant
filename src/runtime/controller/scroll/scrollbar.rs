//! Scrollbar hit testing and drag routing.

use super::{
    super::{ScrollDragCapture, SurfaceRuntime},
    ScrollEditBatch, ScrollUpdate, ScrollUpdateMetadata,
};
use crate::runtime::controller::interaction_state::ScrollbarAxis;
use crate::runtime::{RuntimeBridge, paint::resolve_scroll_affordance};
use crate::{
    gui::types::{Point, Rect, Vector2},
    layout::NodeId,
    runtime::paint::{
        resolve_horizontal_scroll_affordance, scrollbar_viewport, scrollbar_visibility_allows,
    },
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
        metadata: ScrollUpdateMetadata,
    ) -> bool {
        let Some((node_id, grip_fraction, axis)) = self.scrollbar_drag_capture_at(point) else {
            return false;
        };
        let Some(content_id) = self
            .traversal
            .containers
            .scroll_content_by_container
            .get(&node_id)
            .copied()
        else {
            return false;
        };
        let Some(viewport) = self.layout.rects.get(&node_id).copied() else {
            return false;
        };
        let Some(affordance) = (match axis {
            ScrollbarAxis::Vertical => resolve_scroll_affordance(node_id, content_id, &self.layout),
            ScrollbarAxis::Horizontal => {
                resolve_horizontal_scroll_affordance(node_id, content_id, &self.layout)
            }
        }) else {
            return false;
        };
        let Some(policy) = self
            .scroll_policy_for_node(node_id)
            .map(|container| container.scroll_policy)
        else {
            return false;
        };
        let start_offset = self.layout_state.scroll_offset(node_id);
        let capture = ScrollDragCapture {
            node_id,
            grip_fraction,
            button,
            axis,
            start_offset,
            edit: crate::widgets::EditEvent::begin(
                start_offset,
                scroll_pointer_provenance(metadata),
            ),
            edit_started: false,
            content_id,
            viewport,
            max_scroll: affordance.max_scroll,
            policy,
            last_position: point,
        };
        self.interaction.pointer.scroll_drag_capture = Some(capture);
        self.interaction.hover.scroll_affordance = Some(node_id);
        self.note_scroll_visibility_mutation();
        self.repaint_requested = true;
        true
    }

    pub(in crate::runtime::controller) fn begin_scrollbar_edit(&mut self) {
        let Some(candidate) = self.interaction.pointer.scroll_drag_capture else {
            return;
        };
        if !self.scrollbar_edit_geometry_matches(candidate)
            || self.layout_state.scroll_offset(candidate.node_id) != candidate.edit.value
        {
            self.interaction.pointer.scroll_drag_capture = None;
            self.interaction
                .pointer
                .set_release_tombstone(candidate.button, true);
            return;
        }
        let Some(capture) = self.interaction.pointer.scroll_drag_capture.as_mut() else {
            return;
        };
        if capture.edit_started {
            return;
        }
        capture.edit_started = true;
        let batch = ScrollEditBatch::new(capture.node_id, &[capture.edit], None);
        if let Some(batch) = batch {
            self.report_scroll_edit(batch, true);
        }
    }

    pub(in crate::runtime::controller) fn drag_scrollbar_to(
        &mut self,
        point: Point,
        refresh_after_message: bool,
        metadata: ScrollUpdateMetadata,
    ) -> bool {
        if self.interaction.pointer.scroll_drag_capture.is_none() {
            return false;
        }
        let update = self.update_scrollbar_offset(point, metadata);
        if let Some(update) = update
            && let Some(capture) = self.interaction.pointer.scroll_drag_capture
            && let Some(batch) =
                ScrollEditBatch::new(capture.node_id, &[capture.edit], Some(update))
        {
            self.report_scroll_edit(batch, refresh_after_message);
        }
        self.repaint_requested |= refresh_after_message;
        self.interaction.pointer.scroll_drag_capture.is_some()
    }

    fn update_scrollbar_offset(
        &mut self,
        point: Point,
        metadata: ScrollUpdateMetadata,
    ) -> Option<ScrollUpdate> {
        let capture = self.interaction.pointer.scroll_drag_capture?;
        if !point.x.is_finite() || !point.y.is_finite() {
            return None;
        }
        if !self.scrollbar_edit_geometry_matches(capture)
            || self.layout_state.scroll_offset(capture.node_id) != capture.edit.value
        {
            self.interaction.pointer.scroll_drag_capture = None;
            self.interaction
                .pointer
                .set_release_tombstone(capture.button, true);
            self.cancel_scrollbar_edit(capture, false);
            return None;
        }
        let affordance = match capture.axis {
            ScrollbarAxis::Vertical => {
                resolve_scroll_affordance(capture.node_id, capture.content_id, &self.layout)
            }
            ScrollbarAxis::Horizontal => resolve_horizontal_scroll_affordance(
                capture.node_id,
                capture.content_id,
                &self.layout,
            ),
        }?;
        let travel = match capture.axis {
            ScrollbarAxis::Vertical => affordance.track.height() - affordance.thumb.height(),
            ScrollbarAxis::Horizontal => affordance.track.width() - affordance.thumb.width(),
        };
        if travel <= f32::EPSILON {
            return None;
        }
        let fraction = match capture.axis {
            ScrollbarAxis::Vertical => {
                (point.y
                    - affordance.thumb.height() * capture.grip_fraction
                    - affordance.track.min.y)
                    / travel
            }
            ScrollbarAxis::Horizontal => {
                (point.x
                    - affordance.thumb.width() * capture.grip_fraction
                    - affordance.track.min.x)
                    / travel
            }
        }
        .clamp(0.0, 1.0);
        let previous_offset = self.layout_state.scroll_offset(capture.node_id);
        let next = match capture.axis {
            ScrollbarAxis::Vertical => {
                Vector2::new(previous_offset.x, fraction * affordance.max_scroll)
            }
            ScrollbarAxis::Horizontal => {
                Vector2::new(fraction * affordance.max_scroll, previous_offset.y)
            }
        };
        if next == previous_offset {
            return None;
        }
        self.layout_state
            .scroll_offsets
            .insert(capture.node_id, next);
        self.note_layout_state_mutation();
        self.relayout_current_surface();
        let offset = self.layout_state.scroll_offset(capture.node_id);
        if offset == previous_offset {
            return None;
        }
        let event = capture
            .edit
            .update(offset, scroll_pointer_provenance(metadata))?;
        if let Some(live) = self.interaction.pointer.scroll_drag_capture.as_mut() {
            live.edit = event;
            live.last_position = point;
        }
        self.interaction.hover.scroll_affordance = Some(capture.node_id);
        Some(self.scrollbar_offset_update(capture, point, previous_offset, offset, metadata))
    }

    pub(in crate::runtime::controller) fn scrollbar_edit_geometry_matches(
        &self,
        capture: ScrollDragCapture,
    ) -> bool {
        if self
            .scroll_policy_for_node(capture.node_id)
            .map(|container| container.scroll_policy)
            != Some(capture.policy)
        {
            return false;
        }
        if self
            .traversal
            .containers
            .scroll_content_by_container
            .get(&capture.node_id)
            != Some(&capture.content_id)
            || self.layout.rects.get(&capture.node_id) != Some(&capture.viewport)
        {
            return false;
        }
        let affordance = match capture.axis {
            ScrollbarAxis::Vertical => {
                resolve_scroll_affordance(capture.node_id, capture.content_id, &self.layout)
            }
            ScrollbarAxis::Horizontal => resolve_horizontal_scroll_affordance(
                capture.node_id,
                capture.content_id,
                &self.layout,
            ),
        };
        affordance.is_some_and(|affordance| affordance.max_scroll == capture.max_scroll)
    }

    fn scrollbar_offset_update(
        &self,
        capture: ScrollDragCapture,
        point: Point,
        previous_offset: Vector2,
        offset: Vector2,
        metadata: ScrollUpdateMetadata,
    ) -> ScrollUpdate {
        ScrollUpdate {
            node_id: capture.node_id,
            position: point,
            delta: Vector2::new(offset.x - previous_offset.x, offset.y - previous_offset.y),
            previous_offset,
            offset,
            viewport: Vector2::new(capture.viewport.width(), capture.viewport.height()),
            metadata,
        }
    }

    pub(in crate::runtime::controller) fn finish_scrollbar_edit(
        &mut self,
        point: Point,
        metadata: ScrollUpdateMetadata,
    ) {
        let Some(previous) = self.interaction.pointer.scroll_drag_capture else {
            return;
        };
        if !point.x.is_finite() || !point.y.is_finite() {
            self.interaction.pointer.scroll_drag_capture = None;
            self.cancel_scrollbar_edit(previous, true);
            return;
        }
        let update = self.update_scrollbar_offset(point, metadata);
        let Some(capture) = self.interaction.pointer.scroll_drag_capture else {
            return;
        };
        if capture.edit.transaction != previous.edit.transaction {
            return;
        }
        self.interaction.pointer.scroll_drag_capture = None;
        let offset = self.layout_state.scroll_offset(capture.node_id);
        let Some(commit) = capture
            .edit
            .commit(offset, scroll_pointer_provenance(metadata))
        else {
            return;
        };
        let batch = if update.is_some() {
            ScrollEditBatch::new(capture.node_id, &[capture.edit, commit], update)
        } else {
            ScrollEditBatch::new(capture.node_id, &[commit], None)
        };
        if let Some(batch) = batch {
            self.report_scroll_edit(batch, true);
        }
        if offset != capture.start_offset
            && self.layout_state.scroll_offset(capture.node_id) == offset
            && self.interaction.pointer.scroll_drag_capture.is_none()
        {
            self.emit_scroll_offset_settled(capture.node_id, offset, true);
        }
    }

    pub(in crate::runtime::controller) fn cancel_scrollbar_edit(
        &mut self,
        capture: ScrollDragCapture,
        restore: bool,
    ) {
        if !capture.edit_started {
            return;
        }
        let previous_offset = self.layout_state.scroll_offset(capture.node_id);
        let restore = restore
            && self.scrollbar_edit_geometry_matches(capture)
            && previous_offset == capture.edit.value;
        let mut update = None;
        if restore && previous_offset != capture.start_offset {
            self.layout_state
                .scroll_offsets
                .insert(capture.node_id, capture.start_offset);
            self.note_layout_state_mutation();
            self.relayout_current_surface();
            let offset = self.layout_state.scroll_offset(capture.node_id);
            update = Some(self.scrollbar_offset_update(
                capture,
                capture.last_position,
                previous_offset,
                offset,
                ScrollUpdateMetadata::default(),
            ));
        }
        let Some(cancel) = capture
            .edit
            .cancel(scroll_pointer_provenance(ScrollUpdateMetadata::default()))
        else {
            return;
        };
        if let Some(batch) = ScrollEditBatch::new(capture.node_id, &[cancel], update) {
            self.report_scroll_edit(batch, true);
        }
        if update.is_some()
            && self.layout_state.scroll_offset(capture.node_id) == capture.start_offset
            && self.interaction.pointer.scroll_drag_capture.is_none()
        {
            self.emit_scroll_offset_settled(capture.node_id, capture.start_offset, true);
        }
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
        let auto_visible = self.scroll_auto_visibility();
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
                if !scrollbar_visibility_allows(policy.scrollbar_visibility, node_id, &auto_visible)
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
                if policy.configured_axes().includes_vertical()
                    && let Some(affordance) =
                        resolve_scroll_affordance(node_id, content_id, &self.layout)
                    && scrollbar_thumb_hit_rect(affordance.thumb).contains(point)
                {
                    let grip_fraction = ((point.y - affordance.thumb.min.y)
                        / affordance.thumb.height())
                    .clamp(0.0, 1.0);
                    return Some((node_id, grip_fraction, ScrollbarAxis::Vertical));
                }
                if policy.configured_axes().includes_horizontal()
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

fn scroll_pointer_provenance(
    metadata: ScrollUpdateMetadata,
) -> crate::widgets::InteractionProvenance {
    crate::widgets::InteractionProvenance::Pointer {
        modifiers: metadata.modifiers,
        timestamp: metadata.timestamp,
        sequence_range: metadata.sequence_range,
    }
}
