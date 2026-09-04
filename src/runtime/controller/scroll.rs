mod scrollbar;
mod wheel;

pub(crate) use wheel::WheelOrScrollRoute;

use super::SurfaceRuntime;
use crate::{
    gui::{
        input::{InputSequenceRange, InputTimestamp},
        types::{Point, Vector2},
    },
    layout::{NodeId, OverflowPolicy},
    runtime::CommandOutcome,
    runtime::RuntimeBridge,
    widgets::PointerModifiers,
};
use std::collections::BTreeSet;

/// Observational input provenance carried by a runtime-owned scroll update.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrollUpdateMetadata {
    /// Effective modifiers captured for the contributing native sample.
    pub modifiers: PointerModifiers,
    /// Opaque timestamp of the newest contributing native sample, when present.
    pub timestamp: Option<InputTimestamp>,
    /// Opaque range from the first through newest contributing native sample.
    pub sequence_range: Option<InputSequenceRange>,
}

/// Runtime-owned scroll movement reported to host bridges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollUpdate {
    /// Scroll container node that accepted the movement.
    pub node_id: NodeId,
    /// Pointer position that selected the scroll container.
    pub position: Point,
    /// Requested logical scroll-offset delta for the components delivered by
    /// this update. Positive `x`/`y` increases the corresponding offset, so
    /// layout renders content left/up. Ordinary native coalesced fallback
    /// reports one selected logical-pixel axis and has no phase or unit field.
    pub delta: Vector2,
    /// Scroll offset before the movement.
    pub previous_offset: Vector2,
    /// Scroll offset after layout clamping.
    pub offset: Vector2,
    /// Logical viewport size of the scroll container that accepted the update.
    pub viewport: Vector2,
    /// Observational provenance for the input that caused this update.
    pub metadata: ScrollUpdateMetadata,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Scroll the topmost scroll container under `point` by an offset delta.
    ///
    /// Positive `x`/`y` increases the corresponding logical offset. The
    /// controller applies `current + delta` and clamps the result through
    /// layout. Returns `true` when a scroll container accepted the delta.
    pub fn scroll_at(&mut self, point: Point, delta: Vector2) -> bool {
        self.scroll_at_with_refresh_and_metadata(
            point,
            delta,
            ScrollUpdateMetadata::default(),
            true,
        )
    }

    #[allow(dead_code)]
    pub(in crate::runtime::controller) fn scroll_at_with_refresh(
        &mut self,
        point: Point,
        delta: Vector2,
        refresh_after_message: bool,
    ) -> bool {
        self.scroll_at_with_refresh_and_metadata(
            point,
            delta,
            ScrollUpdateMetadata::default(),
            refresh_after_message,
        )
    }

    pub(crate) fn scroll_container_accepts_wheel_at(&self, point: Point) -> bool {
        self.scroll_container_at(point).is_some()
    }

    pub(in crate::runtime::controller) fn scroll_at_with_refresh_and_metadata(
        &mut self,
        point: Point,
        delta: Vector2,
        metadata: ScrollUpdateMetadata,
        refresh_after_message: bool,
    ) -> bool {
        let candidates: Vec<_> = self
            .traversal
            .containers
            .scroll
            .visible()
            .iter()
            .rev()
            .copied()
            .collect();
        let mut remaining = delta;
        let mut accepted = false;
        for node_id in candidates {
            if !self.scroll_container_accepts_point(node_id, point) {
                continue;
            }
            accepted = true;
            let Some(policy) = self
                .scroll_policy_for_node(node_id)
                .map(|container| container.scroll_policy)
            else {
                continue;
            };
            let mut effective = remaining;
            let allows_horizontal =
                policy.axes.includes_horizontal() || policy.allows_legacy_horizontal();
            if !allows_horizontal {
                effective.x = 0.0;
            }
            if !policy.axes.includes_vertical() {
                effective.y = 0.0;
            }
            match policy.axis_lock {
                crate::layout::ScrollAxisLock::Horizontal
                    if effective.x.abs() >= effective.y.abs() =>
                {
                    effective.y = 0.0
                }
                crate::layout::ScrollAxisLock::Horizontal => effective.x = 0.0,
                crate::layout::ScrollAxisLock::Vertical
                    if effective.y.abs() >= effective.x.abs() =>
                {
                    effective.x = 0.0
                }
                crate::layout::ScrollAxisLock::Vertical => effective.y = 0.0,
                crate::layout::ScrollAxisLock::None => {}
            }
            let current = self.layout_state.scroll_offset(node_id);
            let requested = Vector2::new(
                (current.x + effective.x).max(0.0),
                (current.y + effective.y).max(0.0),
            );
            if requested == current {
                if !policy.chaining {
                    return true;
                }
                continue;
            }
            self.layout_state.scroll_offsets.insert(node_id, requested);
            self.note_layout_state_mutation();
            self.relayout_current_surface();
            let offset = self.layout_state.scroll_offset(node_id);
            if offset != current {
                let consumed = Vector2::new(offset.x - current.x, offset.y - current.y);
                let mut residual = remaining;
                if allows_horizontal {
                    residual.x -= consumed.x;
                }
                if policy.axes.includes_vertical() {
                    residual.y -= consumed.y;
                }
                let viewport = self
                    .layout
                    .rects
                    .get(&node_id)
                    .map(|rect| Vector2::new(rect.width(), rect.height()))
                    .unwrap_or_default();
                self.report_scroll_update_with_refresh(
                    ScrollUpdate {
                        node_id,
                        position: point,
                        delta: effective,
                        previous_offset: current,
                        offset,
                        viewport,
                        metadata,
                    },
                    refresh_after_message,
                );
                if !policy.chaining
                    || (residual.x.abs() <= f32::EPSILON && residual.y.abs() <= f32::EPSILON)
                {
                    return true;
                }
                remaining = residual;
                continue;
            }
            // A boundary may offer its unconsumed delta to the next ancestor.
            if !policy.chaining {
                return true;
            }
        }
        accepted
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
                self.dispatch_message_inner_deferred_refresh(message, &mut outcome);
                deferred_surface_refresh = outcome.surface_refresh_requested;
                self.pending_input_command_outcome.merge(outcome);
            }
            self.repaint_requested |= !deferred_surface_refresh;
            return;
        }
        if let Some(command) = self.host_scroll_updated(update) {
            if refresh_after_message {
                let outcome = self.execute_command(command);
                if !outcome.surface_refresh_requested {
                    self.refresh();
                }
            } else {
                let mut outcome = CommandOutcome::default();
                if command.requires_fresh_surface_before_dispatch() {
                    outcome.surface_refresh_requested = true;
                }
                self.execute_command_inner_deferred_refresh(command, &mut outcome);
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
            .find(|node_id| self.scroll_container_accepts_point(*node_id, point))
    }

    pub(in crate::runtime::controller) fn scroll_policy_for_node(
        &self,
        node_id: NodeId,
    ) -> Option<&crate::layout::ContainerPolicy> {
        fn find<'a>(
            node: &'a crate::layout::LayoutNode,
            id: NodeId,
        ) -> Option<&'a crate::layout::ContainerPolicy> {
            let crate::layout::LayoutNode::Container(container) = node else {
                return None;
            };
            if container.id == id {
                return Some(&container.policy);
            }
            container
                .children
                .iter()
                .find_map(|child| find(&child.child, id))
        }
        find(&self.layout_root, node_id)
    }

    pub(in crate::runtime::controller) fn scroll_container_accepts_point(
        &self,
        node_id: NodeId,
        point: Point,
    ) -> bool {
        self.layout
            .rects
            .get(&node_id)
            .is_some_and(|rect| rect.contains(point))
            && self
                .layout
                .overflow_flags
                .get(&node_id)
                .is_some_and(|overflow| {
                    overflow.policy == OverflowPolicy::Scroll && (overflow.x || overflow.y)
                })
            && self.container_clip_contains_point(node_id, point)
    }

    pub(in crate::runtime::controller) fn scroll_keyboard_fallback(
        &mut self,
        key: crate::widgets::WidgetKey,
    ) -> bool {
        let Some(widget_id) = self.focused_widget() else {
            return false;
        };
        let mut candidates = self
            .traversal
            .widgets
            .paths
            .clip_ancestors
            .get(&widget_id)
            .map(|path| path.as_slice().iter().rev().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        if candidates.is_empty() {
            candidates.extend(
                self.traversal
                    .containers
                    .scroll
                    .visible()
                    .iter()
                    .rev()
                    .copied(),
            );
        }
        let mut seen = BTreeSet::new();
        for node_id in candidates {
            if !seen.insert(node_id) {
                continue;
            }
            let Some(policy) = self
                .scroll_policy_for_node(node_id)
                .map(|c| c.scroll_policy)
            else {
                continue;
            };
            let Some(viewport) = self.layout.viewport_bounds.get(&node_id).copied() else {
                continue;
            };
            let Some(content_id) = self
                .traversal
                .containers
                .scroll_content_by_container
                .get(&node_id)
                .copied()
            else {
                continue;
            };
            let Some(content) = self.layout.rects.get(&content_id).copied() else {
                continue;
            };
            let current = self.layout_state.scroll_offset(node_id);
            let page = policy.page_fraction.clamp(0.1, 4.0);
            let mut next = current;
            match key {
                crate::widgets::WidgetKey::PageUp => {
                    if policy.axes.includes_vertical() {
                        next.y -= viewport.height().max(1.0) * page;
                    }
                }
                crate::widgets::WidgetKey::PageDown => {
                    if policy.axes.includes_vertical() {
                        next.y += viewport.height().max(1.0) * page;
                    }
                }
                crate::widgets::WidgetKey::Home => {
                    if policy.axes.includes_vertical() {
                        next.y = 0.0;
                    }
                    if policy.axes.includes_horizontal() {
                        next.x = 0.0;
                    }
                }
                crate::widgets::WidgetKey::End => {
                    if policy.axes.includes_vertical() {
                        next.y = (content.height() - viewport.height()).max(0.0);
                    }
                    if policy.axes.includes_horizontal() {
                        next.x = (content.width() - viewport.width()).max(0.0);
                    }
                }
                _ => return false,
            }
            if next == current {
                continue;
            }
            self.scroll_to_offset(node_id, next);
            return true;
        }
        false
    }
}
