//! Inert admission for one fresh ordinary-surface preparation candidate.
//!
//! This module deliberately stops before layout publication, widget-state
//! synchronization, declarative-owner reconciliation, or any other active
//! runtime commit.  Its only authority is the exact raw classifier result and
//! the runtime-issued request witness captured at admission.

#![allow(dead_code)]

use super::SurfaceRuntime;
use crate::gui::layout_core::{
    LayoutAuthorityEvidence, LayoutStateAuthorityOwner, MountedLayoutSourceAuthorityOwner,
    RootLayoutAuthorityOwner,
};
use crate::gui::types::Rect;
use crate::layout::LayoutDebugOptions;
use crate::runtime::{
    RepaintScope, RuntimeBridge, RuntimeLifecyclePhase, SurfaceRuntimeProjection, UiSurface,
    WindowEnvironment,
    surface::{
        DEFAULT_VIEW_DELTA_SCRATCH_CAPACITY, RefreshExecutionDecision, SourceMetadata,
        SourceTopology, SourceTraversalIndex, SurfaceTraversalIndex as ProjectedTraversalIndex,
        ViewDelta, ViewDeltaEffect, ViewDeltaScratch, WidgetReplacementPlan, classify_view_delta,
    },
};

/// Exact runtime authority issued for one non-paint fresh-surface request.
///
/// The fields are private so a caller cannot manufacture a witness outside
/// this controller module.  A newer request replaces the stored witness and
/// makes every earlier copy stale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::runtime::controller) struct FreshSurfaceRefreshRequest {
    runtime_identity: u64,
    lifecycle_phase: RuntimeLifecyclePhase,
    lifecycle_transition_sequence: u64,
    active_surface_generation: u64,
    request_revision: u64,
    scope: RepaintScope,
    viewport: Rect,
    window_environment: WindowEnvironment,
}

