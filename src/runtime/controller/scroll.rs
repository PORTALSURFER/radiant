mod scrollbar;
mod wheel;

pub(crate) use wheel::WheelOrScrollRoute;

use super::SurfaceRuntime;
use crate::{
    gui::types::{Point, Vector2},
    layout::{NodeId, OverflowPolicy},
    runtime::CommandOutcome,
    runtime::RuntimeBridge,
};

/// Runtime-owned scroll movement reported to host bridges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollUpdate {
    /// Scroll container node that accepted the movement.
    pub node_id: NodeId,
    /// Pointer position that selected the scroll container.
    pub position: Point,
    /// Requested logical scroll delta.
    pub delta: Vector2,
    /// Scroll offset before the movement.
    pub previous_offset: Vector2,
    /// Scroll offset after layout clamping.
    pub offset: Vector2,
    /// Logical viewport size of the scroll container that accepted the update.
    pub viewport: Vector2,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Scroll the topmost scroll container under `point`.
    ///
    /// Returns `true` when a scroll container accepted the delta.
    pub fn scroll_at(&mut self, point: Point, delta: Vector2) -> bool {
        self.scroll_at_with_refresh(point, delta, true)
    }

    pub(in crate::runtime::controller) fn scroll_at_deferred_refresh(
        &mut self,
        point: Point,
        delta: Vector2,
    ) -> bool {
        self.scroll_at_with_refresh(point, delta, false)
    }

    pub(crate) fn scroll_container_accepts_wheel_at(&self, point: Point) -> bool {
        self.scroll_container_at(point).is_some()
    }

    fn scroll_at_with_refresh(
        &mut self,
        point: Point,
        delta: Vector2,
        refresh_after_message: bool,
    ) -> bool {
        let Some(node_id) = self.scroll_container_at(point) else {
            return false;
        };
        let current = self.layout_state.scroll_offset(node_id);
        let requested = Vector2::new(
            (current.x + delta.x).max(0.0),
            (current.y + delta.y).max(0.0),
        );
        if current == Vector2::default() && requested == current {
            return true;
        }
        self.layout_state.scroll_offsets.insert(node_id, requested);
        self.relayout_current_surface();
        let offset = self.layout_state.scroll_offset(node_id);
        if offset == current {
            return true;
        }
        let viewport = self
            .layout
            .rects
            .get(&node_id)
            .map(|rect| Vector2::new(rect.width(), rect.height()))
            .unwrap_or_default();
        let update = ScrollUpdate {
            node_id,
            position: point,
            delta,
            previous_offset: current,
            offset,
            viewport,
        };
        self.report_scroll_update_with_refresh(update, refresh_after_message);
        true
    }

    pub(super) fn report_scroll_update(&mut self, update: ScrollUpdate) {
        self.report_scroll_update_with_refresh(update, true);
    }

    pub(super) fn report_scroll_update_with_refresh(
        &mut self,
        update: ScrollUpdate,
        refresh_after_message: bool,
    ) {
        let mut deferred_surface_refresh = false;
        if let Some(message) = self.surface.root().scroll_message(update) {
            if refresh_after_message {
                let outcome = self.execute_command(crate::runtime::Command::Message(message));
                if !outcome.surface_refresh_requested {
                    self.refresh();
                }
            } else {
                let mut outcome = CommandOutcome::default();
                self.execute_command_inner(crate::runtime::Command::Message(message), &mut outcome);
                deferred_surface_refresh = outcome.surface_refresh_requested;
                self.pending_input_command_outcome.merge(outcome);
            }
            self.repaint_requested |= !deferred_surface_refresh;
            return;
        }
        if let Some(command) = self.bridge.scroll_updated(update) {
            if refresh_after_message {
                let outcome = self.execute_command(command);
                if !outcome.surface_refresh_requested {
                    self.refresh();
                }
            } else {
                let mut outcome = CommandOutcome::default();
                self.execute_command_inner(command, &mut outcome);
                deferred_surface_refresh = outcome.surface_refresh_requested;
                self.pending_input_command_outcome.merge(outcome);
            }
            self.repaint_requested |= !deferred_surface_refresh;
            return;
        }
        self.repaint_requested = true;
    }

    fn scroll_container_at(&self, point: Point) -> Option<NodeId> {
        self.traversal
            .containers
            .scroll
            .visible()
            .iter()
            .rev()
            .copied()
            .find(|node_id| {
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
}
