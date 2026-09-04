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
                layout_state
                    .scroll_offsets
                    .insert(id, Vector2::new(offset.x.max(0.0), offset.y.max(0.0)));
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
            layout_state
                .scroll_offsets
                .insert(id, Vector2::new(value.x.max(0.0), value.y.max(0.0)));
            entry.controlled_generation = Some(controlled.generation());
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
                    if policy.scroll_policy.axes.includes_horizontal() {
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
                    if policy.scroll_policy.axes.includes_horizontal() {
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
        if policy.scroll_policy.axes.includes_horizontal() {
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
        if next.x.is_finite() && next.y.is_finite() {
            let state = layout_state.scroll_runtime.entry(id).or_default();
            state.request_generation = Some(request.generation);
            if next != current {
                layout_state
                    .scroll_offsets
                    .insert(id, Vector2::new(next.x.max(0.0), next.y.max(0.0)));
                changed.push((id, current));
            }
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
        gui::types::Point,
        layout::{
            Constraints, ContainerKind, ContainerPolicy, Controlled, OverflowPolicy,
            ScrollAlignment, ScrollAxis, ScrollPolicy, ScrollRequest, ScrollTarget,
            ScrollbarPlacement, SizeModeCross, SizeModeMain, SlotParams,
        },
        runtime::{RuntimeBridge, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper},
        widgets::{TextWidget, WidgetSizing},
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
    fn focus_reveal_uses_translated_content_coordinates_at_nonzero_offset() {
        let mut runtime = SurfaceRuntime::new(FocusRevealBridge, Vector2::new(100.0, 80.0));
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
        runtime.reveal_widget_in_scroll_ancestors(FocusRevealBridge::TARGET_ID);
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

    struct FocusRevealBridge;

    impl FocusRevealBridge {
        const SCROLL_ID: u64 = 1;
        const CONTENT_ID: u64 = 2;
        const TARGET_ID: u64 = 10 + 7;
    }

    impl RuntimeBridge<()> for FocusRevealBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            let rows = (0..12)
                .map(|index| {
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
                            TextWidget::new(
                                10 + index,
                                format!("Row {index}"),
                                WidgetSizing::fixed(Vector2::new(80.0, 30.0)),
                            ),
                            WidgetMessageMapper::none(),
                        ),
                    )
                })
                .collect();
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
                Self::SCROLL_ID,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
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
                                WidgetSizing::fixed(Vector2::new(80.0, 400.0)),
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