impl FreshSurfaceRefreshRequest {
    fn exactly_matches(self, other: Self) -> bool {
        self.runtime_identity == other.runtime_identity
            && self.lifecycle_phase == other.lifecycle_phase
            && self.lifecycle_transition_sequence == other.lifecycle_transition_sequence
            && self.active_surface_generation == other.active_surface_generation
            && self.request_revision == other.request_revision
            && self.scope == other.scope
            && same_rect_bits(self.viewport, other.viewport)
            && self.window_environment == other.window_environment
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FreshSurfacePreparationAuthority {
    request: FreshSurfaceRefreshRequest,
    active_root_id: u64,
    active_root_state_version: u64,
    layout_root_authority: LayoutAuthorityEvidence<RootLayoutAuthorityOwner>,
    layout_state_authority: LayoutAuthorityEvidence<LayoutStateAuthorityOwner>,
    mounted_layout_source_authority: LayoutAuthorityEvidence<MountedLayoutSourceAuthorityOwner>,
    mounted_layout_source_present: bool,
    layout_state_generation: u64,
    layout_debug_options: LayoutDebugOptions,
}

/// Candidate-owned, single-use-by-convention fresh surface preparation.
///
/// This type intentionally does not implement `Clone`.  It owns the fresh
/// surface, one co-derived projection, the raw classifier result, its private
/// execution decision, and an inert widget replacement plan.  Dropping it is
/// an observational discard; there is no commit method in this slice.
pub(in crate::runtime::controller) struct FreshSurfacePreparationCandidate<Message> {
    surface: UiSurface<Message>,
    layout_root: crate::layout::LayoutNode,
    traversal: ProjectedTraversalIndex<Message>,
    source: SourceTraversalIndex,
    view_delta: ViewDelta,
    execution: RefreshExecutionDecision,
    replacement_plan: WidgetReplacementPlan,
    view_delta_scratch: ViewDeltaScratch,
    authority: FreshSurfacePreparationAuthority,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Issue an exact private witness for a fresh-surface request.
    ///
    /// Paint-only work never creates this authority.  Checked generations are
    /// bounded deliberately: once any generation would reach the reserved
    /// exhaustion value, later preparation is permanently vetoed.
    pub(in crate::runtime::controller) fn issue_fresh_surface_refresh_request(
        &mut self,
        scope: RepaintScope,
    ) -> Option<FreshSurfaceRefreshRequest> {
        if !scope.refreshes_projection()
            || self.lifecycle_phase() != RuntimeLifecyclePhase::Running
            || self.fresh_surface_authority_exhausted
        {
            return None;
        }

        let Some(request_revision) = self.fresh_surface_request_revision.checked_add(1) else {
            self.fresh_surface_authority_exhausted = true;
            return None;
        };
        if !valid_checked_generation(request_revision)
            || !valid_checked_generation(self.fresh_surface_active_generation)
            || !valid_checked_generation(self.lifecycle_transition_sequence())
        {
            self.fresh_surface_authority_exhausted = true;
            return None;
        }

        let request = FreshSurfaceRefreshRequest {
            runtime_identity: self.runtime_identity(),
            lifecycle_phase: self.lifecycle_phase(),
            lifecycle_transition_sequence: self.lifecycle_transition_sequence(),
            active_surface_generation: self.fresh_surface_active_generation,
            request_revision,
            scope,
            viewport: self.viewport,
            window_environment: self.window_environment,
        };
        self.fresh_surface_request_revision = request_revision;
        self.fresh_surface_request = Some(request);
        Some(request)
    }

    /// Admit one already-owned fresh surface without consulting the bridge.
    ///
    /// Every fallible check happens before the candidate is returned.  No
    /// active traversal/source scratch is borrowed mutably, and no widget,
    /// mapper, owner, interaction, diagnostic, paint, or repaint state is
    /// changed by this method.
    pub(in crate::runtime::controller) fn prepare_fresh_surface(
        &self,
        surface: UiSurface<Message>,
        request: FreshSurfaceRefreshRequest,
    ) -> Option<FreshSurfacePreparationCandidate<Message>> {
        if !self.fresh_surface_request_is_current(request)
            || self.fresh_surface_authority_exhausted
            || self.lifecycle_phase() != RuntimeLifecyclePhase::Running
            || !self.virtual_layout.is_empty()
            || !request.scope.refreshes_projection()
            || self.layout_authority_exhausted
        {
            return None;
        }

        if !same_rect_bits(self.viewport, request.viewport)
            || self.window_environment != request.window_environment
            || surface.window_environment() != self.window_environment
            || self.surface.window_environment() != self.window_environment
            || self.layout_root != self.surface.layout_node()
        {
            return None;
        }

        let SurfaceRuntimeProjection {
            layout_root,
            traversal,
            source,
        } = surface.runtime_projection();
        if layout_root != surface.layout_node()
            || !traversal.virtual_layout_registrations.is_empty()
            || !self.virtual_layout.is_empty()
            || !traversal_matches_runtime(&traversal, &self.traversal)
            || !source_indices_match(&source, &self.scratch.projection_source)
        {
            return None;
        }

        let mut view_delta_scratch =
            ViewDeltaScratch::with_capacity(DEFAULT_VIEW_DELTA_SCRATCH_CAPACITY);
        if !view_delta_scratch.has_identity_capacity() {
            return None;
        }
        let view_delta = classify_view_delta(&self.surface, &surface, &mut view_delta_scratch);
        if !complete_raw_preparation_evidence(&view_delta) {
            return None;
        }

        let execution = RefreshExecutionDecision::from_view_delta(request.scope, &view_delta);
        let replacement_plan = self.surface.plan_widget_replacements(
            &surface,
            &self.traversal.widgets.stateful_order,
            &self.traversal.widgets.hit_order,
            &traversal.widget_paint_order,
            &traversal.widget_paths,
            &self.traversal.widgets.paths.current,
        );
        let authority = FreshSurfacePreparationAuthority {
            request,
            active_root_id: self.layout_root.id(),
            active_root_state_version: self.layout_root.state_version(),
            layout_root_authority: self.layout_root_authority,
            layout_state_authority: self.layout_state_authority,
            mounted_layout_source_authority: self.mounted_layout_source_authority,
            mounted_layout_source_present: self.mounted_layout_source_present,
            layout_state_generation: self.layout_state_generation,
            layout_debug_options: self.layout_debug_options,
        };

        Some(FreshSurfacePreparationCandidate {
            surface,
            layout_root,
            traversal,
            source,
            view_delta,
            execution,
            replacement_plan,
            view_delta_scratch,
            authority,
        })
    }

    /// Advance the active-surface generation at the existing refresh commit
    /// boundary.  This is authority bookkeeping only; it does not alter the
    /// established refresh behavior or its fallback decisions.
    pub(super) fn advance_fresh_surface_active_generation(&mut self) {
        if self.fresh_surface_authority_exhausted {
            return;
        }
        let Some(next_generation) = self.fresh_surface_active_generation.checked_add(1) else {
            self.fresh_surface_authority_exhausted = true;
            return;
        };
        if !valid_checked_generation(next_generation) {
            self.fresh_surface_authority_exhausted = true;
            return;
        }
        self.fresh_surface_active_generation = next_generation;
    }

    fn fresh_surface_request_is_current(&self, request: FreshSurfaceRefreshRequest) -> bool {
        self.fresh_surface_request
            .is_some_and(|current| current.exactly_matches(request))
            && valid_checked_generation(request.request_revision)
            && valid_checked_generation(self.fresh_surface_request_revision)
            && request.request_revision == self.fresh_surface_request_revision
            && request.runtime_identity == self.runtime_identity()
            && request.lifecycle_phase == RuntimeLifecyclePhase::Running
            && request.lifecycle_phase == self.lifecycle_phase()
            && request.lifecycle_transition_sequence == self.lifecycle_transition_sequence()
            && request.active_surface_generation == self.fresh_surface_active_generation
            && same_rect_bits(request.viewport, self.viewport)
            && request.window_environment == self.window_environment
            && request.scope.refreshes_projection()
    }
}

impl<Message> FreshSurfacePreparationCandidate<Message> {
    /// Return whether the candidate remains admissible.  This method is
    /// observational and has no publication side effects.
    pub(in crate::runtime::controller) fn is_current<Bridge>(
        &self,
        runtime: &SurfaceRuntime<Bridge, Message>,
    ) -> bool
    where
        Bridge: RuntimeBridge<Message>,
    {
        let authority = self.authority;
        if runtime.fresh_surface_authority_exhausted
            || !valid_checked_generation(authority.request.request_revision)
            || !valid_checked_generation(authority.request.active_surface_generation)
            || !valid_checked_generation(authority.request.lifecycle_transition_sequence)
            || !runtime.fresh_surface_request_is_current(authority.request)
            || authority.request.runtime_identity != runtime.runtime_identity()
            || authority.request.lifecycle_phase != RuntimeLifecyclePhase::Running
            || authority.request.lifecycle_phase != runtime.lifecycle_phase()
            || authority.request.lifecycle_transition_sequence
                != runtime.lifecycle_transition_sequence()
            || authority.request.active_surface_generation
                != runtime.fresh_surface_active_generation
            || !same_rect_bits(authority.request.viewport, runtime.viewport)
            || authority.request.window_environment != runtime.window_environment
            || runtime.surface.window_environment() != runtime.window_environment
            || runtime.layout_authority_exhausted
            || authority.layout_root_authority != runtime.layout_root_authority
            || authority.layout_state_authority != runtime.layout_state_authority
            || authority.mounted_layout_source_authority != runtime.mounted_layout_source_authority
            || authority.mounted_layout_source_present != runtime.mounted_layout_source_present
            || authority.layout_state_generation != runtime.layout_state_generation
            || authority.layout_debug_options != runtime.layout_debug_options
            || authority.active_root_id != runtime.layout_root.id()
            || authority.active_root_state_version != runtime.layout_root.state_version()
            || runtime.layout_root != runtime.surface.layout_node()
            || self.surface.window_environment() != authority.request.window_environment
            || self.layout_root != self.surface.layout_node()
            || !self.traversal.virtual_layout_registrations.is_empty()
            || !self.source_matches_active(runtime)
            || !traversal_matches_runtime(&self.traversal, &runtime.traversal)
        {
            return false;
        }
        true
    }

    /// Explicitly abandon the candidate.  Dropping the candidate is equally
    /// inert; this spelling makes the discard boundary testable without a
    /// consumer or commit operation.
    pub(in crate::runtime::controller) fn discard(self) {}

    fn source_matches_active<Bridge>(&self, runtime: &SurfaceRuntime<Bridge, Message>) -> bool
    where
        Bridge: RuntimeBridge<Message>,
    {
        source_indices_match(&self.source, &runtime.scratch.projection_source)
    }
}

fn valid_checked_generation(value: u64) -> bool {
    value != 0 && value != u64::MAX
}

fn same_rect_bits(first: Rect, second: Rect) -> bool {
    first.min.x.to_bits() == second.min.x.to_bits()
        && first.min.y.to_bits() == second.min.y.to_bits()
        && first.max.x.to_bits() == second.max.x.to_bits()
        && first.max.y.to_bits() == second.max.y.to_bits()
}

fn complete_raw_preparation_evidence(delta: &ViewDelta) -> bool {
    if delta.conservative
        || delta.omitted_events != 0
        || delta.truncated_paths
        || delta.effect == ViewDeltaEffect::Structural
        || delta
            .events
            .iter()
            .flatten()
            .any(|event| event.effect == ViewDeltaEffect::Structural || event.path.truncated)
        || delta.diagnostic.conservative
        || delta.diagnostic.omitted_events != 0
        || delta.diagnostic.truncated_paths
        || delta.diagnostic.effect == ViewDeltaEffect::Structural
        || delta
            .diagnostic
            .events
            .iter()
            .flatten()
            .any(|event| event.effect == ViewDeltaEffect::Structural || event.path.truncated)
    {
        return false;
    }
    true
}

fn traversal_matches_runtime<Message>(
    candidate: &ProjectedTraversalIndex<Message>,
    active: &super::traversal_state::RuntimeTraversalState<Message>,
) -> bool {
    candidate.widget_paint_order == active.widgets.hit_order
        && candidate.focusable_widget_order == active.widgets.focusable.order()
        && candidate.keyboard_focus_order == active.widgets.keyboard_focus.order()
        && candidate.pointer_hit_order == active.widgets.pointer.order()
        && candidate.wheel_hit_order == active.widgets.wheel.order()
        && candidate.wheel_target_order == active.widgets.wheel_targets.order()
        && candidate.native_file_drop_hit_order == active.widgets.native_file_drop.order()
        && candidate.stateful_widget_order == active.widgets.stateful_order
        && candidate.widget_paths == active.widgets.paths.current
        && candidate.container_hover_suppression == active.widgets.paths.container_hover_suppression
        && candidate.styled_container_order == active.containers.styled.order()
        && candidate.scroll_container_order == active.containers.scroll.order()
        && candidate.widget_clip_ancestors == active.widgets.paths.clip_ancestors
        && candidate.container_clip_ancestors == active.containers.clip_ancestors
        && candidate.scroll_content_by_container == active.containers.scroll_content_by_container
        && layout_interactions_match(
            &candidate.layout_interactions,
            &active.containers.layout_interactions,
        )
        && candidate.split_pane_runtime == active.containers.split_pane_runtime
        && candidate.split_pane_dividers == active.containers.split_pane_dividers
        && candidate.virtual_layout_registrations.is_empty()
        && active.containers.virtual_layout_registrations.is_empty()
}

fn layout_interactions_match<Message>(
    candidate: &[crate::runtime::surface::SurfaceLayoutInteractionRecord<Message>],
    active: &[crate::runtime::surface::SurfaceLayoutInteractionRecord<Message>],
) -> bool {
    candidate.len() == active.len()
        && candidate.iter().zip(active).all(|(candidate, active)| {
            candidate.id == active.id
                && candidate.contract_version == active.contract_version
                && candidate.revision == active.revision
                && candidate.state.as_ref().map(|state| state.id())
                    == active.state.as_ref().map(|state| state.id())
                && candidate.foreign_state_declaration == active.foreign_state_declaration
        })
}

fn source_indices_match(first: &SourceTraversalIndex, second: &SourceTraversalIndex) -> bool {
    first.records.len() == second.records.len()
        && first
            .records
            .iter()
            .zip(&second.records)
            .all(|(first, second)| {
                first.node_id == second.node_id
                    && match (&first.metadata, &second.metadata) {
                        (None, None) => true,
                        (Some(first), Some(second)) => source_metadata_matches(first, second),
                        _ => false,
                    }
            })
}

fn source_metadata_matches(first: &SourceMetadata, second: &SourceMetadata) -> bool {
    first.identity == second.identity
        && first.compatibility == second.compatibility
        && source_topology_matches(&first.topology, &second.topology)
}

fn source_topology_matches(first: &SourceTopology, second: &SourceTopology) -> bool {
    first.keyed_nodes.len() == second.keyed_nodes.len()
        && first
            .keyed_nodes
            .iter()
            .zip(&second.keyed_nodes)
            .all(|(first, second)| {
                first.identity() == second.identity()
                    && first.compatibility() == second.compatibility()
                    && first.effect_owner() == second.effect_owner()
            })
        && first.overlays == second.overlays
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{IntoView, text};
    use crate::gui::types::Vector2;
    use crate::layout::{
        ContainerPolicy, LayoutOutput, SlotParams, VirtualLayoutBudget, VirtualLayoutPolicy,
        VirtualLayoutPolicyDecision, VirtualLayoutPolicyIdentity, VirtualLayoutUnavailableReason,
    };
    use crate::runtime::{
        RuntimeBridge, SurfaceChild, SurfaceNode, SurfaceRefreshCounters,
        SurfaceRefreshDiagnostics, WidgetMessageMapper,
        surface::{SourceCompatibility, SourceIdentity},
    };
    use crate::widgets::{
        TextWidget, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    };
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::Arc;

    type CallCount = Rc<Cell<usize>>;

    struct SpyBridge {
        surface: UiSurface<()>,
        pull_calls: CallCount,
        project_calls: CallCount,
    }

    type Fixture = (SurfaceRuntime<SpyBridge, ()>, CallCount, CallCount);

    impl RuntimeBridge<()> for SpyBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            self.project_calls.set(self.project_calls.get() + 1);
            crate::runtime::test_arc_surface(self.surface.clone())
        }

        fn pull_surface(&mut self) -> UiSurface<()> {
            self.pull_calls.set(self.pull_calls.get() + 1);
            self.surface.clone()
        }
    }

