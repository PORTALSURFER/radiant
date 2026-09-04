use super::super::{SurfaceRuntime, SurfaceTraversalIndex};
use crate::gui::types::{Point, Rect};
use crate::{
    gui::layout_core::{ScrollRuntimeState, resolve_scroll_alignment},
    gui::types::Vector2,
    layout::{LayoutDiagnosticCode, LayoutNode, ScrollEdge, ScrollTarget},
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
            for (node_id, _proposed_offset) in settled {
                let offset = self.layout_state.scroll_offset(node_id);
                if let Some(message) = self.surface.root().offset_settled(node_id, offset) {
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
            .is_some_and(|(id, _)| !live.contains(&id))
        {
            self.interaction.wheel.pending_scroll_settlement = None;
            self.interaction.wheel.scroll_settlement_deadline = None;
        }
        self.layout_state
            .scroll_runtime
            .retain(|id, _| live.contains(id));
        self.layout_state
            .scroll_offsets
            .retain(|id, _| live.contains(id));
        for (id, _content_id, policy) in declarations {
            let entry = self
                .layout_state
                .scroll_runtime
                .entry(id)
                .or_insert_with(|| ScrollRuntimeState {
                    mount_generation: self.layout_state_generation.max(1),
                    ..ScrollRuntimeState::default()
                });
            if !entry.initial_seeded {
                if let Some(offset) = policy
                    .initial_offset
                    .filter(|offset| offset.x.is_finite() && offset.y.is_finite())
                {
                    self.layout_state
                        .scroll_offsets
                        .insert(id, Vector2::new(offset.x.max(0.0), offset.y.max(0.0)));
                }
                entry.initial_seeded = true;
            }
            if let Some(controlled) = policy.controlled_offset.filter(|value| {
                value.generation() != u64::MAX
                    && value.value().x.is_finite()
                    && value.value().y.is_finite()
            }) {
                if entry
                    .controlled_generation
                    .is_none_or(|generation| controlled.generation() > generation)
                {
                    let value = *controlled.value();
                    self.layout_state
                        .scroll_offsets
                        .insert(id, Vector2::new(value.x.max(0.0), value.y.max(0.0)));
                    entry.controlled_generation = Some(controlled.generation());
                }
            }
        }
    }

    /// Resolve finite committed rectangles and edges and consume each request
    /// once. Keyed requests remain unavailable until a committed materialized
    /// key index is provided by the virtual-layout consumer.
    fn apply_declarative_scroll_requests(&mut self) -> Vec<(crate::layout::NodeId, Vector2)> {
        let mut requests = Vec::new();
        collect_scroll_requests(&self.layout_root, &mut requests);
        let mut changed = Vec::new();
        for (id, policy, child_id) in requests {
            let Some(request) = policy.scroll_request.as_ref() else {
                continue;
            };
            let already_consumed = self
                .layout_state
                .scroll_runtime
                .get(&id)
                .and_then(|state| state.request_generation)
                .is_some_and(|generation| request.generation <= generation);
            if request.generation == u64::MAX || already_consumed {
                continue;
            }
            let Some(viewport) = self
                .layout
                .viewport_bounds
                .get(&id)
                .or_else(|| self.layout.rects.get(&id))
                .copied()
            else {
                continue;
            };
            let Some(content) = self.layout.rects.get(&child_id).copied() else {
                continue;
            };
            let current = self.layout_state.scroll_offset(id);
            let (target_x, target_y, target_w, target_h) = match &request.target {
                ScrollTarget::Keyed(key) => {
                    let Some((owner, payload_id)) =
                        self.virtual_layout.materialized_key_payload(key)
                    else {
                        continue;
                    };
                    if owner != id || !self.layout.rects.contains_key(&payload_id) {
                        continue;
                    }
                    let Some(rect) = self.layout.rects.get(&payload_id).copied() else {
                        continue;
                    };
                    if !rect.is_finite() || !rect.has_finite_positive_area() {
                        continue;
                    }
                    (rect.min.x, rect.min.y, rect.width(), rect.height())
                }
                ScrollTarget::Rect(rect) if rect.has_finite_positive_area() => {
                    (rect.min.x, rect.min.y, rect.width(), rect.height())
                }
                ScrollTarget::Edge(edge) => match edge {
                    ScrollEdge::Top => (current.x, 0.0, 0.0, 0.0),
                    ScrollEdge::Bottom => (current.x, content.height(), 0.0, 0.0),
                    ScrollEdge::Left | ScrollEdge::Start => (0.0, current.y, 0.0, 0.0),
                    ScrollEdge::Right | ScrollEdge::End => (content.width(), current.y, 0.0, 0.0),
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
                let state = self.layout_state.scroll_runtime.entry(id).or_default();
                state.request_generation = Some(request.generation);
                if next != current {
                    self.layout_state
                        .scroll_offsets
                        .insert(id, Vector2::new(next.x.max(0.0), next.y.max(0.0)));
                    changed.push((id, next));
                }
            }
        }
        changed
    }

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

fn collect_scroll_declarations<'a>(
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
    if container.policy.kind == crate::layout::ContainerKind::ScrollView {
        if let Some(child) = container.children.first() {
            output.push((container.id, child.child.id(), &container.policy));
        }
    }
    for child in &container.children {
        collect_scroll_declarations(&child.child, output);
    }
}

fn collect_scroll_requests<'a>(
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
    if container.policy.kind == crate::layout::ContainerKind::ScrollView {
        if let Some(child) = container.children.first() {
            output.push((container.id, &container.policy, child.child.id()));
        }
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

    struct PaddedScrollBridge;

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
                        TextWidget::new(2, "Tall", WidgetSizing::fixed(Vector2::new(80.0, 400.0))),
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
