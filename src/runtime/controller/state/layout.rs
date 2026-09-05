use super::super::{SurfaceRuntime, SurfaceTraversalIndex};
use crate::gui::types::{Point, Rect};
use crate::{
    gui::layout_core::{ScrollRuntimeState, resolve_scroll_alignment},
    gui::types::Vector2,
    layout::LayoutDiagnosticCode,
    layout::{LayoutNode, ScrollEdge, ScrollTarget},
    runtime::RuntimeBridge,
};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::runtime::controller) fn relayout(&mut self) {
        let mut traversal = self.take_reusable_traversal_index(true);
        let layout_root = self.surface.runtime_projection_reusing_with_scratch(
            &mut traversal,
            &mut self.scratch.projection_scroll_stack,
            &mut self.scratch.projection_child_path,
            &mut self.scratch.projection_source,
        );
        self.replace_layout_root(layout_root);
        self.relayout_with_traversal(traversal);
        self.install_declarative_owner_projection();
    }

    pub(in crate::runtime::controller) fn relayout_current_surface(&mut self) {
        let traversal = self.take_reusable_traversal_index(true);
        self.relayout_with_traversal(traversal);
    }

    pub(in crate::runtime::controller) fn queue_current_surface_relayout(&mut self) {
        self.pending_current_surface_relayout = true;
        self.repaint_requested = true;
    }

    pub(in crate::runtime::controller) fn service_pending_current_surface_relayout(&mut self) {
        if self.servicing_current_surface_relayout {
            return;
        }
        self.servicing_current_surface_relayout = true;
        for _ in 0..2 {
            if !self.pending_current_surface_relayout {
                break;
            }
            self.pending_current_surface_relayout = false;
            self.relayout_current_surface();
            self.repaint_requested = true;
        }
        self.servicing_current_surface_relayout = false;
    }

    pub(in crate::runtime::controller) fn install_traversal_with_candidate(
        &mut self,
        traversal: SurfaceTraversalIndex<Message>,
        candidate: super::super::layout_state::RuntimeLayoutContainerStateCandidate,
    ) {
        let candidate_source_present = candidate.source_present();
        let candidate_mutates_values_or_identity = candidate.mutates_values_or_identity();
        let accepted_source_present = self.mounted_layout_source_present;
        self.install_traversal_index(traversal);
        self.refresh_visible_traversal_orders();
        self.commit_layout_container_state_candidate(candidate);
        if !candidate_mutates_values_or_identity
            && candidate_source_present != accepted_source_present
        {
            self.note_mounted_layout_source_mutation(true);
        }
        self.mounted_layout_source_present = candidate_source_present;
        self.traversal
            .containers
            .bind_committed_mounted_state_ids(&self.interaction.layout_state);
        self.traversal
            .containers
            .rebuild_split_pane_ratio_action_authorities(&self.interaction.layout_state);
        self.traversal
            .containers
            .rebuild_split_pane_separator_projections(&self.interaction.layout_state);
        self.traversal
            .rebuild_mixed_focus_order(self.lifecycle_phase(), &self.interaction.layout_state);
        // Separator focus is revalidated only after the committed mounted
        // state and its projections are both installed. Prepared candidates
        // therefore cannot mutate the active owner.
        self.revalidate_focus_owner();
    }

    pub(in crate::runtime::controller) fn relayout_with_traversal(
        &mut self,
        traversal: SurfaceTraversalIndex<Message>,
    ) {
        self.sync_declarative_scroll_inputs();
        let candidate = self.prepare_layout_container_state_candidate(&traversal);
        let container_state_source = self.interaction.layout_state.read_source(&candidate);
        self.layout_engine
            .layout_with_state_and_direction_and_source_into(
                &self.layout_root,
                self.viewport,
                &self.layout_state,
                self.layout_debug_options,
                self.surface.resolved_environment().writing_direction(),
                Some(&container_state_source),
                &mut self.layout,
            );
        self.install_traversal_with_candidate(traversal, candidate);
        self.sync_scroll_offsets();
        let settled = self.apply_declarative_scroll_requests();
        if !settled.is_empty() {
            let traversal = self.take_reusable_traversal_index(true);
            let candidate = self.prepare_layout_container_state_candidate(&traversal);
            let container_state_source = self.interaction.layout_state.read_source(&candidate);
            self.layout_engine.layout_with_state_and_source_into(
                &self.layout_root,
                self.viewport,
                &self.layout_state,
                self.layout_debug_options,
                Some(&container_state_source),
                &mut self.layout,
            );
            self.install_traversal_with_candidate(traversal, candidate);
            self.sync_scroll_offsets();
            for (node_id, previous_offset) in settled {
                let offset = self.layout_state.scroll_offset(node_id);
                if offset != previous_offset
                    && let Some(message) = self.surface.root().offset_settled(node_id, offset)
                {
                    let outcome = self.execute_command(crate::runtime::Command::Message(message));
                    if !outcome.surface_refresh_requested {
                        self.refresh();
                    }
                }
            }
        }
        self.record_completed_layout();
    }

    /// Apply mount-seeded and controlled declarative offsets before layout.
    /// The state map is runtime-owned; declarative refreshes never overwrite a
    /// live offset unless a strictly newer controlled generation is supplied.
    fn sync_declarative_scroll_inputs(&mut self) {
        let mut declarations = Vec::new();
        collect_scroll_declarations(&self.layout_root, &mut declarations);
        let live: std::collections::BTreeSet<_> =
            declarations.iter().map(|(id, _, _)| *id).collect();
        if self
            .interaction
            .wheel
            .pending_scroll_settlement
            .iter()
            .any(|(id, _)| !live.contains(id))
        {
            self.interaction
                .wheel
                .pending_scroll_settlement
                .retain(|(id, _)| live.contains(id));
            if self.interaction.wheel.pending_scroll_settlement.is_empty() {
                self.interaction.wheel.scroll_settlement_deadline = None;
            }
        }
        self.layout_state
            .scroll_runtime
            .retain(|id, _| live.contains(id));
        self.layout_state
            .scroll_offsets
            .retain(|id, _| live.contains(id));
        sync_declarative_scroll_inputs_for(
            declarations,
            &mut self.layout_state,
            self.layout_state_generation,
        );
    }

    /// Resolve finite committed rectangles and edges and consume each request
    /// once. Keyed requests remain unavailable until a committed materialized
    /// key index is provided by the virtual-layout consumer.
    fn apply_declarative_scroll_requests(&mut self) -> Vec<(crate::layout::NodeId, Vector2)> {
        let mut requests = Vec::new();
        collect_scroll_requests(&self.layout_root, &mut requests);
        apply_declarative_scroll_requests_for(
            requests,
            &self.layout,
            &mut self.layout_state,
            |owner, key| self.virtual_layout.materialized_key_payload(owner, key),
        )
    }
}

pub(in crate::runtime::controller) fn sync_declarative_scroll_inputs_for(
    declarations: Vec<(
        crate::layout::NodeId,
        crate::layout::NodeId,
        &crate::layout::ContainerPolicy,
    )>,
    layout_state: &mut crate::gui::layout_core::LayoutState,
    layout_state_generation: u64,
) {
    for (id, _content_id, policy) in declarations {
        let entry = layout_state
            .scroll_runtime
            .entry(id)
            .or_insert_with(|| ScrollRuntimeState {
                mount_generation: layout_state_generation.max(1),
                ..ScrollRuntimeState::default()
            });
        if !entry.initial_seeded {
            if let Some(offset) = policy
                .initial_offset
                .filter(|offset| offset.x.is_finite() && offset.y.is_finite())
            {
                layout_state.scroll_offsets.insert(
                    id,
                    policy
                        .scroll_policy
                        .project_offset_axes(Vector2::new(offset.x.max(0.0), offset.y.max(0.0))),
                );
            }
            entry.initial_seeded = true;
        }
        if let Some(controlled) = policy.controlled_offset.filter(|value| {
            value.generation() != u64::MAX
                && value.value().x.is_finite()
                && value.value().y.is_finite()
        }) && entry
            .controlled_generation
            .is_none_or(|generation| controlled.generation() > generation)
        {
            let value = *controlled.value();
            layout_state.scroll_offsets.insert(
                id,
                policy
                    .scroll_policy
                    .project_offset_axes(Vector2::new(value.x.max(0.0), value.y.max(0.0))),
            );
            entry.controlled_generation = Some(controlled.generation());
        }
        if let Some(current) = layout_state.scroll_offsets.get(&id).copied() {
            let projected = policy.scroll_policy.project_offset_axes(current);
            if projected != current {
                layout_state.scroll_offsets.insert(id, projected);
            }
        }
    }
}

pub(in crate::runtime::controller) fn apply_declarative_scroll_requests_for(
    requests: Vec<(
        crate::layout::NodeId,
        &crate::layout::ContainerPolicy,
        crate::layout::NodeId,
    )>,
    layout: &crate::gui::layout_core::LayoutOutput,
    layout_state: &mut crate::gui::layout_core::LayoutState,
    mut materialized_key_payload: impl FnMut(
        crate::layout::NodeId,
        &crate::layout::VirtualLayoutItemKey,
    )
        -> Option<(crate::layout::NodeId, crate::layout::NodeId)>,
) -> Vec<(crate::layout::NodeId, Vector2)> {
    let mut changed = Vec::new();
    for (id, policy, child_id) in requests {
        let Some(request) = policy.scroll_request.as_ref() else {
            continue;
        };
        let already_consumed = layout_state
            .scroll_runtime
            .get(&id)
            .and_then(|state| state.request_generation)
            .is_some_and(|generation| request.generation <= generation);
        if request.generation == u64::MAX || already_consumed {
            continue;
        }
        // A mounted owner consumes every valid generation at the admission
        // boundary.  Target lookup and geometry are deliberately after this
        // fence: an unavailable virtual item or malformed target must not be
        // retried by a later reprojection with the same generation.
        let Some(state) = layout_state.scroll_runtime.get_mut(&id) else {
            continue;
        };
        state.request_generation = Some(request.generation);
        let Some(viewport) = layout
            .viewport_bounds
            .get(&id)
            .or_else(|| layout.rects.get(&id))
            .copied()
        else {
            continue;
        };
        let Some(content) = layout.rects.get(&child_id).copied() else {
            continue;
        };
        let current = layout_state.scroll_offset(id);
        let (target_x, target_y, target_w, target_h) = match &request.target {
            ScrollTarget::Keyed(key) => {
                let Some((owner, payload_id)) = materialized_key_payload(id, key) else {
                    continue;
                };
                if owner != id || !layout.rects.contains_key(&payload_id) {
                    continue;
                }
                let Some(rect) = layout.rects.get(&payload_id).copied() else {
                    continue;
                };
                if !rect.is_finite() || !rect.has_finite_positive_area() {
                    continue;
                }
                (
                    rect.min.x - content.min.x,
                    rect.min.y - content.min.y,
                    rect.width(),
                    rect.height(),
                )
            }
            ScrollTarget::Rect(rect) if rect.has_finite_positive_area() => {
                (rect.min.x, rect.min.y, rect.width(), rect.height())
            }
            ScrollTarget::Edge(edge) => match edge {
                ScrollEdge::Top => (current.x, 0.0, 0.0, 0.0),
                ScrollEdge::Bottom => (current.x, content.height(), 0.0, 0.0),
                ScrollEdge::Left => (0.0, current.y, 0.0, 0.0),
                ScrollEdge::Right => (content.width(), current.y, 0.0, 0.0),
                ScrollEdge::Start => (
                    if policy.scroll_policy.allows_horizontal() {
                        0.0
                    } else {
                        current.x
                    },
                    if policy.scroll_policy.axes.includes_vertical() {
                        0.0
                    } else {
                        current.y
                    },
                    0.0,
                    0.0,
                ),
                ScrollEdge::End => (
                    if policy.scroll_policy.allows_horizontal() {
                        content.width()
                    } else {
                        current.x
                    },
                    if policy.scroll_policy.axes.includes_vertical() {
                        content.height()
                    } else {
                        current.y
                    },
                    0.0,
                    0.0,
                ),
            },
            _ => continue,
        };
        let mut next = current;
        if policy.scroll_policy.allows_horizontal() {
            next.x = resolve_scroll_alignment(
                current.x,
                viewport.width(),
                target_x,
                target_x + target_w,
                request.alignment,
            );
        }
        if policy.scroll_policy.axes.includes_vertical() {
            next.y = resolve_scroll_alignment(
                current.y,
                viewport.height(),
                target_y,
                target_y + target_h,
                request.alignment,
            );
        }
        if next.x.is_finite() && next.y.is_finite() && next != current {
            layout_state
                .scroll_offsets
                .insert(id, Vector2::new(next.x.max(0.0), next.y.max(0.0)));
            changed.push((id, current));
        }
    }
    changed
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::runtime::controller) fn record_completed_layout(&mut self) {
        self.external_layout_dirty = false;
        self.completed_layout = Some(super::super::CompletedLayoutContext {
            viewport: effective_layout_viewport(self.viewport),
            window_environment: self.window_environment,
            direction: self.surface.resolved_environment().writing_direction(),
            layout_state_generation: self.layout_state_generation,
            layout_debug_options: self.layout_debug_options,
        });
    }

    pub(in crate::runtime::controller) fn sync_scroll_offsets(&mut self) {
        self.scratch.scroll_clamp_updates.clear();
        self.scratch.scroll_clamp_updates.extend(
            self.layout
                .diagnostics
                .iter()
                .filter(|diagnostic| {
                    diagnostic.code == LayoutDiagnosticCode::InvalidScrollOffsetClamped
                })
                .filter_map(|diagnostic| {
                    let child_rect = self.layout.rects.get(
                        self.traversal
                            .containers
                            .scroll_content_by_container
                            .get(&diagnostic.node_id)?,
                    )?;
                    let viewport_rect = self
                        .layout
                        .viewport_bounds
                        .get(&diagnostic.node_id)
                        .or_else(|| self.layout.rects.get(&diagnostic.node_id))?;
                    let current_offset = self.layout_state.scroll_offset(diagnostic.node_id);
                    Some((
                        diagnostic.node_id,
                        clamped_scroll_offset(current_offset, *child_rect, *viewport_rect),
                    ))
                }),
        );
        let scroll_clamp_updates = std::mem::take(&mut self.scratch.scroll_clamp_updates);
        for (node_id, offset) in scroll_clamp_updates {
            if self.layout_state.scroll_offset(node_id) != offset {
                self.layout_state.scroll_offsets.insert(node_id, offset);
                self.note_layout_state_mutation();
            }
        }
    }
}