    fn runtime_for_surface(surface: UiSurface<()>) -> Fixture {
        let pull_calls = Rc::new(Cell::new(0));
        let project_calls = Rc::new(Cell::new(0));
        let runtime = SurfaceRuntime::new(
            SpyBridge {
                surface,
                pull_calls: Rc::clone(&pull_calls),
                project_calls: Rc::clone(&project_calls),
            },
            Vector2::new(120.0, 80.0),
        );
        (runtime, pull_calls, project_calls)
    }

    fn ordinary_surface(intrinsic: Vector2) -> UiSurface<()> {
        UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![SurfaceChild::new(
                SlotParams::fill(),
                SurfaceNode::static_widget(TextWidget::new(
                    2,
                    "ordinary",
                    WidgetSizing::fixed(intrinsic),
                )),
            )],
        ))
    }

    fn runtime_fixture() -> Fixture {
        runtime_for_surface(ordinary_surface(Vector2::new(24.0, 16.0)))
    }

    fn flat_surface(root_id: u64, widget_ids: &[u64]) -> UiSurface<()> {
        UiSurface::new(SurfaceNode::container(
            root_id,
            ContainerPolicy::default(),
            widget_ids
                .iter()
                .copied()
                .map(|id| {
                    SurfaceChild::new(
                        SlotParams::fill(),
                        SurfaceNode::static_widget(TextWidget::new(
                            id,
                            "ordinary",
                            WidgetSizing::fixed(Vector2::new(24.0, 16.0)),
                        )),
                    )
                })
                .collect(),
        ))
    }

    fn deep_surface(depth: usize, text_value: &'static str) -> UiSurface<()> {
        let mut node = SurfaceNode::static_widget(TextWidget::new(
            2,
            text_value,
            WidgetSizing::fixed(Vector2::new(24.0, 16.0)),
        ));
        for id in (0..depth).rev() {
            node = SurfaceNode::container(
                100 + id as u64,
                ContainerPolicy::default(),
                vec![SurfaceChild::fill(node)],
            );
        }
        UiSurface::new(node)
    }

    fn mapped_surface() -> UiSurface<()> {
        UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![SurfaceChild::fill(SurfaceNode::widget(
                TextWidget::new(2, "mapped", WidgetSizing::fixed(Vector2::new(24.0, 16.0))),
                WidgetMessageMapper::dynamic(|_| None),
            ))],
        ))
    }

    fn incompatible_surface() -> UiSurface<()> {
        UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![SurfaceChild::fill(SurfaceNode::static_widget(
                crate::widgets::ButtonWidget::new(
                    2,
                    "incompatible",
                    WidgetSizing::fixed(Vector2::new(24.0, 16.0)),
                ),
            ))],
        ))
    }

    fn sourced_surface(resolved_id: u64) -> UiSurface<()> {
        let widget = SurfaceNode::static_widget(TextWidget::new(
            2,
            "sourced",
            WidgetSizing::fixed(Vector2::new(24.0, 16.0)),
        ));
        let source = crate::runtime::surface::SourceMetadata::new(
            SourceIdentity {
                resolved_id,
                structural_scope: 1,
                origin: crate::application::DeclarativeIdentityOrigin::ExplicitNumericId,
            },
            SourceCompatibility::from_surface_node(&widget),
            crate::runtime::surface::SourceTopology::default(),
        );
        UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![SurfaceChild::fill(widget.with_source_metadata(source))],
        ))
    }

    fn virtual_surface() -> UiSurface<()> {
        struct EmptyPolicy;

        impl VirtualLayoutPolicy for EmptyPolicy {
            fn query(
                &self,
                _input: &crate::layout::VirtualLayoutQueryInput,
                _sink: &mut crate::layout::VirtualLayoutQuerySink,
            ) -> VirtualLayoutPolicyDecision {
                VirtualLayoutPolicyDecision::Unavailable(
                    VirtualLayoutUnavailableReason::Unsupported,
                )
            }
        }

        let view = crate::application::virtual_layout::virtual_layout_from_parts(
            crate::application::virtual_layout::VirtualLayoutParts::new(
                Rc::new(EmptyPolicy),
                VirtualLayoutPolicyIdentity::new("fresh-surface-test"),
                crate::layout::VirtualLayoutOverscan::new(0.0, 0.0).expect("overscan"),
                VirtualLayoutBudget::new(1),
                crate::runtime::VirtualLayoutRevisions::default(),
                Rc::new(|| text::<()>("shell")),
                Rc::new(|_| text::<()>("item")),
                Rc::new(|_| VirtualLayoutPolicyIdentity::new("item")),
            ),
        );
        view.into_surface()
    }

    #[derive(Clone)]
    struct RetiringWidget {
        common: WidgetCommon,
        marker: u64,
        prepare_calls: Rc<Cell<usize>>,
    }

    impl RetiringWidget {
        fn new(marker: u64, prepare_calls: Rc<Cell<usize>>) -> Self {
            Self {
                common: WidgetCommon::fixed(2, 24.0, 16.0),
                marker,
                prepare_calls,
            }
        }
    }

    impl Widget for RetiringWidget {
        fn revision(&self) -> crate::widgets::WidgetRevision {
            crate::widgets::WidgetRevision::exact((), (), self.marker, ())
        }

        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }

        fn handle_input(
            &mut self,
            _bounds: crate::gui::types::Rect,
            _input: WidgetInput,
        ) -> Option<WidgetOutput> {
            None
        }

        fn prepare_replacement(&mut self, _successor: Option<&dyn Widget>) -> Option<WidgetOutput> {
            self.prepare_calls
                .set(self.prepare_calls.get().saturating_add(1));
            Some(WidgetOutput::typed(()))
        }

        fn append_paint(
            &self,
            _primitives: &mut Vec<crate::runtime::PaintPrimitive>,
            _bounds: crate::gui::types::Rect,
            _layout: &LayoutOutput,
            _theme: &crate::theme::ThemeTokens,
        ) {
        }
    }

    fn retiring_surface(marker: u64, prepare_calls: Rc<Cell<usize>>) -> UiSurface<()> {
        UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![SurfaceChild::fill(SurfaceNode::static_widget(
                RetiringWidget::new(marker, prepare_calls),
            ))],
        ))
    }

    #[derive(Clone, Debug, PartialEq)]
    struct TraversalSnapshot {
        widget_paint_order: Vec<u64>,
        focusable: Vec<u64>,
        keyboard_focus: Vec<u64>,
        pointer: Vec<u64>,
        wheel: Vec<u64>,
        wheel_targets: Vec<crate::runtime::WheelHitTarget>,
        native_file_drop: Vec<u64>,
        stateful: Vec<u64>,
        paths: std::collections::HashMap<u64, crate::runtime::WidgetPath>,
        hover_suppression: std::collections::HashSet<u64>,
        styled: Vec<u64>,
        scroll: Vec<u64>,
        widget_clips: std::collections::HashMap<u64, crate::runtime::ClipAncestors>,
        container_clips: std::collections::HashMap<u64, crate::runtime::ClipAncestors>,
        scroll_content: std::collections::HashMap<u64, u64>,
        layout_interactions: Vec<(u64, u16, Option<crate::layout::ContainerStateId>, bool)>,
        split_runtime: Vec<crate::gui::layout_core::SplitPaneRuntimeStateInput>,
        split_dividers: Vec<crate::gui::layout_core::SplitPaneDividerDescriptor>,
    }

    #[derive(Clone)]
    struct ActiveSnapshot {
        surface_root: crate::layout::LayoutNode,
        layout_root: crate::layout::LayoutNode,
        layout: LayoutOutput,
        completed_layout: Option<super::super::CompletedLayoutContext>,
        traversal: TraversalSnapshot,
        source: crate::runtime::surface::SourceTraversalIndex,
        focus: String,
        pointer: String,
        wheel: String,
        composition: String,
        owner_counts: (u64, u64),
        diagnostics: SurfaceRefreshDiagnostics,
        view_delta_diagnostics: crate::runtime::surface::ViewDeltaDiagnostics,
        counters: SurfaceRefreshCounters,
        paint_observation: crate::runtime::PaintSegmentObservation,
        base_paint_reuse: bool,
        repaint_requested: bool,
        pending_relayout: bool,
        external_layout_dirty: bool,
    }

    fn traversal_snapshot(runtime: &SurfaceRuntime<SpyBridge, ()>) -> TraversalSnapshot {
        TraversalSnapshot {
            widget_paint_order: runtime.traversal.widgets.hit_order.clone(),
            focusable: runtime.traversal.widgets.focusable.order().to_vec(),
            keyboard_focus: runtime.traversal.widgets.keyboard_focus.order().to_vec(),
            pointer: runtime.traversal.widgets.pointer.order().to_vec(),
            wheel: runtime.traversal.widgets.wheel.order().to_vec(),
            wheel_targets: runtime.traversal.widgets.wheel_targets.order().to_vec(),
            native_file_drop: runtime.traversal.widgets.native_file_drop.order().to_vec(),
            stateful: runtime.traversal.widgets.stateful_order.clone(),
            paths: runtime.traversal.widgets.paths.current.clone(),
            hover_suppression: runtime
                .traversal
                .widgets
                .paths
                .container_hover_suppression
                .clone(),
            styled: runtime.traversal.containers.styled.order().to_vec(),
            scroll: runtime.traversal.containers.scroll.order().to_vec(),
            widget_clips: runtime.traversal.widgets.paths.clip_ancestors.clone(),
            container_clips: runtime.traversal.containers.clip_ancestors.clone(),
            scroll_content: runtime
                .traversal
                .containers
                .scroll_content_by_container
                .clone(),
            layout_interactions: runtime
                .traversal
                .containers
                .layout_interactions
                .iter()
                .map(|interaction| {
                    (
                        interaction.id,
                        interaction.contract_version,
                        interaction.state.as_ref().map(|state| state.id()),
                        interaction.foreign_state_declaration,
                    )
                })
                .collect(),
            split_runtime: runtime.traversal.containers.split_pane_runtime.clone(),
            split_dividers: runtime.traversal.containers.split_pane_dividers.clone(),
        }
    }

    fn active_snapshot(runtime: &SurfaceRuntime<SpyBridge, ()>) -> ActiveSnapshot {
        ActiveSnapshot {
            surface_root: runtime.surface.layout_node(),
            layout_root: runtime.layout_root.clone(),
            layout: runtime.layout.clone(),
            completed_layout: runtime.completed_layout,
            traversal: traversal_snapshot(runtime),
            source: runtime.scratch.projection_source.clone(),
            focus: format!("{:?}", runtime.interaction.focus),
            pointer: format!("{:?}", runtime.interaction.pointer),
            wheel: format!("{:?}", runtime.interaction.wheel),
            composition: format!("{:?}", runtime.interaction.composition),
            owner_counts: (
                runtime.declarative_owner.installation_count(),
                runtime.declarative_owner_ledger.reconciliation_count(),
            ),
            diagnostics: runtime.last_refresh_diagnostics,
            view_delta_diagnostics: runtime.last_view_delta_diagnostics,
            counters: runtime.refresh_counters,
            paint_observation: runtime.latest_paint_segment_observation,
            base_paint_reuse: runtime.base_paint_plan_reuse_eligible,
            repaint_requested: runtime.repaint_requested,
            pending_relayout: runtime.pending_current_surface_relayout,
            external_layout_dirty: runtime.external_layout_dirty,
        }
    }

    fn assert_active_snapshot_unchanged(before: &ActiveSnapshot, after: &ActiveSnapshot) {
        assert_eq!(before.surface_root, after.surface_root);
        assert_eq!(before.layout_root, after.layout_root);
        assert_eq!(before.layout, after.layout);
        assert_eq!(before.completed_layout, after.completed_layout);
        assert_eq!(before.traversal, after.traversal);
        assert!(source_indices_match(&before.source, &after.source));
        assert_eq!(before.focus, after.focus);
        assert_eq!(before.pointer, after.pointer);
        assert_eq!(before.wheel, after.wheel);
        assert_eq!(before.composition, after.composition);
        assert_eq!(before.owner_counts, after.owner_counts);
        assert_eq!(before.diagnostics, after.diagnostics);
        assert_eq!(before.view_delta_diagnostics, after.view_delta_diagnostics);
        assert_eq!(before.counters, after.counters);
        assert_eq!(before.paint_observation, after.paint_observation);
        assert_eq!(before.base_paint_reuse, after.base_paint_reuse);
        assert_eq!(before.repaint_requested, after.repaint_requested);
        assert_eq!(before.pending_relayout, after.pending_relayout);
        assert_eq!(before.external_layout_dirty, after.external_layout_dirty);
    }

    fn assert_veto(active: UiSurface<()>, candidate_surface: UiSurface<()>) {
        let (mut runtime, _, _) = runtime_for_surface(active);
        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Surface)
            .expect("request");
        let before = active_snapshot(&runtime);
        assert!(
            runtime
                .prepare_fresh_surface(candidate_surface, request)
                .is_none()
        );
        let after = active_snapshot(&runtime);
        assert_active_snapshot_unchanged(&before, &after);
    }

    #[test]
    fn positive_ordinary_preparation_is_candidate_owned_and_bridge_free() {
        let (mut runtime, pull_calls, project_calls) = runtime_fixture();
        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("running runtime should issue a request");
        let pull_before = pull_calls.get();
        let project_before = project_calls.get();
        let candidate = runtime
            .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), request)
            .expect("unchanged ordinary surface should prepare");

        assert!(candidate.is_current(&runtime));
        assert_eq!(candidate.view_delta.effect, ViewDeltaEffect::Unchanged);
        assert_eq!(pull_calls.get(), pull_before);
        assert_eq!(project_calls.get(), project_before);
        assert_eq!(candidate.layout_root, candidate.surface.layout_node());
    }

    #[test]
    fn complete_geometry_evidence_is_allowed_without_active_mutation() {
        let (mut runtime, _, _) = runtime_fixture();
        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("request");
        let before = active_snapshot(&runtime);
        let candidate = runtime
            .prepare_fresh_surface(ordinary_surface(Vector2::new(32.0, 16.0)), request)
            .expect("exact geometry evidence should prepare");
        assert_eq!(candidate.view_delta.effect, ViewDeltaEffect::Geometry);
        assert!(candidate.is_current(&runtime));
        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
        candidate.discard();
        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
    }

    #[test]
    fn stale_request_and_active_generation_are_vetoed_without_bridge_calls() {
        let (mut runtime, pull_calls, project_calls) = runtime_fixture();
        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Surface)
            .expect("request");
        let newer = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Surface)
            .expect("newer request");
        assert!(
            runtime
                .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), request)
                .is_none()
        );

        let pull_before = pull_calls.get();
        let project_before = project_calls.get();
        runtime.advance_fresh_surface_active_generation();
        assert!(
            runtime
                .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), newer)
                .is_none()
        );
        assert_eq!(pull_calls.get(), pull_before);
        assert_eq!(project_calls.get(), project_before);
    }

    #[test]
    fn lifecycle_runtime_identity_scope_viewport_and_environment_veto() {
        let (mut runtime, _, _) = runtime_fixture();
        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("request");
        let mut wrong_scope = request;
        wrong_scope.scope = RepaintScope::PaintOnly;
        assert!(
            runtime
                .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), wrong_scope)
                .is_none()
        );

        let (other_runtime, _, _) = runtime_fixture();
        assert!(
            other_runtime
                .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), request)
                .is_none()
        );

        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("new request");
        runtime.viewport.max.x += 1.0;
        assert!(
            runtime
                .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), request)
                .is_none()
        );

        let (mut runtime, _, _) = runtime_fixture();
        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("environment request");
        let mut candidate_surface = ordinary_surface(Vector2::new(24.0, 16.0));
        candidate_surface.set_window_environment(crate::runtime::WindowEnvironment::new(
            crate::theme::DpiScale::new(2.0),
            None,
            false,
            false,
        ));
        assert!(
            runtime
                .prepare_fresh_surface(candidate_surface, request)
                .is_none()
        );

        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("environment request");
        runtime.set_window_environment(crate::runtime::WindowEnvironment::new(
            crate::theme::DpiScale::new(2.0),
            None,
            false,
            false,
        ));
        assert!(
            runtime
                .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), request)
                .is_none()
        );

        let (mut runtime, _, _) = runtime_fixture();
        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("lifecycle request");
        assert!(runtime.transition_lifecycle(RuntimeLifecyclePhase::Recovering));
        assert!(
            runtime
                .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), request)
                .is_none()
        );
    }

    #[test]
    fn co_origin_and_active_path_order_root_source_and_virtual_guards_veto() {
        let (mut path_runtime, _, _) = runtime_fixture();
        let request = path_runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("path request");
        path_runtime.traversal.widgets.paths.current.clear();
        assert!(
            path_runtime
                .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), request)
                .is_none()
        );

        let (mut order_runtime, _, _) = runtime_for_surface(flat_surface(1, &[2, 3]));
        let request = order_runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("order request");
        order_runtime.traversal.widgets.hit_order.reverse();
        assert!(
            order_runtime
                .prepare_fresh_surface(flat_surface(1, &[2, 3]), request)
                .is_none()
        );

        let (mut root_runtime, _, _) = runtime_fixture();
        let request = root_runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("root request");
        root_runtime.layout_root = crate::layout::LayoutNode::widget(999, Vector2::new(1.0, 1.0));
        assert!(
            root_runtime
                .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), request)
                .is_none()
        );

        assert_veto(sourced_surface(10), sourced_surface(11));
        assert_veto(
            ordinary_surface(Vector2::new(24.0, 16.0)),
            virtual_surface(),
        );
    }

    #[test]
    fn classifier_vetoes_root_topology_incompatible_duplicate_opaque_and_revision_evidence() {
        assert_veto(
            ordinary_surface(Vector2::new(24.0, 16.0)),
            UiSurface::new(SurfaceNode::container(
                99,
                ContainerPolicy::default(),
                vec![SurfaceChild::fill(SurfaceNode::static_widget(
                    TextWidget::new(2, "ordinary", WidgetSizing::fixed(Vector2::new(24.0, 16.0))),
                ))],
            )),
        );
        assert_veto(flat_surface(1, &[2, 3]), flat_surface(1, &[3, 2]));
        assert_veto(
            ordinary_surface(Vector2::new(24.0, 16.0)),
            incompatible_surface(),
        );
        assert_veto(flat_surface(1, &[2, 2]), flat_surface(1, &[2, 2]));
        assert_veto(ordinary_surface(Vector2::new(24.0, 16.0)), mapped_surface());
    }

    #[test]
    fn classifier_vetoes_truncated_and_insufficient_identity_capacity() {
        assert_veto(deep_surface(10, "before"), deep_surface(10, "after"));

        let count = DEFAULT_VIEW_DELTA_SCRATCH_CAPACITY * 2 + 1;
        let surface = flat_surface(1, &(2..2 + count as u64).collect::<Vec<_>>());
        assert_veto(surface.clone(), surface);
    }

    #[test]
    fn checked_request_generation_exhaustion_fails_closed() {
        let (mut runtime, _, _) = runtime_fixture();
        runtime.fresh_surface_request_revision = u64::MAX - 1;
        assert!(
            runtime
                .issue_fresh_surface_refresh_request(RepaintScope::Projection)
                .is_none()
        );
        assert!(runtime.fresh_surface_authority_exhausted);

        let (mut runtime, _, _) = runtime_fixture();
        runtime.fresh_surface_active_generation = u64::MAX;
        assert!(
            runtime
                .issue_fresh_surface_refresh_request(RepaintScope::Projection)
                .is_none()
        );
        assert!(runtime.fresh_surface_authority_exhausted);
    }

    #[test]
    fn allowed_vetoed_and_discarded_preparation_preserve_active_state() {
        let (mut runtime, _, _) = runtime_fixture();
        runtime.interaction.focus.focused_widget = Some(2);
        runtime.interaction.pointer.capture = Some(2);
        runtime.interaction.wheel.managed_sequence =
            super::super::interaction_state::RuntimeManagedWheelSequenceState::Active {
                widget_id: 2,
            };
        runtime.interaction.composition.managed_composition =
            super::super::interaction_state::RuntimeManagedCompositionState::Active {
                widget_id: 2,
            };
        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("request");
        let before = active_snapshot(&runtime);
        let candidate = runtime
            .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), request)
            .expect("candidate");
        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
        candidate.discard();
        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));

        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("second request");
        let before = active_snapshot(&runtime);
        assert!(
            runtime
                .prepare_fresh_surface(mapped_surface(), request)
                .is_none()
        );
        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
    }

    #[test]
    fn retiring_widget_fixtures_have_no_callbacks_or_messages_on_prepare_discard_drop() {
        let prepare_calls = Rc::new(Cell::new(0));
        let (mut runtime, _, _) =
            runtime_for_surface(retiring_surface(1, Rc::clone(&prepare_calls)));
        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("request");
        let before = active_snapshot(&runtime);
        let candidate = runtime
            .prepare_fresh_surface(retiring_surface(2, Rc::clone(&prepare_calls)), request)
            .expect("paint-only retained widget evidence");
        assert_eq!(prepare_calls.get(), 0);
        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
        candidate.discard();
        assert_eq!(prepare_calls.get(), 0);

        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("second request");
        let candidate = runtime
            .prepare_fresh_surface(retiring_surface(3, Rc::clone(&prepare_calls)), request)
            .expect("repeat preparation");
        drop(candidate);
        assert_eq!(prepare_calls.get(), 0);

        let stale_request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("stale request");
        runtime.advance_fresh_surface_active_generation();
        assert!(
            runtime
                .prepare_fresh_surface(
                    retiring_surface(4, Rc::clone(&prepare_calls)),
                    stale_request
                )
                .is_none()
        );
        assert_eq!(prepare_calls.get(), 0);
    }

    #[test]
    fn production_refresh_and_direct_relayout_remain_separate_from_preparation() {
        let (mut runtime, pull_calls, project_calls) = runtime_fixture();
        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("request");
        let candidate = runtime
            .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), request)
            .expect("candidate");
        candidate.discard();

        let pull_before = pull_calls.get();
        let project_before = project_calls.get();
        let counters_before = runtime.refresh_counters();
        runtime.refresh_with_scope(RepaintScope::Projection);
        assert_eq!(pull_calls.get(), pull_before + 1);
        assert_eq!(project_calls.get(), project_before);
        assert_eq!(
            runtime.refresh_counters().application_projection,
            counters_before.application_projection + 1
        );

        let counters_before_relayout = runtime.refresh_counters();
        runtime.relayout();
        assert_eq!(runtime.refresh_counters(), counters_before_relayout);
        assert_eq!(runtime.layout_root, runtime.surface.layout_node());
        assert!(!runtime.scratch.projection_source.records.is_empty());
    }

    #[test]
    fn preparation_discards_stale_and_drop_without_replacement_callbacks() {
        let (mut runtime, _, _) = runtime_fixture();
        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("request");
        let candidate = runtime
            .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), request)
            .expect("candidate");
        candidate.discard();

        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("request after discard");
        let candidate = runtime
            .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), request)
            .expect("candidate after discard");
        drop(candidate);
        let candidate = runtime
            .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), request)
            .expect("same exact request can be prepared again");
        candidate.discard();
    }
}
