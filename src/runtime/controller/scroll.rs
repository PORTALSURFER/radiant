mod scrollbar;
mod wheel;

use super::*;

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
        self.report_scroll_update(update);
        true
    }

    pub(super) fn report_scroll_update(&mut self, update: ScrollUpdate) {
        if let Some(command) = self.bridge.scroll_updated(update) {
            let outcome = self.execute_command(command);
            if !outcome.surface_refresh_requested {
                self.refresh();
            }
            self.repaint_requested = true;
        }
    }

    fn scroll_container_at(&self, point: Point) -> Option<NodeId> {
        self.scroll_containers
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