fn effective_layout_viewport(viewport: Rect) -> Rect {
    Rect::from_min_size(
        Point::new(viewport.min.x.floor(), viewport.min.y.floor()),
        Vector2::new(
            viewport.width().round().max(0.0),
            viewport.height().round().max(0.0),
        ),
    )
}

/// Clamp candidate-owned scroll offsets against one candidate layout output.
/// This helper performs no diagnostics or active-runtime bookkeeping, which
/// lets prepared refresh stage the exact state it will publish.
pub(in crate::runtime::controller) fn sync_scroll_offsets_for<Message>(
    layout: &crate::gui::layout_core::LayoutOutput,
    traversal: &SurfaceTraversalIndex<Message>,
    layout_state: &mut crate::gui::layout_core::LayoutState,
) {
    let updates: Vec<_> = layout
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == LayoutDiagnosticCode::InvalidScrollOffsetClamped)
        .filter_map(|diagnostic| {
            let child_rect = layout.rects.get(
                traversal
                    .scroll_content_by_container
                    .get(&diagnostic.node_id)?,
            )?;
            let viewport_rect = layout
                .viewport_bounds
                .get(&diagnostic.node_id)
                .or_else(|| layout.rects.get(&diagnostic.node_id))?;
            let current_offset = layout_state.scroll_offset(diagnostic.node_id);
            Some((
                diagnostic.node_id,
                clamped_scroll_offset(current_offset, *child_rect, *viewport_rect),
            ))
        })
        .collect();
    for (node_id, offset) in updates {
        layout_state.scroll_offsets.insert(node_id, offset);
    }
}

fn clamped_scroll_offset(current: Vector2, child_rect: Rect, viewport_rect: Rect) -> Vector2 {
    Vector2::new(
        current
            .x
            .min((child_rect.width() - viewport_rect.width()).max(0.0)),
        current
            .y
            .min((child_rect.height() - viewport_rect.height()).max(0.0)),
    )
}

pub(in crate::runtime::controller) fn collect_scroll_declarations<'a>(
    node: &'a LayoutNode,
    output: &mut Vec<(
        crate::layout::NodeId,
        crate::layout::NodeId,
        &'a crate::layout::ContainerPolicy,
    )>,
) {
    let LayoutNode::Container(container) = node else {
        return;
    };
    if container.policy.kind == crate::layout::ContainerKind::ScrollView
        && let Some(child) = container.children.first()
    {
        output.push((container.id, child.child.id(), &container.policy));
    }
    for child in &container.children {
        collect_scroll_declarations(&child.child, output);
    }
}

pub(in crate::runtime::controller) fn collect_scroll_requests<'a>(
    node: &'a LayoutNode,
    output: &mut Vec<(
        crate::layout::NodeId,
        &'a crate::layout::ContainerPolicy,
        crate::layout::NodeId,
    )>,
) {
    let LayoutNode::Container(container) = node else {
        return;
    };
    if container.policy.kind == crate::layout::ContainerKind::ScrollView
        && let Some(child) = container.children.first()
    {
        output.push((container.id, &container.policy, child.child.id()));
    }
    for child in &container.children {
        collect_scroll_requests(&child.child, output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::{layout_core::LayoutOutput, types::Point},
        layout::{
            Constraints, ContainerKind, ContainerPolicy, Controlled, OverflowPolicy,
            ScrollAlignment, ScrollAxis, ScrollPolicy, ScrollRequest, ScrollTarget,
            ScrollbarPlacement, SizeModeCross, SizeModeMain, SlotParams,
        },
        runtime::{
            ClipAncestors, Event, LayerKind, RuntimeBridge, SurfaceChild, SurfaceLayer,
            SurfaceNode, UiSurface, WidgetMessageMapper,
        },
        theme::ThemeTokens,
        widgets::{
            ButtonWidget, KeyboardModifiers, TextWidget, Widget, WidgetCommon, WidgetKey,
            WidgetOutput, WidgetSizing,
        },
    };
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    #[test]
    fn clamped_scroll_offset_reuses_current_offset_once_for_both_axes() {
        let child = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 260.0));
        let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 200.0));

        assert_eq!(
            clamped_scroll_offset(Vector2::new(80.0, 90.0), child, viewport),
            Vector2::new(20.0, 60.0)
        );
    }

    #[test]
    fn clamped_scroll_offset_keeps_zero_max_when_content_fits_viewport() {
        let child = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 160.0));
        let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 200.0));

        assert_eq!(
            clamped_scroll_offset(Vector2::new(8.0, 12.0), child, viewport),
            Vector2::new(0.0, 0.0)
        );
    }

    #[test]
    fn scroll_offset_sync_uses_padded_viewport_bounds() {
        let mut runtime = SurfaceRuntime::new(PaddedScrollBridge, Vector2::new(100.0, 80.0));
        let point = Point::new(8.0, 8.0);

        assert!(runtime.scroll_at(point, Vector2::new(0.0, 10_000.0)));
        let before = runtime
            .layout()
            .rects
            .get(&PaddedScrollBridge::CONTENT_ID)
            .copied()
            .expect("content rect after scroll");

        runtime.refresh();
        let after = runtime
            .layout()
            .rects
            .get(&PaddedScrollBridge::CONTENT_ID)
            .copied()
            .expect("content rect after refresh");

        assert_eq!(
            after, before,
            "refresh should not rewrite a padded scroll viewport offset using the outer container"
        );
    }

    #[test]
    fn scroll_policy_reprojection_preserves_mount_and_generation_state() {
        let request = ScrollRequest::new(
            ScrollTarget::Edge(crate::layout::ScrollEdge::Bottom),
            ScrollAlignment::Nearest,
            7,
        );
        let mut runtime = SurfaceRuntime::new(
            ScrollInputRefreshBridge {
                policy: ScrollPolicy::default(),
                initial: Vector2::new(0.0, 11.0),
                controlled: Some(Controlled::new(Vector2::new(0.0, 17.0), 7)),
                request: Some(request),
                scroll: true,
            },
            Vector2::new(100.0, 80.0),
        );
        assert!(runtime.scroll_at(Point::new(8.0, 8.0), Vector2::new(0.0, -23.0)));
        let user_offset = runtime.layout_state.scroll_offset(1);
        assert!(user_offset.y > 17.0);

        runtime.bridge_mut().policy = ScrollPolicy::default()
            .axes(ScrollAxis::Vertical)
            .scrollbar_placement(ScrollbarPlacement::Reserved);
        runtime.refresh();

        assert_eq!(runtime.layout_state.scroll_offset(1), user_offset);
        assert_eq!(
            runtime
                .layout_state
                .scroll_runtime
                .get(&1)
                .and_then(|state| state.controlled_generation),
            Some(7)
        );
        assert_eq!(
            runtime
                .layout_state
                .scroll_runtime
                .get(&1)
                .and_then(|state| state.request_generation),
            Some(7)
        );

        runtime.bridge_mut().scroll = false;
        runtime.refresh();
        assert!(!runtime.layout_state.scroll_runtime.contains_key(&1));

        runtime.bridge_mut().scroll = true;
        runtime.bridge_mut().initial = Vector2::new(0.0, 13.0);
        runtime.bridge_mut().controlled = Some(Controlled::new(Vector2::new(0.0, 17.0), 7));
        runtime.bridge_mut().request = Some(ScrollRequest::new(
            ScrollTarget::Edge(crate::layout::ScrollEdge::Top),
            ScrollAlignment::Nearest,
            7,
        ));
        runtime.refresh();
        assert_eq!(
            runtime.layout_state.scroll_offset(1),
            Vector2::new(0.0, 0.0)
        );
        assert_eq!(
            runtime
                .layout_state
                .scroll_runtime
                .get(&1)
                .and_then(|state| state.controlled_generation),
            Some(7)
        );
        assert_eq!(
            runtime
                .layout_state
                .scroll_runtime
                .get(&1)
                .and_then(|state| state.request_generation),
            Some(7)
        );
    }

    #[test]
    fn declarative_offsets_project_axes_and_generation_fences_are_exact() {
        for (scroll_policy, expected, controlled_expected, projected_noop_input) in [
            (
                ScrollPolicy::default(),
                Vector2::new(13.0, 17.0),
                Vector2::new(31.0, 37.0),
                Vector2::new(31.0, 37.0),
            ),
            (
                ScrollPolicy::default().axes(ScrollAxis::Vertical),
                Vector2::new(0.0, 17.0),
                Vector2::new(0.0, 37.0),
                Vector2::new(99.0, 37.0),
            ),
            (
                ScrollPolicy::default().axes(ScrollAxis::Horizontal),
                Vector2::new(13.0, 0.0),
                Vector2::new(31.0, 0.0),
                Vector2::new(31.0, 99.0),
            ),
            (
                ScrollPolicy::default().axes(ScrollAxis::Both),
                Vector2::new(13.0, 17.0),
                Vector2::new(31.0, 37.0),
                Vector2::new(31.0, 37.0),
            ),
        ] {
            let policy = ContainerPolicy {
                kind: ContainerKind::ScrollView,
                scroll_policy,
                initial_offset: Some(Vector2::new(13.0, 17.0)),
                ..ContainerPolicy::default()
            };
            let mut state = crate::gui::layout_core::LayoutState::default();
            sync_declarative_scroll_inputs_for(vec![(1, 2, &policy)], &mut state, 9);
            assert_eq!(state.scroll_offset(1), expected);
            assert_eq!(state.scroll_runtime[&1].mount_generation, 9);
            assert!(state.scroll_runtime[&1].initial_seeded);

            let controlled = ContainerPolicy {
                kind: ContainerKind::ScrollView,
                scroll_policy,
                controlled_offset: Some(Controlled::new(Vector2::new(31.0, 37.0), 4)),
                ..ContainerPolicy::default()
            };
            sync_declarative_scroll_inputs_for(vec![(1, 2, &controlled)], &mut state, 10);
            assert_eq!(state.scroll_offset(1), controlled_expected);
            assert_eq!(state.scroll_runtime[&1].controlled_generation, Some(4));

            // A newer value is consumed even when projection makes it a no-op.
            let projected_noop = ContainerPolicy {
                kind: ContainerKind::ScrollView,
                scroll_policy,
                controlled_offset: Some(Controlled::new(projected_noop_input, 5)),
                ..ContainerPolicy::default()
            };
            sync_declarative_scroll_inputs_for(vec![(1, 2, &projected_noop)], &mut state, 10);
            assert_eq!(state.scroll_offset(1), controlled_expected);
            assert_eq!(state.scroll_runtime[&1].controlled_generation, Some(5));
        }

        let policy = ContainerPolicy {
            kind: ContainerKind::ScrollView,
            scroll_policy: ScrollPolicy::default().axes(ScrollAxis::Horizontal),
            controlled_offset: Some(Controlled::new(Vector2::new(31.0, 37.0), 9)),
            ..ContainerPolicy::default()
        };
        let mut state = crate::gui::layout_core::LayoutState::default();
        sync_declarative_scroll_inputs_for(vec![(1, 2, &policy)], &mut state, 1);
        assert_eq!(state.scroll_offset(1), Vector2::new(31.0, 0.0));
        assert_eq!(state.scroll_runtime[&1].controlled_generation, Some(9));

        for controlled in [
            Controlled::new(Vector2::new(99.0, 99.0), 8),
            Controlled::new(Vector2::new(99.0, 99.0), 9),
            Controlled::new(Vector2::new(99.0, 99.0), u64::MAX),
            Controlled::new(Vector2::new(f32::NAN, 99.0), 10),
            Controlled::new(Vector2::new(99.0, f32::INFINITY), 10),
        ] {
            let invalid_or_stale = ContainerPolicy {
                kind: ContainerKind::ScrollView,
                scroll_policy: policy.scroll_policy,
                controlled_offset: Some(controlled),
                ..ContainerPolicy::default()
            };
            sync_declarative_scroll_inputs_for(vec![(1, 2, &invalid_or_stale)], &mut state, 2);
            assert_eq!(state.scroll_offset(1), Vector2::new(31.0, 0.0));
            assert_eq!(state.scroll_runtime[&1].controlled_generation, Some(9));
        }
    }

    #[test]
    fn policy_reprojection_clears_only_disabled_retained_axis_without_resurrection() {
        for (axes, expected) in [
            (ScrollAxis::Vertical, Vector2::new(0.0, 50.0)),
            (ScrollAxis::Horizontal, Vector2::new(40.0, 0.0)),
        ] {
            let request = ScrollRequest::rect(
                Rect::from_min_size(Point::new(40.0, 50.0), Vector2::new(20.0, 20.0)),
                ScrollAlignment::Nearest,
                11,
            );
            let mut runtime = SurfaceRuntime::new(
                ScrollInputRefreshBridge {
                    policy: ScrollPolicy::default().axes(ScrollAxis::Both),
                    initial: Vector2::new(40.0, 50.0),
                    controlled: Some(Controlled::new(Vector2::new(40.0, 50.0), 7)),
                    request: Some(request.clone()),
                    scroll: true,
                },
                Vector2::new(100.0, 80.0),
            );
            assert_eq!(
                runtime.layout_state.scroll_offset(1),
                Vector2::new(40.0, 50.0)
            );
            let before = runtime.layout_state.scroll_runtime[&1].clone();

            runtime.bridge_mut().policy = ScrollPolicy::default().axes(axes);
            runtime.bridge_mut().controlled = Some(Controlled::new(Vector2::new(40.0, 50.0), 7));
            runtime.bridge_mut().request = Some(request.clone());
            runtime.refresh();

            assert_eq!(runtime.layout_state.scroll_offset(1), expected);
            assert_eq!(
                runtime.layout_state.scroll_runtime[&1].mount_generation,
                before.mount_generation
            );
            assert_eq!(
                runtime.layout_state.scroll_runtime[&1].controlled_generation,
                Some(7)
            );
            assert_eq!(
                runtime.layout_state.scroll_runtime[&1].request_generation,
                Some(11)
            );

            // Re-enabling both axes with the same generations must not restore
            // the component discarded by the policy reprojection.
            runtime.bridge_mut().policy = ScrollPolicy::default().axes(ScrollAxis::Both);
            runtime.bridge_mut().controlled = Some(Controlled::new(Vector2::new(40.0, 50.0), 7));
            runtime.bridge_mut().request = Some(request);
            runtime.refresh();
            assert_eq!(runtime.layout_state.scroll_offset(1), expected);
            assert_eq!(
                runtime.layout_state.scroll_runtime[&1].controlled_generation,
                Some(7)
            );
            assert_eq!(
                runtime.layout_state.scroll_runtime[&1].request_generation,
                Some(11)
            );
        }
    }

    #[test]
    fn direct_runtime_projects_axes_before_clamping_allowed_content_bounds() {
        for (policy, expected) in [
            (
                ScrollPolicy::default().axes(ScrollAxis::Vertical),
                Vector2::new(0.0, 320.0),
            ),
            (
                ScrollPolicy::default().axes(ScrollAxis::Horizontal),
                Vector2::new(300.0, 0.0),
            ),
            (
                ScrollPolicy::default().axes(ScrollAxis::Both),
                Vector2::new(300.0, 320.0),
            ),
            (ScrollPolicy::default(), Vector2::new(300.0, 320.0)),
        ] {
            let runtime = SurfaceRuntime::new(
                RequestSettlementBridge {
                    request: ScrollRequest::edge(ScrollEdge::Bottom, ScrollAlignment::Nearest, 1),
                    initial: Vector2::new(10_000.0, 10_000.0),
                    policy,
                    settled: Vec::new(),
                },
                Vector2::new(100.0, 80.0),
            );
            assert_eq!(runtime.layout_state.scroll_offset(1), expected);
            assert_eq!(
                runtime.layout().rects[&2].min,
                Point::new(-expected.x, -expected.y)
            );
            assert!(runtime.bridge().settled.is_empty());
        }
    }

    #[test]
    fn phase_less_wheel_settles_once_at_the_bounded_idle_deadline() {
        let mut runtime =
            SurfaceRuntime::new(WheelSettlementBridge::default(), Vector2::new(100.0, 80.0));
        let now = Instant::now();
        runtime.set_timed_repaint_clock(Some(now));

        assert!(runtime.wheel_or_scroll_at(Point::new(8.0, 8.0), Vector2::new(0.0, 12.0)));
        assert_eq!(runtime.bridge().settled, 0);
        let deadline = runtime
            .timed_repaint_deadline()
            .expect("phase-less wheel should arm an idle settlement deadline");
        assert_eq!(deadline, now + Duration::from_millis(100));
        assert!(!runtime.advance_timed_repaints(deadline - Duration::from_millis(1)));
        assert_eq!(runtime.bridge().settled, 0);
        assert!(runtime.advance_timed_repaints(deadline));
        assert_eq!(runtime.bridge().settled, 1);
        assert!(!runtime.advance_timed_repaints(deadline));
        assert_eq!(runtime.bridge().settled, 1);
    }

    #[test]
    fn auto_scroll_affordance_tracks_viewport_and_visual_activity() {
        let mut runtime =
            SurfaceRuntime::new(WheelSettlementBridge::default(), Vector2::new(100.0, 80.0));
        let theme = crate::theme::ThemeTokens::default();
        let has_thumb = |runtime: &SurfaceRuntime<WheelSettlementBridge, String>| {
            runtime
                .paint_plan(&theme)
                .primitives
                .iter()
                .any(|primitive| {
                    matches!(
                        primitive,
                        crate::runtime::PaintPrimitive::FillRect(fill) if fill.widget_id == 1
                    )
                })
        };
        assert!(!has_thumb(&runtime));
        runtime.dispatch_pointer_move_with_outcome(Point::new(8.0, 8.0));
        assert!(has_thumb(&runtime));
        runtime.dispatch_pointer_move_with_outcome(Point::new(-8.0, -8.0));
        assert!(!has_thumb(&runtime));

        let now = Instant::now();
        runtime.set_timed_repaint_clock(Some(now));
        let phaseful = |phase| {
            crate::widgets::WheelSample::from_parts(
                crate::widgets::WheelDelta::Pixels(Vector2::new(0.0, 12.0)),
                Some(phase),
                crate::widgets::PointerModifiers::default(),
                None,
                None,
            )
        };
        assert!(runtime.wheel_or_scroll_at_with_sample(
            Point::new(8.0, 8.0),
            phaseful(crate::widgets::WheelPhase::Started)
        ));
        assert!(has_thumb(&runtime));
        runtime.dispatch_pointer_move_with_outcome(Point::new(-8.0, -8.0));
        assert!(has_thumb(&runtime));
        assert!(!runtime.wheel_or_scroll_at_with_sample(
            Point::new(-8.0, -8.0),
            phaseful(crate::widgets::WheelPhase::Ended)
        ));
        assert!(!has_thumb(&runtime));
        assert_eq!(runtime.bridge().settled, 1);
        assert!(!runtime.wheel_or_scroll_at_with_sample(
            Point::new(-8.0, -8.0),
            phaseful(crate::widgets::WheelPhase::Ended)
        ));
        assert_eq!(runtime.bridge().settled, 1);

        assert!(runtime.wheel_or_scroll_at_with_sample(
            Point::new(8.0, 8.0),
            phaseful(crate::widgets::WheelPhase::Started)
        ));
        runtime.dispatch_pointer_move_with_outcome(Point::new(-8.0, -8.0));
        assert!(has_thumb(&runtime));
        assert!(!runtime.wheel_or_scroll_at_with_sample(
            Point::new(-8.0, -8.0),
            phaseful(crate::widgets::WheelPhase::Cancelled)
        ));
        assert!(!has_thumb(&runtime));
        assert_eq!(runtime.bridge().settled, 1);

        assert!(runtime.wheel_or_scroll_at(Point::new(8.0, 8.0), Vector2::new(0.0, 12.0)));
        assert!(has_thumb(&runtime));
        let deadline = runtime
            .timed_repaint_deadline()
            .expect("visual idle deadline");
        assert!(runtime.advance_timed_repaints(deadline));
        assert!(!has_thumb(&runtime));
    }

    #[test]
    fn terminal_over_wheel_sibling_finalizes_scroll_once_without_swallowing_widget() {
        let mut runtime =
            SurfaceRuntime::new(WheelSiblingBridge::default(), Vector2::new(100.0, 160.0));
        let sample = |phase| {
            crate::widgets::WheelSample::from_parts(
                crate::widgets::WheelDelta::Pixels(Vector2::new(0.0, 12.0)),
                Some(phase),
                crate::widgets::PointerModifiers::default(),
                None,
                None,
            )
        };

        assert!(runtime.wheel_or_scroll_at_with_sample(
            Point::new(8.0, 8.0),
            sample(crate::widgets::WheelPhase::Started)
        ));
        assert_eq!(runtime.bridge().settled, 0);
        assert!(runtime.wheel_or_scroll_at_with_sample(
            Point::new(8.0, 100.0),
            sample(crate::widgets::WheelPhase::Ended)
        ));
        assert_eq!(runtime.bridge().settled, 1);
        assert_eq!(runtime.bridge().sibling_wheels, 1);
        assert!(runtime.wheel_or_scroll_at_with_sample(
            Point::new(8.0, 100.0),
            sample(crate::widgets::WheelPhase::Ended)
        ));
        assert_eq!(runtime.bridge().settled, 1);
        assert_eq!(runtime.bridge().sibling_wheels, 2);

        assert!(runtime.wheel_or_scroll_at_with_sample(
            Point::new(8.0, 8.0),
            sample(crate::widgets::WheelPhase::Started)
        ));
        assert!(runtime.wheel_or_scroll_at_with_sample(
            Point::new(8.0, 100.0),
            sample(crate::widgets::WheelPhase::Cancelled)
        ));
        assert_eq!(runtime.bridge().settled, 1);
        assert_eq!(runtime.bridge().sibling_wheels, 3);
        let now = Instant::now();
        runtime.set_timed_repaint_clock(Some(now));
        assert!(!runtime.advance_timed_repaints(now + Duration::from_millis(100)));
        assert_eq!(runtime.bridge().settled, 1);
    }

    #[test]
    fn removing_scroll_container_cancels_pending_settlement_before_readd() {
        let mut runtime =
            SurfaceRuntime::new(WheelSettlementBridge::default(), Vector2::new(100.0, 80.0));
        let now = Instant::now();
        runtime.set_timed_repaint_clock(Some(now));
        assert!(runtime.wheel_or_scroll_at(Point::new(8.0, 8.0), Vector2::new(0.0, 12.0)));
        let deadline = runtime
            .timed_repaint_deadline()
            .expect("wheel should have a pending settlement");

        runtime.bridge_mut().scroll = false;
        runtime.refresh();
        assert_eq!(runtime.timed_repaint_deadline(), None);
        assert!(!runtime.advance_timed_repaints(deadline));
        assert_eq!(runtime.bridge().settled, 0);
    }

    #[test]
    fn reveal_settlement_reports_the_committed_clamped_offset() {
        let request = ScrollRequest::rect(
            Rect::from_min_size(Point::new(0.0, 10_000.0), Vector2::new(20.0, 20.0)),
            ScrollAlignment::Start,
            1,
        );
        let runtime = SurfaceRuntime::new(
            RequestSettlementBridge {
                request,
                initial: Vector2::default(),
                policy: ScrollPolicy::default(),
                settled: Vec::new(),
            },
            Vector2::new(100.0, 80.0),
        );

        assert_eq!(runtime.bridge().settled, vec![Vector2::new(0.0, 320.0)]);
        assert_eq!(
            runtime.layout_state.scroll_offset(1),
            Vector2::new(0.0, 320.0)
        );
    }

    #[test]
    fn horizontal_rect_requests_follow_effective_axis_and_generation_fence() {
        for (policy, expected_offset, expected_settlements) in [
            (
                ScrollPolicy::default(),
                Vector2::new(300.0, 0.0),
                vec![Vector2::new(300.0, 0.0)],
            ),
            (
                ScrollPolicy::default().axes(ScrollAxis::Vertical),
                Vector2::default(),
                Vec::new(),
            ),
            (
                ScrollPolicy::default().axes(ScrollAxis::Horizontal),
                Vector2::new(300.0, 0.0),
                vec![Vector2::new(300.0, 0.0)],
            ),
            (
                ScrollPolicy::default().axes(ScrollAxis::Both),
                Vector2::new(300.0, 0.0),
                vec![Vector2::new(300.0, 0.0)],
            ),
        ] {
            let mut runtime = SurfaceRuntime::new(
                RequestSettlementBridge {
                    request: ScrollRequest::rect(
                        Rect::from_min_size(Point::new(300.0, 0.0), Vector2::new(20.0, 20.0)),
                        ScrollAlignment::Start,
                        7,
                    ),
                    initial: Vector2::default(),
                    policy,
                    settled: Vec::new(),
                },
                Vector2::new(100.0, 80.0),
            );

            assert_eq!(runtime.layout_state.scroll_offset(1), expected_offset);
            assert_eq!(runtime.bridge().settled, expected_settlements);
            assert_eq!(
                runtime.layout_state.scroll_runtime[&1].request_generation,
                Some(7)
            );

            // A committed generation is consumed once, including the no-op
            // vertical policy and an already-settled effective change.
            runtime.refresh();
            assert_eq!(runtime.layout_state.scroll_offset(1), expected_offset);
            assert_eq!(runtime.bridge().settled, expected_settlements);
        }
    }

    #[test]
    fn horizontal_right_and_end_requests_follow_effective_axis_once() {
        for edge in [ScrollEdge::Right, ScrollEdge::End] {
            for policy in [
                ScrollPolicy::default(),
                ScrollPolicy::default().axes(ScrollAxis::Vertical),
                ScrollPolicy::default().axes(ScrollAxis::Horizontal),
                ScrollPolicy::default().axes(ScrollAxis::Both),
            ] {
                let expected_offset = Vector2::new(
                    if policy.allows_horizontal() {
                        300.0
                    } else {
                        0.0
                    },
                    if edge == ScrollEdge::End && policy.axes.includes_vertical() {
                        320.0
                    } else {
                        0.0
                    },
                );
                let expected_settlements = if expected_offset == Vector2::default() {
                    Vec::new()
                } else {
                    vec![expected_offset]
                };
                let mut runtime = SurfaceRuntime::new(
                    RequestSettlementBridge {
                        request: ScrollRequest::new(
                            ScrollTarget::Edge(edge),
                            ScrollAlignment::Start,
                            11,
                        ),
                        initial: Vector2::default(),
                        policy,
                        settled: Vec::new(),
                    },
                    Vector2::new(100.0, 80.0),
                );

                assert_eq!(runtime.layout_state.scroll_offset(1), expected_offset);
                assert_eq!(runtime.bridge().settled, expected_settlements);
                assert_eq!(
                    runtime.layout_state.scroll_runtime[&1].request_generation,
                    Some(11)
                );
                runtime.refresh();
                assert_eq!(runtime.bridge().settled, expected_settlements);
            }
        }
    }

    #[test]
    fn request_at_boundary_consumes_generation_without_settlement() {
        let request = ScrollRequest::new(
            ScrollTarget::Edge(crate::layout::ScrollEdge::Bottom),
            ScrollAlignment::Nearest,
            9,
        );
        let runtime = SurfaceRuntime::new(
            RequestSettlementBridge {
                request,
                initial: Vector2::new(0.0, 320.0),
                policy: ScrollPolicy::default(),
                settled: Vec::new(),
            },
            Vector2::new(100.0, 80.0),
        );

        assert_eq!(
            runtime.layout_state.scroll_offset(1),
            Vector2::new(0.0, 320.0)
        );
        assert!(runtime.bridge().settled.is_empty());
        assert_eq!(
            runtime
                .layout_state
                .scroll_runtime
                .get(&1)
                .and_then(|state| state.request_generation),
            Some(9)
        );
    }

    #[test]
    fn scroll_edges_follow_configured_axis_and_fixed_axis_wheel_lock() {
        for (axes, edge, expected) in [
            (
                ScrollAxis::Vertical,
                ScrollEdge::Start,
                Vector2::new(17.0, 0.0),
            ),
            (
                ScrollAxis::Vertical,
                ScrollEdge::End,
                Vector2::new(17.0, 400.0),
            ),
            (
                ScrollAxis::Horizontal,
                ScrollEdge::Start,
                Vector2::new(0.0, 23.0),
            ),
            (
                ScrollAxis::Horizontal,
                ScrollEdge::End,
                Vector2::new(400.0, 23.0),
            ),
        ] {
            let policy = ContainerPolicy {
                kind: ContainerKind::ScrollView,
                overflow: OverflowPolicy::Scroll,
                scroll_policy: ScrollPolicy::default().axes(axes),
                scroll_request: Some(ScrollRequest::new(
                    ScrollTarget::Edge(edge),
                    ScrollAlignment::Start,
                    1,
                )),
                ..ContainerPolicy::default()
            };
            let mut layout = crate::gui::layout_core::LayoutOutput::default();
            layout.rects.insert(
                1,
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0)),
            );
            layout.rects.insert(
                2,
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(400.0, 400.0)),
            );
            layout.viewport_bounds.insert(1, layout.rects[&1]);
            let mut state = crate::gui::layout_core::LayoutState::default();
            state.scroll_offsets.insert(1, Vector2::new(17.0, 23.0));
            state
                .scroll_runtime
                .insert(1, ScrollRuntimeState::default());
            let changed = apply_declarative_scroll_requests_for(
                vec![(1, &policy, 2)],
                &layout,
                &mut state,
                |_, _| None,
            );
            assert!(!changed.is_empty());
            assert_eq!(state.scroll_offset(1), expected);
        }

        for (lock, expected) in [
            (
                crate::layout::ScrollAxisLock::Horizontal,
                Vector2::new(12.0, 0.0),
            ),
            (
                crate::layout::ScrollAxisLock::Vertical,
                Vector2::new(0.0, 12.0),
            ),
        ] {
            let mut runtime = SurfaceRuntime::new(
                ScrollInputRefreshBridge {
                    policy: ScrollPolicy::default()
                        .axes(ScrollAxis::Both)
                        .axis_lock(lock),
                    initial: Vector2::default(),
                    controlled: None,
                    request: None,
                    scroll: true,
                },
                Vector2::new(100.0, 80.0),
            );
            assert!(runtime.scroll_at(Point::new(8.0, 8.0), Vector2::new(12.0, 12.0)));
            assert_eq!(runtime.layout_state.scroll_offset(1), expected);
        }
    }

    #[test]
    fn keyed_request_scopes_materialized_lookup_to_its_container() {
        let key = crate::layout::VirtualLayoutItemKey::new(7_u32);
        let policy = ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            scroll_policy: ScrollPolicy::default(),
            scroll_request: Some(ScrollRequest::new(
                ScrollTarget::Keyed(key.clone()),
                ScrollAlignment::Start,
                1,
            )),
            ..ContainerPolicy::default()
        };
        let mut layout = crate::gui::layout_core::LayoutOutput::default();
        layout.rects.insert(
            3,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0)),
        );
        layout.rects.insert(
            4,
            Rect::from_min_size(Point::new(0.0, 160.0), Vector2::new(20.0, 20.0)),
        );
        layout.rects.insert(
            5,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 400.0)),
        );
        layout.viewport_bounds.insert(3, layout.rects[&3]);
        let mut state = crate::gui::layout_core::LayoutState::default();
        state
            .scroll_runtime
            .insert(3, ScrollRuntimeState::default());
        let changed = apply_declarative_scroll_requests_for(
            vec![(3, &policy, 5)],
            &layout,
            &mut state,
            |owner, candidate| {
                assert_eq!(candidate, &key);
                (owner == 3).then_some((owner, 4))
            },
        );
        assert_eq!(changed, vec![(3, Vector2::default())]);
        assert_eq!(state.scroll_offset(3), Vector2::new(0.0, 160.0));
    }

    #[test]
    fn rejected_keyed_request_is_consumed_before_materialization() {
        let key = crate::layout::VirtualLayoutItemKey::new(7_u32);
        let mut policy = ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            scroll_policy: ScrollPolicy::default(),
            scroll_request: Some(ScrollRequest::new(
                ScrollTarget::Keyed(key.clone()),
                ScrollAlignment::Start,
                4,
            )),
            ..ContainerPolicy::default()
        };
        let mut layout = crate::gui::layout_core::LayoutOutput::default();
        layout.rects.insert(
            3,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0)),
        );
        layout.rects.insert(
            5,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 400.0)),
        );
        layout.rects.insert(
            4,
            Rect::from_min_size(Point::new(0.0, 160.0), Vector2::new(20.0, 20.0)),
        );
        layout.viewport_bounds.insert(3, layout.rects[&3]);
        let mut state = crate::gui::layout_core::LayoutState::default();
        state
            .scroll_runtime
            .insert(3, ScrollRuntimeState::default());
        let first = apply_declarative_scroll_requests_for(
            vec![(3, &policy, 5)],
            &layout,
            &mut state,
            |_, _| None,
        );
        assert!(first.is_empty());
        assert_eq!(state.scroll_runtime[&3].request_generation, Some(4));
        let second = apply_declarative_scroll_requests_for(
            vec![(3, &policy, 5)],
            &layout,
            &mut state,
            |_, candidate| (candidate == &key).then_some((3, 4)),
        );
        assert!(second.is_empty());

        policy.scroll_request = Some(ScrollRequest::new(
            ScrollTarget::Keyed(key.clone()),
            ScrollAlignment::Start,
            5,
        ));
        let third = apply_declarative_scroll_requests_for(
            vec![(3, &policy, 5)],
            &layout,
            &mut state,
            |_, candidate| (candidate == &key).then_some((3, 4)),
        );
        assert_eq!(third, vec![(3, Vector2::default())]);
        assert_eq!(state.scroll_offset(3), Vector2::new(0.0, 160.0));
    }

    #[test]
    fn focus_reveal_requires_a_recorded_clip_ancestor_path() {
        for empty_path in [false, true] {
            let mut runtime = SurfaceRuntime::new(
                SiblingFocusRevealBridge::default(),
                Vector2::new(100.0, 80.0),
            );
            let offsets = [
                runtime
                    .layout_state
                    .scroll_offset(SiblingFocusRevealBridge::FIRST_ID),
                runtime
                    .layout_state
                    .scroll_offset(SiblingFocusRevealBridge::SECOND_ID),
            ];
            let content_rects = [
                runtime.layout().rects[&SiblingFocusRevealBridge::FIRST_CONTENT_ID],
                runtime.layout().rects[&SiblingFocusRevealBridge::SECOND_CONTENT_ID],
            ];
            let layout_state_generation = runtime.layout_state_generation;
            let rects = runtime.layout().rects.clone();
            assert!(runtime.bridge().settled.is_empty());

            if empty_path {
                runtime.traversal.widgets.paths.clip_ancestors.insert(
                    SiblingFocusRevealBridge::OUTSIDE_ID,
                    ClipAncestors::from_slice(&[]),
                );
            } else {
                runtime
                    .traversal
                    .widgets
                    .paths
                    .clip_ancestors
                    .remove(&SiblingFocusRevealBridge::OUTSIDE_ID);
            }

            assert!(runtime.focus_widget(SiblingFocusRevealBridge::OUTSIDE_ID));
            assert_eq!(
                runtime.focused_widget(),
                Some(SiblingFocusRevealBridge::OUTSIDE_ID)
            );
            assert!(runtime.bridge().settled.is_empty());
            assert_eq!(
                [
                    runtime
                        .layout_state
                        .scroll_offset(SiblingFocusRevealBridge::FIRST_ID),
                    runtime
                        .layout_state
                        .scroll_offset(SiblingFocusRevealBridge::SECOND_ID),
                ],
                offsets
            );
            assert_eq!(
                [
                    runtime.layout().rects[&SiblingFocusRevealBridge::FIRST_CONTENT_ID],
                    runtime.layout().rects[&SiblingFocusRevealBridge::SECOND_CONTENT_ID],
                ],
                content_rects
            );
            assert_eq!(runtime.layout_state_generation, layout_state_generation);
            assert_eq!(runtime.layout().rects, rects);
        }
    }

    #[test]
    fn malformed_request_is_consumed_and_max_generation_is_ignored() {
        let mut policy = ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            scroll_policy: ScrollPolicy::default(),
            scroll_request: Some(ScrollRequest::rect(
                Rect::from_min_size(Point::new(0.0, 40.0), Vector2::new(0.0, 20.0)),
                ScrollAlignment::Start,
                8,
            )),
            ..ContainerPolicy::default()
        };
        let mut layout = crate::gui::layout_core::LayoutOutput::default();
        layout.rects.insert(
            3,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0)),
        );
        layout.rects.insert(
            5,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 400.0)),
        );
        layout.viewport_bounds.insert(3, layout.rects[&3]);
        let mut state = crate::gui::layout_core::LayoutState::default();
        state
            .scroll_runtime
            .insert(3, ScrollRuntimeState::default());
        assert!(
            apply_declarative_scroll_requests_for(
                vec![(3, &policy, 5)],
                &layout,
                &mut state,
                |_, _| None,
            )
            .is_empty()
        );
        assert_eq!(state.scroll_runtime[&3].request_generation, Some(8));
        policy.scroll_request = Some(ScrollRequest::rect(
            Rect::from_min_size(Point::new(0.0, 40.0), Vector2::new(20.0, 20.0)),
            ScrollAlignment::Start,
            8,
        ));
        assert!(
            apply_declarative_scroll_requests_for(
                vec![(3, &policy, 5)],
                &layout,
                &mut state,
                |_, _| None,
            )
            .is_empty()
        );
        policy.scroll_request = Some(ScrollRequest::new(
            ScrollTarget::Edge(ScrollEdge::Bottom),
            ScrollAlignment::Start,
            u64::MAX,
        ));
        assert!(
            apply_declarative_scroll_requests_for(
                vec![(3, &policy, 5)],
                &layout,
                &mut state,
                |_, _| None,
            )
            .is_empty()
        );
        assert_eq!(state.scroll_runtime[&3].request_generation, Some(8));
        policy.scroll_request = Some(ScrollRequest::new(
            ScrollTarget::Edge(ScrollEdge::Bottom),
            ScrollAlignment::Start,
            9,
        ));
        assert_eq!(
            apply_declarative_scroll_requests_for(
                vec![(3, &policy, 5)],
                &layout,
                &mut state,
                |_, _| None,
            ),
            vec![(3, Vector2::default())]
        );
        assert_eq!(state.scroll_offset(3), Vector2::new(0.0, 400.0));
    }

    #[test]
    fn focus_reveal_uses_translated_content_coordinates_at_nonzero_offset() {
        let mut runtime = SurfaceRuntime::new(
            FocusRevealBridge {
                policy: ScrollPolicy::default(),
            },
            Vector2::new(100.0, 80.0),
        );
        let content = runtime.layout().rects[&FocusRevealBridge::CONTENT_ID];
        let target = runtime.layout().rects[&FocusRevealBridge::TARGET_ID];
        let viewport = runtime.layout().viewport_bounds[&FocusRevealBridge::SCROLL_ID];
        let expected = (target.max.y - content.min.y - viewport.height()).max(0.0);

        assert_eq!(
            runtime
                .layout_state
                .scroll_offset(FocusRevealBridge::SCROLL_ID)
                .y,
            40.0
        );
        assert!(runtime.focus_widget(FocusRevealBridge::TARGET_ID));
        assert_eq!(
            runtime
                .layout_state
                .scroll_offset(FocusRevealBridge::SCROLL_ID)
                .y,
            expected,
            "revealing a translated target must subtract the translated content origin once"
        );
    }

    #[test]
    fn focused_home_end_and_page_keys_follow_effective_axes() {
        for (policy, expected_end, expected_page_up, expected_page_down) in [
            (
                ScrollPolicy::default(),
                Vector2::new(60.0, 880.0),
                Vector2::new(60.0, 800.0),
                Vector2::new(0.0, 80.0),
            ),
            (
                ScrollPolicy::default().axes(ScrollAxis::Vertical),
                Vector2::new(0.0, 880.0),
                Vector2::new(0.0, 800.0),
                Vector2::new(0.0, 80.0),
            ),
            (
                ScrollPolicy::default().axes(ScrollAxis::Horizontal),
                Vector2::new(60.0, 0.0),
                Vector2::new(60.0, 0.0),
                Vector2::new(0.0, 0.0),
            ),
            (
                ScrollPolicy::default().axes(ScrollAxis::Both),
                Vector2::new(60.0, 880.0),
                Vector2::new(60.0, 800.0),
                Vector2::new(0.0, 80.0),
            ),
        ] {
            let mut runtime =
                SurfaceRuntime::new(FocusRevealBridge { policy }, Vector2::new(100.0, 80.0));
            assert!(runtime.focus_widget(FocusRevealBridge::TARGET_ID));

            assert_eq!(
                runtime.dispatch_event(Event::KeyPress {
                    key: WidgetKey::End,
                    modifiers: KeyboardModifiers::default(),
                    repeat: false,
                    timestamp: None,
                }),
                Some(FocusRevealBridge::TARGET_ID)
            );
            assert_eq!(
                runtime
                    .layout_state
                    .scroll_offset(FocusRevealBridge::SCROLL_ID),
                expected_end
            );

            assert_eq!(
                runtime.dispatch_event(Event::KeyPress {
                    key: WidgetKey::PageUp,
                    modifiers: KeyboardModifiers::default(),
                    repeat: false,
                    timestamp: None,
                }),
                Some(FocusRevealBridge::TARGET_ID)
            );
            assert_eq!(
                runtime
                    .layout_state
                    .scroll_offset(FocusRevealBridge::SCROLL_ID),
                expected_page_up
            );

            assert_eq!(
                runtime.dispatch_event(Event::KeyPress {
                    key: WidgetKey::Home,
                    modifiers: KeyboardModifiers::default(),
                    repeat: false,
                    timestamp: None,
                }),
                Some(FocusRevealBridge::TARGET_ID)
            );
            assert_eq!(
                runtime
                    .layout_state
                    .scroll_offset(FocusRevealBridge::SCROLL_ID),
                Vector2::default()
            );

            assert_eq!(
                runtime.dispatch_event(Event::KeyPress {
                    key: WidgetKey::PageDown,
                    modifiers: KeyboardModifiers::default(),
                    repeat: false,
                    timestamp: None,
                }),
                Some(FocusRevealBridge::TARGET_ID)
            );
            assert_eq!(
                runtime
                    .layout_state
                    .scroll_offset(FocusRevealBridge::SCROLL_ID),
                expected_page_down
            );
        }
    }

    #[test]
    fn focus_reveal_settles_nested_scrollers_inside_out_once() {
        let mut runtime = SurfaceRuntime::new(
            NestedFocusRevealBridge::default(),
            Vector2::new(100.0, 80.0),
        );

        assert!(runtime.focus_widget(NestedFocusRevealBridge::TARGET_ID));
        assert_eq!(
            runtime.focused_widget(),
            Some(NestedFocusRevealBridge::TARGET_ID)
        );
        assert_eq!(
            runtime.bridge().settled.len(),
            2,
            "each changed scroll owner settles once"
        );
        assert_eq!(
            runtime.bridge().settled[0].0,
            NestedFocusRevealBridge::INNER_ID
        );
        assert_eq!(
            runtime.bridge().settled[1].0,
            NestedFocusRevealBridge::OUTER_ID
        );
        assert!(
            runtime
                .layout_state
                .scroll_offset(NestedFocusRevealBridge::INNER_ID)
                .y
                > 20.0
        );
        assert!(
            runtime
                .layout_state
                .scroll_offset(NestedFocusRevealBridge::OUTER_ID)
                .y
                > 10.0
        );

        let target = runtime.layout().rects[&NestedFocusRevealBridge::TARGET_ID];
        for node_id in [
            NestedFocusRevealBridge::INNER_ID,
            NestedFocusRevealBridge::OUTER_ID,
        ] {
            let viewport = runtime.layout().viewport_bounds[&node_id];
            assert!(target.min.y >= viewport.min.y);
            assert!(target.max.y <= viewport.max.y);
        }
    }

    #[test]
    fn default_horizontal_focus_reveal_settles_recorded_ancestors_inside_out() {
        let mut runtime = SurfaceRuntime::new(
            NestedFocusRevealBridge::horizontal(),
            Vector2::new(100.0, 80.0),
        );
        let unrelated_before = runtime
            .layout_state
            .scroll_offset(NestedFocusRevealBridge::UNRELATED_ID);

        assert!(runtime.focus_widget(NestedFocusRevealBridge::TARGET_ID));
        assert_eq!(
            runtime.bridge().settled.len(),
            2,
            "only recorded inner and outer ancestors settle"
        );
        assert_eq!(
            runtime.bridge().settled[0].0,
            NestedFocusRevealBridge::INNER_ID
        );
        assert_eq!(
            runtime.bridge().settled[1].0,
            NestedFocusRevealBridge::OUTER_ID
        );
        assert_eq!(runtime.bridge().settled[0].1.x, 100.0);
        assert_eq!(runtime.bridge().settled[1].1.x, 120.0);
        assert_eq!(
            runtime
                .layout_state
                .scroll_offset(NestedFocusRevealBridge::UNRELATED_ID),
            unrelated_before
        );
        assert!(
            !runtime
                .bridge()
                .settled
                .iter()
                .any(|(id, _)| *id == NestedFocusRevealBridge::UNRELATED_ID)
        );

        let target = runtime.layout().rects[&NestedFocusRevealBridge::TARGET_ID];
        for node_id in [
            NestedFocusRevealBridge::INNER_ID,
            NestedFocusRevealBridge::OUTER_ID,
        ] {
            let viewport = runtime.layout().viewport_bounds[&node_id];
            assert!(target.min.x >= viewport.min.x);
            assert!(target.max.x <= viewport.max.x);
        }
    }

    #[test]
    fn focus_reveal_isolated_between_scene_base_and_layer_scrollers() {
        let mut runtime =
            SurfaceRuntime::new(SceneFocusRevealBridge::default(), Vector2::new(100.0, 80.0));
        let base_offset = runtime
            .layout_state
            .scroll_offset(SceneFocusRevealBridge::BASE_ID);

        assert!(runtime.focus_widget(SceneFocusRevealBridge::LAYER_OUTSIDE_ID));
        assert!(runtime.bridge().settled.is_empty());
        assert_eq!(
            runtime
                .layout_state
                .scroll_offset(SceneFocusRevealBridge::BASE_ID),
            base_offset
        );

        assert!(runtime.focus_widget(SceneFocusRevealBridge::LAYER_TARGET_ID));
        assert_eq!(
            runtime.focused_widget(),
            Some(SceneFocusRevealBridge::LAYER_TARGET_ID)
        );
        assert_eq!(
            runtime.bridge().settled.len(),
            1,
            "the layer-local owner is the only changed scroll container"
        );
        assert_eq!(
            runtime.bridge().settled[0].0,
            SceneFocusRevealBridge::LAYER_SCROLL_ID
        );
        assert_eq!(
            runtime
                .layout_state
                .scroll_offset(SceneFocusRevealBridge::BASE_ID),
            base_offset
        );
        assert!(
            runtime
                .layout_state
                .scroll_offset(SceneFocusRevealBridge::LAYER_SCROLL_ID)
                .y
                > 40.0
        );
    }

    #[test]
    fn focus_reveal_rechecks_ancestry_after_reentrant_inner_settlement() {
        let mut runtime = SurfaceRuntime::new(
            ReentrantFocusRevealBridge::default(),
            Vector2::new(100.0, 80.0),
        );
        let outer_offset = runtime
            .layout_state
            .scroll_offset(ReentrantFocusRevealBridge::OUTER_ID);
        let outer_content = runtime.layout().rects[&ReentrantFocusRevealBridge::OUTER_CONTENT_ID];

        assert!(runtime.focus_widget(ReentrantFocusRevealBridge::TARGET_ID));
        assert_eq!(
            runtime.focused_widget(),
            Some(ReentrantFocusRevealBridge::TARGET_ID)
        );
        assert!(runtime.bridge().moved);
        assert_eq!(
            runtime.bridge().settled.len(),
            1,
            "the stale outer owner must not settle after reparenting"
        );
        assert_eq!(
            runtime.bridge().settled[0].0,
            ReentrantFocusRevealBridge::INNER_ID
        );
        assert_eq!(
            runtime
                .layout_state
                .scroll_offset(ReentrantFocusRevealBridge::OUTER_ID),
            outer_offset
        );
        assert_eq!(
            runtime.layout().rects[&ReentrantFocusRevealBridge::OUTER_CONTENT_ID],
            outer_content
        );
        assert!(
            runtime
                .traversal
                .widgets
                .paths
                .clip_ancestors
                .get(&ReentrantFocusRevealBridge::TARGET_ID)
                .is_none_or(|path| path.as_slice().is_empty())
        );
    }

    #[test]
    fn overlapping_sibling_scroll_containers_do_not_join_residual_chain() {
        let mut runtime = SurfaceRuntime::new(OverlappingScrollBridge, Vector2::new(100.0, 80.0));
        assert!(runtime.scroll_at(Point::new(8.0, 8.0), Vector2::new(0.0, 10_000.0)));

        assert_eq!(
            runtime
                .layout_state
                .scroll_offset(OverlappingScrollBridge::TOP_ID)
                .y,
            0.0
        );
        assert_eq!(
            runtime
                .layout_state
                .scroll_offset(OverlappingScrollBridge::BOTTOM_ID)
                .y,
            320.0
        );
    }

    #[test]
    fn nested_wheel_settles_each_changed_scroll_owner_once_at_idle() {
        let settled = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut runtime = SurfaceRuntime::new(
            NestedWheelBridge::new(std::rc::Rc::clone(&settled)),
            Vector2::new(100.0, 80.0),
        );
        let now = Instant::now();
        runtime.set_timed_repaint_clock(Some(now));

        let inner_origin = runtime.layout().rects[&NestedWheelBridge::INNER_ID].min;
        assert!(runtime.wheel_or_scroll_at(
            Point::new(inner_origin.x + 8.0, inner_origin.y + 8.0),
            Vector2::new(0.0, 10_000.0)
        ));
        assert!(
            runtime
                .layout_state
                .scroll_offset(NestedWheelBridge::INNER_ID)
                .y
                > 0.0
        );
        assert!(
            runtime
                .layout_state
                .scroll_offset(NestedWheelBridge::OUTER_ID)
                .y
                > 0.0
        );
        let deadline = runtime
            .timed_repaint_deadline()
            .expect("wheel idle deadline");
        assert!(runtime.advance_timed_repaints(deadline));
        let settled = settled.borrow();
        assert_eq!(settled.len(), 2);
        assert_eq!(
            settled
                .iter()
                .filter(|(id, _)| *id == NestedWheelBridge::INNER_ID)
                .count(),
            1
        );
        assert_eq!(
            settled
                .iter()
                .filter(|(id, _)| *id == NestedWheelBridge::OUTER_ID)
                .count(),
            1
        );
    }

    struct PaddedScrollBridge;

    struct FocusRevealBridge {
        policy: ScrollPolicy,
    }

    impl FocusRevealBridge {
        const SCROLL_ID: u64 = 1;
        const CONTENT_ID: u64 = 2;
        const TARGET_ID: u64 = 10 + 7;
    }

    impl RuntimeBridge<()> for FocusRevealBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            let rows = (0..12)
                .map(|index| {
                    let size_cross = if index == 7 {
                        SizeModeCross::Intrinsic
                    } else {
                        SizeModeCross::Fill
                    };
                    let widget = if index == 7 {
                        SurfaceNode::widget(
                            ButtonWidget::new(
                                10 + index,
                                format!("Row {index}"),
                                WidgetSizing::fixed(Vector2::new(160.0, 30.0)),
                            ),
                            WidgetMessageMapper::none(),
                        )
                    } else {
                        SurfaceNode::widget(
                            TextWidget::new(
                                10 + index,
                                format!("Row {index}"),
                                WidgetSizing::fixed(Vector2::new(80.0, 30.0)),
                            ),
                            WidgetMessageMapper::none(),
                        )
                    };
                    SurfaceChild::new(
                        SlotParams {
                            size_main: SizeModeMain::Fixed(80.0),
                            size_cross,
                            constraints: Constraints::unconstrained(),
                            margin: Default::default(),
                            align_cross_override: None,
                            allow_fixed_compress: false,
                        },
                        widget,
                    )
                })
                .collect();
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
                Self::SCROLL_ID,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    scroll_policy: self.policy,
                    initial_offset: Some(Vector2::new(0.0, 40.0)),
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::fill(SurfaceNode::column(
                    Self::CONTENT_ID,
                    0.0,
                    rows,
                ))],
            )))
        }

        fn reduce_message(&mut self, _message: ()) {}
    }

    #[derive(Default)]
    struct NestedFocusRevealBridge {
        settled: Vec<(u64, Vector2)>,
        horizontal: bool,
    }

    impl NestedFocusRevealBridge {
        const OUTER_ID: u64 = 1;
        const OUTER_CONTENT_ID: u64 = 2;
        const INNER_ID: u64 = 3;
        const INNER_CONTENT_ID: u64 = 4;
        const PREFIX_ID: u64 = 5;
        const TARGET_ID: u64 = 17;
        const UNRELATED_ID: u64 = 50;

        fn horizontal() -> Self {
            Self {
                horizontal: true,
                ..Self::default()
            }
        }
    }

    impl RuntimeBridge<(u64, Vector2)> for NestedFocusRevealBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<(u64, Vector2)>> {
            let horizontal = self.horizontal;
            let rows = (0..12)
                .map(|index| {
                    let widget = if index == 7 {
                        SurfaceNode::widget(
                            ButtonWidget::new(
                                Self::TARGET_ID,
                                "Target",
                                WidgetSizing::fixed(Vector2::new(80.0, 30.0)),
                            ),
                            WidgetMessageMapper::none(),
                        )
                    } else {
                        SurfaceNode::widget(
                            TextWidget::new(
                                20 + index,
                                format!("Inner {index}"),
                                WidgetSizing::fixed(Vector2::new(80.0, 30.0)),
                            ),
                            WidgetMessageMapper::none(),
                        )
                    };
                    let size_cross = if horizontal && index == 7 {
                        SizeModeCross::Intrinsic
                    } else {
                        SizeModeCross::Fill
                    };
                    let widget = if horizontal && index == 7 {
                        SurfaceNode::row(
                            170,
                            0.0,
                            vec![
                                SurfaceChild::new(
                                    SlotParams {
                                        size_main: SizeModeMain::Fixed(120.0),
                                        size_cross: SizeModeCross::Fixed(30.0),
                                        constraints: Constraints::unconstrained(),
                                        margin: Default::default(),
                                        align_cross_override: None,
                                        allow_fixed_compress: false,
                                    },
                                    SurfaceNode::text(
                                        171,
                                        "Spacer",
                                        WidgetSizing::fixed(Vector2::new(120.0, 30.0)),
                                    ),
                                ),
                                SurfaceChild::new(
                                    SlotParams {
                                        size_main: SizeModeMain::Fixed(80.0),
                                        size_cross: SizeModeCross::Fixed(30.0),
                                        constraints: Constraints::unconstrained(),
                                        margin: Default::default(),
                                        align_cross_override: None,
                                        allow_fixed_compress: false,
                                    },
                                    widget,
                                ),
                            ],
                        )
                    } else {
                        widget
                    };
                    SurfaceChild::new(
                        SlotParams {
                            size_main: SizeModeMain::Fixed(80.0),
                            size_cross,
                            constraints: Constraints::unconstrained(),
                            margin: Default::default(),
                            align_cross_override: None,
                            allow_fixed_compress: false,
                        },
                        widget,
                    )
                })
                .collect();
            let inner_content = SurfaceNode::column(Self::INNER_CONTENT_ID, 0.0, rows);
            let inner_content = if horizontal {
                SurfaceChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Intrinsic,
                        size_cross: SizeModeCross::Intrinsic,
                        constraints: Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    inner_content,
                )
            } else {
                SurfaceChild::fill(inner_content)
            };
            let inner = SurfaceNode::container(
                Self::INNER_ID,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    initial_offset: Some(Vector2::new(0.0, 20.0)),
                    ..ContainerPolicy::default()
                },
                vec![inner_content],
            )
            .on_offset_settled(|offset| (Self::INNER_ID, offset));
            let inner = if horizontal {
                SurfaceNode::row(
                    180,
                    0.0,
                    vec![
                        SurfaceChild::new(
                            SlotParams {
                                size_main: SizeModeMain::Fixed(120.0),
                                size_cross: SizeModeCross::Fixed(80.0),
                                constraints: Constraints::unconstrained(),
                                margin: Default::default(),
                                align_cross_override: None,
                                allow_fixed_compress: false,
                            },
                            SurfaceNode::text(
                                181,
                                "Outer spacer",
                                WidgetSizing::fixed(Vector2::new(120.0, 80.0)),
                            ),
                        ),
                        SurfaceChild::new(
                            SlotParams {
                                size_main: SizeModeMain::Fixed(100.0),
                                size_cross: SizeModeCross::Fixed(100.0),
                                constraints: Constraints::unconstrained(),
                                margin: Default::default(),
                                align_cross_override: None,
                                allow_fixed_compress: false,
                            },
                            inner,
                        ),
                    ],
                )
            } else {
                inner
            };
            let outer_content = SurfaceNode::column(
                Self::OUTER_CONTENT_ID,
                0.0,
                vec![
                    SurfaceChild::new(
                        SlotParams {
                            size_main: SizeModeMain::Fixed(120.0),
                            size_cross: SizeModeCross::Fill,
                            constraints: Constraints::unconstrained(),
                            margin: Default::default(),
                            align_cross_override: None,
                            allow_fixed_compress: false,
                        },
                        SurfaceNode::widget(
                            TextWidget::new(
                                Self::PREFIX_ID,
                                "Prefix",
                                WidgetSizing::fixed(Vector2::new(80.0, 120.0)),
                            ),
                            WidgetMessageMapper::none(),
                        ),
                    ),
                    SurfaceChild::new(
                        SlotParams {
                            size_main: SizeModeMain::Fixed(80.0),
                            size_cross: if horizontal {
                                SizeModeCross::Intrinsic
                            } else {
                                SizeModeCross::Fill
                            },
                            constraints: Constraints::unconstrained(),
                            margin: Default::default(),
                            align_cross_override: None,
                            allow_fixed_compress: false,
                        },
                        inner,
                    ),
                ],
            );
            let outer_content = if horizontal {
                SurfaceChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Intrinsic,
                        size_cross: SizeModeCross::Intrinsic,
                        constraints: Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    outer_content,
                )
            } else {
                SurfaceChild::fill(outer_content)
            };
            let outer = SurfaceNode::container(
                Self::OUTER_ID,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    initial_offset: Some(Vector2::new(0.0, 10.0)),
                    ..ContainerPolicy::default()
                },
                vec![outer_content],
            )
            .on_offset_settled(|offset| (Self::OUTER_ID, offset));
            let root = if horizontal {
                SurfaceNode::stack(
                    99,
                    vec![
                        SurfaceChild::fill(SurfaceNode::scroll_area(
                            Self::UNRELATED_ID,
                            SurfaceNode::text(
                                51,
                                "Unrelated",
                                WidgetSizing::fixed(Vector2::new(180.0, 400.0)),
                            ),
                        )),
                        SurfaceChild::fill(outer),
                    ],
                )
            } else {
                outer
            };
            crate::runtime::test_arc_surface(UiSurface::new(root))
        }

        fn reduce_message(&mut self, message: (u64, Vector2)) {
            self.settled.push(message);
        }
    }

    #[derive(Default)]
    struct SceneFocusRevealBridge {
        settled: Vec<(u64, Vector2)>,
    }

    impl SceneFocusRevealBridge {
        const BASE_ID: u64 = 1;
        const BASE_CONTENT_ID: u64 = 2;
        const LAYER_SCROLL_ID: u64 = 3;
        const LAYER_CONTENT_ID: u64 = 4;
        const LAYER_OUTSIDE_ID: u64 = 10;
        const LAYER_TARGET_ID: u64 = 17;

        fn layer_rows() -> Vec<SurfaceChild<(u64, Vector2)>> {
            (0..8)
                .map(|index| {
                    let widget = if index == 7 {
                        SurfaceNode::widget(
                            ButtonWidget::new(
                                Self::LAYER_TARGET_ID,
                                "Layer target",
                                WidgetSizing::fixed(Vector2::new(80.0, 30.0)),
                            ),
                            WidgetMessageMapper::none(),
                        )
                    } else {
                        SurfaceNode::widget(
                            TextWidget::new(
                                20 + index,
                                format!("Layer row {index}"),
                                WidgetSizing::fixed(Vector2::new(80.0, 30.0)),
                            ),
                            WidgetMessageMapper::none(),
                        )
                    };
                    SurfaceChild::new(
                        SlotParams {
                            size_main: SizeModeMain::Fixed(80.0),
                            size_cross: SizeModeCross::Fill,
                            constraints: Constraints::unconstrained(),
                            margin: Default::default(),
                            align_cross_override: None,
                            allow_fixed_compress: false,
                        },
                        widget,
                    )
                })
                .collect()
        }
    }

    impl RuntimeBridge<(u64, Vector2)> for SceneFocusRevealBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<(u64, Vector2)>> {
            let base = SurfaceNode::container(
                Self::BASE_ID,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    initial_offset: Some(Vector2::new(0.0, 40.0)),
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::fill(SurfaceNode::widget(
                    TextWidget::new(
                        Self::BASE_CONTENT_ID,
                        "Base content",
                        WidgetSizing::fixed(Vector2::new(100.0, 400.0)),
                    ),
                    WidgetMessageMapper::none(),
                ))],
            )
            .on_offset_settled(|offset| (Self::BASE_ID, offset));
            let layer_scroll = SurfaceNode::container(
                Self::LAYER_SCROLL_ID,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    initial_offset: Some(Vector2::new(0.0, 40.0)),
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::fill(SurfaceNode::column(
                    Self::LAYER_CONTENT_ID,
                    0.0,
                    Self::layer_rows(),
                ))],
            )
            .on_offset_settled(|offset| (Self::LAYER_SCROLL_ID, offset));
            let layer = SurfaceNode::stack(
                20,
                vec![
                    SurfaceChild::fill(SurfaceNode::widget(
                        ButtonWidget::new(
                            Self::LAYER_OUTSIDE_ID,
                            "Outside layer scroll",
                            WidgetSizing::fixed(Vector2::new(100.0, 24.0)),
                        ),
                        WidgetMessageMapper::none(),
                    )),
                    SurfaceChild::fill(layer_scroll),
                ],
            );
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::scene(
                100,
                base,
                vec![SurfaceLayer::new(LayerKind::Modal, layer)],
            )))
        }

        fn reduce_message(&mut self, message: (u64, Vector2)) {
            self.settled.push(message);
        }
    }

    #[derive(Default)]
    struct ReentrantFocusRevealBridge {
        moved: bool,
        settled: Vec<(u64, Vector2)>,
    }

    impl ReentrantFocusRevealBridge {
        const OUTER_ID: u64 = 1;
        const OUTER_CONTENT_ID: u64 = 2;
        const INNER_ID: u64 = 3;
        const INNER_CONTENT_ID: u64 = 4;
        const PREFIX_ID: u64 = 5;
        const TARGET_ID: u64 = 17;
        const TARGET_PLACEHOLDER_ID: u64 = 18;

        fn nested_content(&self) -> SurfaceNode<(u64, Vector2)> {
            let rows = (0..12)
                .map(|index| {
                    let widget = if index == 7 && !self.moved {
                        SurfaceNode::widget(
                            ButtonWidget::new(
                                Self::TARGET_ID,
                                "Target",
                                WidgetSizing::fixed(Vector2::new(80.0, 30.0)),
                            ),
                            WidgetMessageMapper::none(),
                        )
                    } else {
                        let id = if index == 7 {
                            Self::TARGET_PLACEHOLDER_ID
                        } else {
                            20 + index
                        };
                        SurfaceNode::widget(
                            TextWidget::new(
                                id,
                                format!("Inner {index}"),
                                WidgetSizing::fixed(Vector2::new(80.0, 30.0)),
                            ),
                            WidgetMessageMapper::none(),
                        )
                    };
                    SurfaceChild::new(
                        SlotParams {
                            size_main: SizeModeMain::Fixed(80.0),
                            size_cross: SizeModeCross::Fill,
                            constraints: Constraints::unconstrained(),
                            margin: Default::default(),
                            align_cross_override: None,
                            allow_fixed_compress: false,
                        },
                        widget,
                    )
                })
                .collect();
            SurfaceNode::column(Self::INNER_CONTENT_ID, 0.0, rows)
        }

        fn outer(&self) -> SurfaceNode<(u64, Vector2)> {
            let inner = SurfaceNode::container(
                Self::INNER_ID,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    initial_offset: Some(Vector2::new(0.0, 20.0)),
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::fill(self.nested_content())],
            )
            .on_offset_settled(|offset| (Self::INNER_ID, offset));
            let content = SurfaceNode::column(
                Self::OUTER_CONTENT_ID,
                0.0,
                vec![
                    SurfaceChild::new(
                        SlotParams {
                            size_main: SizeModeMain::Fixed(120.0),
                            size_cross: SizeModeCross::Fill,
                            constraints: Constraints::unconstrained(),
                            margin: Default::default(),
                            align_cross_override: None,
                            allow_fixed_compress: false,
                        },
                        SurfaceNode::widget(
                            TextWidget::new(
                                Self::PREFIX_ID,
                                "Prefix",
                                WidgetSizing::fixed(Vector2::new(80.0, 120.0)),
                            ),
                            WidgetMessageMapper::none(),
                        ),
                    ),
                    SurfaceChild::new(
                        SlotParams {
                            size_main: SizeModeMain::Fixed(80.0),
                            size_cross: SizeModeCross::Fill,
                            constraints: Constraints::unconstrained(),
                            margin: Default::default(),
                            align_cross_override: None,
                            allow_fixed_compress: false,
                        },
                        inner,
                    ),
                ],
            );
            SurfaceNode::container(
                Self::OUTER_ID,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    initial_offset: Some(Vector2::new(0.0, 10.0)),
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::fill(content)],
            )
            .on_offset_settled(|offset| (Self::OUTER_ID, offset))
        }
    }

    impl RuntimeBridge<(u64, Vector2)> for ReentrantFocusRevealBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<(u64, Vector2)>> {
            let outer = self.outer();
            let root = if self.moved {
                SurfaceNode::stack(
                    100,
                    vec![
                        SurfaceChild::fill(outer),
                        SurfaceChild::fill(SurfaceNode::widget(
                            ButtonWidget::new(
                                Self::TARGET_ID,
                                "Reparented target",
                                WidgetSizing::fixed(Vector2::new(80.0, 30.0)),
                            ),
                            WidgetMessageMapper::none(),
                        )),
                    ],
                )
            } else {
                outer
            };
            crate::runtime::test_arc_surface(UiSurface::new(root))
        }

        fn reduce_message(&mut self, message: (u64, Vector2)) {
            self.settled.push(message);
            if message.0 == Self::INNER_ID {
                self.moved = true;
            }
        }
    }

    #[derive(Default)]
    struct SiblingFocusRevealBridge {
        settled: Vec<(u64, Vector2)>,
    }

    impl SiblingFocusRevealBridge {
        const FIRST_ID: u64 = 1;
        const FIRST_CONTENT_ID: u64 = 2;
        const SECOND_ID: u64 = 3;
        const SECOND_CONTENT_ID: u64 = 4;
        const OUTSIDE_ID: u64 = 10;

        fn scroll(id: u64, content_id: u64, initial_offset: f32) -> SurfaceNode<(u64, Vector2)> {
            SurfaceNode::container(
                id,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    initial_offset: Some(Vector2::new(0.0, initial_offset)),
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::fill(SurfaceNode::widget(
                    TextWidget::new(
                        content_id,
                        "Tall",
                        WidgetSizing::fixed(Vector2::new(100.0, 400.0)),
                    ),
                    WidgetMessageMapper::none(),
                ))],
            )
            .on_offset_settled(move |offset| (id, offset))
        }
    }

    impl RuntimeBridge<(u64, Vector2)> for SiblingFocusRevealBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<(u64, Vector2)>> {
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::stack(
                9,
                vec![
                    SurfaceChild::fill(Self::scroll(Self::FIRST_ID, Self::FIRST_CONTENT_ID, 40.0)),
                    SurfaceChild::fill(Self::scroll(
                        Self::SECOND_ID,
                        Self::SECOND_CONTENT_ID,
                        60.0,
                    )),
                    SurfaceChild::fill(SurfaceNode::widget(
                        ButtonWidget::new(
                            Self::OUTSIDE_ID,
                            "Outside",
                            WidgetSizing::fixed(Vector2::new(80.0, 24.0)),
                        ),
                        WidgetMessageMapper::none(),
                    )),
                ],
            )))
        }

        fn reduce_message(&mut self, message: (u64, Vector2)) {
            self.settled.push(message);
        }
    }

    struct OverlappingScrollBridge;

    impl OverlappingScrollBridge {
        const TOP_ID: u64 = 1;
        const BOTTOM_ID: u64 = 3;

        fn scroll(id: u64, content_id: u64) -> SurfaceNode<()> {
            SurfaceNode::container(
                id,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::fill(SurfaceNode::widget(
                    TextWidget::new(
                        content_id,
                        "Overlapping content",
                        WidgetSizing::fixed(Vector2::new(400.0, 400.0)),
                    ),
                    WidgetMessageMapper::none(),
                ))],
            )
        }
    }

    impl RuntimeBridge<()> for OverlappingScrollBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::stack(
                9,
                vec![
                    SurfaceChild::fill(Self::scroll(Self::TOP_ID, 2)),
                    SurfaceChild::fill(Self::scroll(Self::BOTTOM_ID, 4)),
                ],
            )))
        }

        fn reduce_message(&mut self, _message: ()) {}
    }

    struct NestedWheelBridge {
        settled: std::rc::Rc<std::cell::RefCell<Vec<(u64, Vector2)>>>,
    }

    impl NestedWheelBridge {
        const OUTER_ID: u64 = 1;
        const INNER_ID: u64 = 3;

        fn new(settled: std::rc::Rc<std::cell::RefCell<Vec<(u64, Vector2)>>>) -> Self {
            Self { settled }
        }
    }

    impl RuntimeBridge<(u64, Vector2)> for NestedWheelBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<(u64, Vector2)>> {
            let inner = SurfaceNode::container(
                Self::INNER_ID,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::fill(SurfaceNode::widget(
                    TextWidget::new(4, "Inner", WidgetSizing::fixed(Vector2::new(80.0, 400.0))),
                    WidgetMessageMapper::none(),
                ))],
            )
            .on_offset_settled(|offset| (Self::INNER_ID, offset));
            let outer_content = SurfaceNode::column(
                5,
                0.0,
                vec![
                    SurfaceChild::new(
                        SlotParams {
                            size_main: SizeModeMain::Fixed(80.0),
                            size_cross: SizeModeCross::Fill,
                            constraints: Constraints::unconstrained(),
                            margin: Default::default(),
                            align_cross_override: None,
                            allow_fixed_compress: false,
                        },
                        inner,
                    ),
                    SurfaceChild::new(
                        SlotParams {
                            size_main: SizeModeMain::Intrinsic,
                            size_cross: SizeModeCross::Fill,
                            constraints: Constraints::unconstrained(),
                            margin: Default::default(),
                            align_cross_override: None,
                            allow_fixed_compress: false,
                        },
                        SurfaceNode::widget(
                            TextWidget::new(
                                6,
                                "Outer continuation",
                                WidgetSizing::fixed(Vector2::new(80.0, 400.0)),
                            ),
                            WidgetMessageMapper::none(),
                        ),
                    ),
                ],
            );
            let outer = SurfaceNode::container(
                Self::OUTER_ID,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Intrinsic,
                        size_cross: SizeModeCross::Fill,
                        constraints: Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    outer_content,
                )],
            )
            .on_offset_settled(|offset| (Self::OUTER_ID, offset));
            crate::runtime::test_arc_surface(UiSurface::new(outer))
        }

        fn reduce_message(&mut self, message: (u64, Vector2)) {
            self.settled.borrow_mut().push(message);
        }
    }

    impl PaddedScrollBridge {
        const CONTENT_ID: u64 = 2;
    }

    impl RuntimeBridge<()> for PaddedScrollBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
                1,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    padding: crate::layout::Insets::all(4.0),
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Intrinsic,
                        size_cross: SizeModeCross::Fill,
                        constraints: Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    SurfaceNode::widget(
                        TextWidget::new(
                            Self::CONTENT_ID,
                            "Tall",
                            WidgetSizing::fixed(Vector2::new(80.0, 400.0)),
                        ),
                        WidgetMessageMapper::none(),
                    ),
                )],
            )))
        }

        fn reduce_message(&mut self, _message: ()) {}
    }

    struct ScrollInputRefreshBridge {
        policy: ScrollPolicy,
        initial: Vector2,
        controlled: Option<Controlled<Vector2>>,
        request: Option<ScrollRequest>,
        scroll: bool,
    }

    impl RuntimeBridge<()> for ScrollInputRefreshBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            if !self.scroll {
                return crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
                    TextWidget::new(
                        1,
                        "Replacement",
                        WidgetSizing::fixed(Vector2::new(80.0, 24.0)),
                    ),
                    WidgetMessageMapper::none(),
                )));
            }
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
                1,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    scroll_policy: self.policy,
                    initial_offset: Some(self.initial),
                    controlled_offset: self.controlled,
                    scroll_request: self.request.clone(),
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Intrinsic,
                        size_cross: SizeModeCross::Fill,
                        constraints: Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    SurfaceNode::widget(
                        TextWidget::new(2, "Tall", WidgetSizing::fixed(Vector2::new(400.0, 400.0))),
                        WidgetMessageMapper::none(),
                    ),
                )],
            )))
        }

        fn reduce_message(&mut self, _message: ()) {}
    }

    #[derive(Clone, Debug)]
    struct WheelSiblingWidget {
        common: WidgetCommon,
    }

    impl WheelSiblingWidget {
        fn new(id: u64) -> Self {
            Self {
                common: WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(100.0, 80.0))),
            }
        }
    }

    impl Widget for WheelSiblingWidget {
        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }

        fn handle_input(
            &mut self,
            _bounds: Rect,
            _input: crate::widgets::WidgetInput,
        ) -> Option<WidgetOutput> {
            None
        }

        fn handle_wheel_sample(
            &mut self,
            _bounds: Rect,
            _position: Point,
            _sample: crate::widgets::WheelSample,
        ) -> Option<WidgetOutput> {
            Some(WidgetOutput::typed(String::from("sibling-wheel")))
        }

        fn accepts_wheel_input(&self) -> bool {
            true
        }

        fn append_paint(
            &self,
            _primitives: &mut Vec<crate::runtime::PaintPrimitive>,
            _bounds: Rect,
            _layout: &LayoutOutput,
            _theme: &ThemeTokens,
        ) {
        }
    }

    #[derive(Default)]
    struct WheelSiblingBridge {
        settled: usize,
        sibling_wheels: usize,
    }

    impl RuntimeBridge<String> for WheelSiblingBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<String>> {
            let scroll = SurfaceNode::container(
                1,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::fill(SurfaceNode::widget(
                    TextWidget::new(2, "Tall", WidgetSizing::fixed(Vector2::new(80.0, 400.0))),
                    WidgetMessageMapper::none(),
                ))],
            )
            .on_offset_settled(|_| String::from("settled"));
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::column(
                9,
                0.0,
                vec![
                    SurfaceChild::new(
                        SlotParams {
                            size_main: SizeModeMain::Fixed(80.0),
                            size_cross: SizeModeCross::Fill,
                            constraints: Constraints::unconstrained(),
                            margin: Default::default(),
                            align_cross_override: None,
                            allow_fixed_compress: false,
                        },
                        scroll,
                    ),
                    SurfaceChild::new(
                        SlotParams {
                            size_main: SizeModeMain::Fixed(80.0),
                            size_cross: SizeModeCross::Fill,
                            constraints: Constraints::unconstrained(),
                            margin: Default::default(),
                            align_cross_override: None,
                            allow_fixed_compress: false,
                        },
                        SurfaceNode::widget(
                            WheelSiblingWidget::new(7),
                            WidgetMessageMapper::typed(|message: String| message),
                        ),
                    ),
                ],
            )))
        }

        fn reduce_message(&mut self, message: String) {
            match message.as_str() {
                "settled" => self.settled += 1,
                "sibling-wheel" => self.sibling_wheels += 1,
                _ => {}
            }
        }
    }

    struct WheelSettlementBridge {
        settled: usize,
        scroll: bool,
    }

    impl Default for WheelSettlementBridge {
        fn default() -> Self {
            Self {
                settled: 0,
                scroll: true,
            }
        }
    }

    impl RuntimeBridge<String> for WheelSettlementBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<String>> {
            if !self.scroll {
                return crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
                    TextWidget::new(
                        1,
                        "Replacement",
                        WidgetSizing::fixed(Vector2::new(80.0, 24.0)),
                    ),
                    WidgetMessageMapper::none(),
                )));
            }
            crate::runtime::test_arc_surface(UiSurface::new(
                SurfaceNode::container(
                    1,
                    ContainerPolicy {
                        kind: ContainerKind::ScrollView,
                        overflow: OverflowPolicy::Scroll,
                        ..ContainerPolicy::default()
                    },
                    vec![SurfaceChild::new(
                        SlotParams {
                            size_main: SizeModeMain::Intrinsic,
                            size_cross: SizeModeCross::Fill,
                            constraints: Constraints::unconstrained(),
                            margin: Default::default(),
                            align_cross_override: None,
                            allow_fixed_compress: false,
                        },
                        SurfaceNode::widget(
                            TextWidget::new(
                                2,
                                "Tall",
                                WidgetSizing::fixed(Vector2::new(80.0, 400.0)),
                            ),
                            WidgetMessageMapper::none(),
                        ),
                    )],
                )
                .on_offset_settled(|_| String::from("settled")),
            ))
        }

        fn reduce_message(&mut self, message: String) {
            if message == "settled" {
                self.settled += 1;
            }
        }
    }

    struct RequestSettlementBridge {
        request: ScrollRequest,
        initial: Vector2,
        policy: ScrollPolicy,
        settled: Vec<Vector2>,
    }

    impl RuntimeBridge<Vector2> for RequestSettlementBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<Vector2>> {
            crate::runtime::test_arc_surface(UiSurface::new(
                SurfaceNode::container(
                    1,
                    ContainerPolicy {
                        kind: ContainerKind::ScrollView,
                        overflow: OverflowPolicy::Scroll,
                        scroll_policy: self.policy,
                        initial_offset: Some(self.initial),
                        ..ContainerPolicy::default()
                    },
                    vec![SurfaceChild::new(
                        SlotParams {
                            size_main: SizeModeMain::Intrinsic,
                            size_cross: SizeModeCross::Fill,
                            constraints: Constraints::unconstrained(),
                            margin: Default::default(),
                            align_cross_override: None,
                            allow_fixed_compress: false,
                        },
                        SurfaceNode::widget(
                            TextWidget::new(
                                2,
                                "Tall",
                                WidgetSizing::fixed(Vector2::new(400.0, 400.0)),
                            ),
                            WidgetMessageMapper::none(),
                        ),
                    )],
                )
                .scroll_request(self.request.clone())
                .on_offset_settled(|offset| offset),
            ))
        }

        fn reduce_message(&mut self, message: Vector2) {
            self.settled.push(message);
        }
    }
}
