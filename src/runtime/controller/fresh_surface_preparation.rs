//! Inert admission for one fresh ordinary-surface preparation candidate.
//!
//! This module deliberately stops before layout publication, widget-state
//! synchronization, declarative-owner reconciliation, or any other active
//! runtime commit.  Its only authority is the exact raw classifier result and
//! the runtime-issued request witness captured at admission.

#![allow(dead_code)]

use super::SurfaceRuntime;
use super::interaction_patch::{InteractionPatchCandidate, InteractionPatchPreparation};
use super::interaction_state::RuntimeManagedPointerCaptureState;
use super::layout_state::RuntimeLayoutContainerStateCandidate;
use super::refresh::SurfaceIdentityDiagnostics;
use super::state::{
    apply_declarative_scroll_requests_for, collect_scroll_declarations, collect_scroll_requests,
    sync_declarative_scroll_inputs_for, sync_scroll_offsets_for,
};
use crate::gui::layout_core::{
    LayoutAuthorityEvidence, LayoutInputEvidence, LayoutOutput, LayoutStateAuthorityOwner,
    MountedLayoutSourceAuthorityOwner, PreparedLayoutPass, RootLayoutAuthorityOwner,
};
use crate::gui::types::Rect;
use crate::layout::{LayoutDebugOptions, LayoutNode, NodeId};
use crate::runtime::{
    ExactChangedRoots, RepaintScope, ResolvedEnvironment, RuntimeBridge, RuntimeLifecyclePhase,
    SurfacePaintPlan, SurfaceRefreshRequest, SurfaceRefreshTimings, SurfaceRuntimeProjection,
    SurfaceUpdate, UiSurface, WindowEnvironment, empty_paint_plan_for_layout,
    surface::{
        DEFAULT_VIEW_DELTA_SCRATCH_CAPACITY, PreparedWidgetStateSyncEvidence,
        PreparedWidgetStateSyncVeto, RefreshExecutionDecision, SourceTraversalIndex, SurfaceDamage,
        SurfaceTraversalIndex as ProjectedTraversalIndex, ViewDelta, ViewDeltaDiagnostics,
        ViewDeltaEffect, ViewDeltaScratch, WidgetReplacementPlan, classify_view_delta,
        source_metadata_matches,
    },
};
use crate::theme::{ResolvedAppearance, ThemeTokens};
use std::time::{Duration, Instant};

/// Exact runtime authority issued for one non-paint fresh-surface request.
///
/// The fields are private so a caller cannot manufacture a witness outside
/// this controller module.  A newer request replaces the stored witness and
/// makes every earlier copy stale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::runtime::controller) struct FreshSurfaceRefreshRequest {
    pub(in crate::runtime::controller) runtime_identity: u64,
    pub(in crate::runtime::controller) lifecycle_phase: RuntimeLifecyclePhase,
    pub(in crate::runtime::controller) lifecycle_transition_sequence: u64,
    pub(in crate::runtime::controller) active_surface_generation: u64,
    pub(in crate::runtime::controller) request_revision: u64,
    pub(in crate::runtime::controller) scope: RepaintScope,
    pub(in crate::runtime::controller) viewport: Rect,
    pub(in crate::runtime::controller) window_environment: WindowEnvironment,
}

impl FreshSurfaceRefreshRequest {
    pub(in crate::runtime::controller) fn exactly_matches(self, other: Self) -> bool {
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

/// Pure publication-authority evidence for one exact stored request.
///
/// The next active generation is computed before any authority mutation so a
/// failed preflight leaves the stored request and generation untouched.
struct FreshSurfaceRefreshAuthorityPreflight {
    request: FreshSurfaceRefreshRequest,
    next_active_surface_generation: u64,
}

/// Non-Clone marker installed after the exact request is consumed and the next
/// active generation is published.  The marker exists only during the
/// irreversible callback/publication sequence.
struct FreshSurfaceRefreshAuthorityCommit {
    request_revision: u64,
    active_surface_generation: u64,
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct FreshSurfaceLayoutAuthority {
    preparation: FreshSurfacePreparationAuthority,
    candidate_root_authority: LayoutAuthorityEvidence<RootLayoutAuthorityOwner>,
    candidate_root_id: u64,
    candidate_root_state_version: u64,
    input: LayoutInputEvidence,
    candidate_mounted_source_present: bool,
}

/// Immutable base-paint inputs captured for one candidate-only traversal.
///
/// The runtime-local fields are retained as evidence even though admission
/// requires them to be neutral.  The candidate therefore owns the exact
/// context supplied to the detached surface instead of rereading active state
/// during or after widget callbacks.
#[derive(Clone, Debug, PartialEq)]
struct FreshSurfacePaintProjectionContext {
    theme: ThemeTokens,
    environment: ResolvedEnvironment,
    appearance: ResolvedAppearance,
    hovered_container: Option<NodeId>,
    active_scroll_affordance: Option<NodeId>,
    auto_scroll_visible: Vec<NodeId>,
}

impl FreshSurfacePaintProjectionContext {
    fn capture<Bridge, Message>(
        runtime: &SurfaceRuntime<Bridge, Message>,
        candidate: &FreshSurfaceLayoutCandidate<Message>,
        appearance: ResolvedAppearance,
    ) -> Self
    where
        Bridge: RuntimeBridge<Message>,
    {
        let environment = candidate.surface.resolved_environment();
        Self {
            theme: appearance.tokens(),
            environment,
            appearance,
            hovered_container: runtime.interaction.hover.container,
            active_scroll_affordance: runtime.interaction.hover.scroll_affordance,
            auto_scroll_visible: runtime.scroll_auto_visibility(),
        }
    }

    fn is_neutral(&self) -> bool {
        self.hovered_container.is_none()
            && self.active_scroll_affordance.is_none()
            && self.auto_scroll_visible.is_empty()
    }

    fn is_current<Bridge, Message>(
        &self,
        runtime: &SurfaceRuntime<Bridge, Message>,
        candidate: &FreshSurfaceLayoutCandidate<Message>,
        appearance: ResolvedAppearance,
    ) -> bool
    where
        Bridge: RuntimeBridge<Message>,
    {
        self.environment == candidate.surface.resolved_environment()
            && candidate.surface.window_environment() == runtime.window_environment
            && self.appearance == appearance
            && self.hovered_container == runtime.interaction.hover.container
            && self.active_scroll_affordance == runtime.interaction.hover.scroll_affordance
            && self.auto_scroll_visible == runtime.scroll_auto_visibility()
    }
}

/// Typed inert veto for the candidate-only paint callback boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime::controller) enum FreshSurfacePaintVeto {
    Panicked,
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

/// Candidate-owned fresh-surface synchronization and prepared layout.
///
/// This type intentionally does not implement `Clone` and has no publication
/// operation. Its surface, mounted state, prepared output, and cleanup storage
/// are all released by dropping this value exactly once.
pub(in crate::runtime::controller) struct FreshSurfaceLayoutCandidate<Message> {
    surface: UiSurface<Message>,
    layout_root: LayoutNode,
    traversal: ProjectedTraversalIndex<Message>,
    source: SourceTraversalIndex,
    view_delta: ViewDelta,
    execution: RefreshExecutionDecision,
    replacement_plan: WidgetReplacementPlan,
    view_delta_scratch: ViewDeltaScratch,
    mounted_state: RuntimeLayoutContainerStateCandidate,
    layout_state: crate::gui::layout_core::LayoutState,
    scroll_settlements: Vec<(NodeId, crate::gui::types::Vector2)>,
    prepared_layout: PreparedLayoutPass,
    damage: SurfaceDamage,
    widget_state_sync: Duration,
    authority: FreshSurfaceLayoutAuthority,
}

/// Candidate-owned base-paint projection for one consumed fresh layout.
///
/// This type intentionally does not implement `Clone`.  Consuming the layout
/// candidate keeps its prepared workspace, synchronized successor surface,
/// inherited damage, and paint plan under one drop boundary until a later
/// private consumer exists.
pub(crate) struct FreshSurfacePaintCandidate<Message> {
    layout_candidate: FreshSurfaceLayoutCandidate<Message>,
    paint_plan: SurfacePaintPlan,
    projection_context: FreshSurfacePaintProjectionContext,
}

/// One complete, single-consumption prepared surface refresh transaction.
///
/// This is the private hand-off between the controller's candidate chain and
/// the native refresh consumer. It owns every value derived from the same
/// surface projection, including the inert replacement plan, candidate-local
/// state synchronization, mounted state, prepared layout, complete damage, and
/// the context-fenced backend-neutral paint plan.
pub(crate) enum PreparedSurfaceRefresh<Message> {
    Full {
        paint_candidate: Box<FreshSurfacePaintCandidate<Message>>,
        appearance: Box<ResolvedAppearance>,
        timings: SurfaceRefreshTimings,
        requested_scope: RepaintScope,
        application_environment: Option<crate::application::ApplicationEnvironment>,
    },
    Interaction {
        candidate: Box<InteractionPatchCandidate<Message>>,
        application_environment: Option<crate::application::ApplicationEnvironment>,
    },
}

/// The only result that crosses the private runtime publication boundary.
///
/// Terminal messages stay owned by the refresh transaction until the
/// successor has been published and the candidate paint plan has been
/// installed by the native caller.
pub(crate) struct PreparedSurfaceRefreshPublication<Message> {
    paint_plan: Option<SurfacePaintPlan>,
    appearance: ResolvedAppearance,
    terminal_messages: Vec<Message>,
    retired_candidate: Option<ExactChangedRoots<Message>>,
}

impl<Message> PreparedSurfaceRefreshPublication<Message> {
    pub(crate) fn into_parts(
        self,
    ) -> (
        Option<SurfacePaintPlan>,
        ResolvedAppearance,
        Vec<Message>,
        Option<ExactChangedRoots<Message>>,
    ) {
        (
            self.paint_plan,
            self.appearance,
            self.terminal_messages,
            self.retired_candidate,
        )
    }
}

impl<Message> PreparedSurfaceRefresh<Message> {
    pub(crate) fn appearance(&self) -> ResolvedAppearance {
        match self {
            Self::Full { appearance, .. } => **appearance,
            Self::Interaction { candidate, .. } => candidate.appearance,
        }
    }

    fn is_current<Bridge>(&self, runtime: &mut SurfaceRuntime<Bridge, Message>) -> bool
    where
        Bridge: RuntimeBridge<Message>,
    {
        match self {
            Self::Full {
                paint_candidate,
                appearance,
                application_environment,
                ..
            } => {
                application_environment == &runtime.sample_application_environment()
                    && paint_candidate.is_current(runtime, **appearance)
            }
            Self::Interaction {
                candidate,
                application_environment,
            } => {
                application_environment == &runtime.sample_application_environment()
                    && runtime.interaction_patch_candidate_is_current(candidate)
            }
        }
    }

    pub(crate) fn discard(self) {}
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Prepare the complete private transaction consumed by the native refresh
    /// path. All callbacks and layout work remain candidate-local until the
    /// transaction is later consumed by the controller publication method.
    pub(crate) fn prepare_fresh_surface_refresh(
        &mut self,
        scope: RepaintScope,
        appearance: ResolvedAppearance,
    ) -> Option<PreparedSurfaceRefresh<Message>> {
        let application_environment = self.sample_application_environment();
        let scope = application_environment
            .as_ref()
            .and_then(|environment| {
                environment.repaint_scope_since(self.surface.application_environment())
            })
            .map_or(scope, |application_scope| scope.merge(application_scope));
        let request = self.issue_fresh_surface_refresh_request(scope)?;
        let application_started = Instant::now();
        let update_request = SurfaceRefreshRequest {
            runtime_identity: self.runtime_identity(),
            request_revision: request.request_revision,
            active_surface_generation: request.active_surface_generation,
            viewport: request.viewport,
            window_environment: request.window_environment,
            expected_provider_authority: self.bridge.surface_update_provider_authority(),
        };
        let mut surface = match self.bridge.pull_surface_update(update_request) {
            SurfaceUpdate::Full(surface) => surface,
            SurfaceUpdate::ExactChangedRoots(candidate) => {
                let application_projection = application_started.elapsed();
                if self.sampled_application_environment_is_current(application_environment.as_ref())
                {
                    match self.prepare_interaction_update(
                        request,
                        update_request.expected_provider_authority,
                        candidate,
                        appearance,
                        application_projection,
                    ) {
                        InteractionPatchPreparation::Candidate(candidate) => {
                            return Some(PreparedSurfaceRefresh::Interaction {
                                candidate,
                                application_environment,
                            });
                        }
                        InteractionPatchPreparation::Full(candidate) => candidate.surface,
                        InteractionPatchPreparation::Terminal => return None,
                    }
                } else {
                    candidate.surface
                }
            }
        };
        if let Some(environment) = application_environment.clone() {
            surface = surface.with_application_environment(environment);
        }
        surface.set_window_environment(self.window_environment);
        let request = if application_environment.is_none() {
            if let Some(application_scope) = surface
                .application_environment()
                .repaint_scope_since(self.surface.application_environment())
            {
                let promoted_scope = request.scope.merge(application_scope);
                let promoted = FreshSurfaceRefreshRequest {
                    scope: promoted_scope,
                    ..request
                };
                self.fresh_surface_request = Some(promoted);
                promoted
            } else {
                request
            }
        } else {
            request
        };
        let application_projection = application_started.elapsed();

        let projection_started = Instant::now();
        let candidate = self.prepare_fresh_surface(surface, request)?;
        let runtime_projection = projection_started.elapsed();

        let layout_started = Instant::now();
        let layout_candidate = match self.prepare_fresh_surface_layout(candidate) {
            Ok(Some(candidate)) => candidate,
            Ok(None) | Err(_) => return None,
        };
        let layout = layout_started
            .elapsed()
            .saturating_sub(layout_candidate.widget_state_sync);

        let paint_candidate = match self.prepare_fresh_surface_paint(layout_candidate, appearance) {
            Ok(Some(candidate)) => candidate,
            Ok(None) | Err(_) => return None,
        };

        Some(PreparedSurfaceRefresh::Full {
            requested_scope: request.scope,
            appearance: Box::new(appearance),
            timings: SurfaceRefreshTimings {
                application_projection,
                runtime_projection,
                widget_state_sync: paint_candidate.layout_candidate.widget_state_sync,
                layout,
            },
            application_environment,
            paint_candidate: Box::new(paint_candidate),
        })
    }

    /// Consume one fully prepared transaction at the only irreversible runtime
    /// boundary. Every currentness and replacement-plan check occurs before
    /// the first retiring-widget callback. Once callbacks begin, failures are
    /// terminal rather than recoverable, preserving the direct refresh order.
    pub(crate) fn publish_prepared_surface_refresh(
        &mut self,
        prepared: PreparedSurfaceRefresh<Message>,
    ) -> Option<PreparedSurfaceRefreshPublication<Message>> {
        if !prepared.is_current(self) {
            prepared.discard();
            return None;
        }

        let PreparedSurfaceRefresh::Interaction {
            candidate: prepared,
            ..
        } = prepared
        else {
            let PreparedSurfaceRefresh::Full {
                paint_candidate,
                appearance,
                timings,
                requested_scope,
                application_environment: _,
            } = prepared
            else {
                unreachable!("prepared refresh variant already matched")
            };
            return self.publish_full_surface_refresh(
                *paint_candidate,
                *appearance,
                timings,
                requested_scope,
            );
        };

        let requested_scope = prepared.request.scope;
        let appearance = prepared.appearance;
        let application_projection = prepared.application_projection;
        let commit = self.publish_interaction_update(*prepared)?;
        let changed_count = commit.changed_count;
        let terminal_messages = commit.terminal_messages;
        self.refresh_counters.application_projection = self
            .refresh_counters
            .application_projection
            .saturating_add(1);
        self.base_paint_plan_reuse_eligible = true;
        let view_delta = ViewDeltaDiagnostics {
            classified: true,
            effect: ViewDeltaEffect::Interaction,
            total_events: changed_count,
            recorded_events: changed_count.min(16) as u8,
            base_paint_reuse_safe: true,
            damage: SurfaceDamage::empty(self.viewport),
            ..ViewDeltaDiagnostics::default()
        };
        self.record_refresh_diagnostics(
            crate::runtime::SurfaceRefreshDiagnostics {
                invalidation: crate::runtime::SurfaceInvalidation::from_repaint_scope(Some(
                    requested_scope,
                )),
                timings: SurfaceRefreshTimings {
                    application_projection,
                    ..SurfaceRefreshTimings::default()
                },
                identity: SurfaceIdentityDiagnostics::default(),
                layout_state: self.last_layout_state_diagnostics,
            },
            Duration::ZERO,
            view_delta,
            requested_scope,
        );
        Some(PreparedSurfaceRefreshPublication {
            paint_plan: None,
            appearance,
            terminal_messages,
            retired_candidate: Some(commit.retired_candidate),
        })
    }

    fn publish_full_surface_refresh(
        &mut self,
        paint_candidate: FreshSurfacePaintCandidate<Message>,
        appearance: ResolvedAppearance,
        timings: SurfaceRefreshTimings,
        requested_scope: RepaintScope,
    ) -> Option<PreparedSurfaceRefreshPublication<Message>> {
        let FreshSurfacePaintCandidate {
            layout_candidate,
            paint_plan,
            projection_context: _,
        } = paint_candidate;
        let FreshSurfaceLayoutCandidate {
            surface,
            layout_root,
            traversal,
            source,
            view_delta,
            execution,
            replacement_plan,
            view_delta_scratch,
            mounted_state,
            layout_state,
            scroll_settlements,
            prepared_layout,
            damage,
            widget_state_sync: _,
            authority,
        } = layout_candidate;

        let previous_widget_order = self.traversal.widgets.hit_order.clone();
        let previous_stateful_widget_order = self.traversal.widgets.stateful_order.clone();
        let previous_paths = self.traversal.widgets.paths.current.clone();
        let validated_replacement_plan = self
            .surface
            .validate_widget_replacement_plan(
                &surface,
                replacement_plan,
                &previous_widget_order,
                &traversal.widget_paint_order,
                &previous_paths,
                &traversal.widget_paths,
            )
            .ok()?;
        let authority_preflight =
            self.preflight_fresh_surface_refresh_authority(authority.preparation.request)?;
        let _authority_commit = self.commit_fresh_surface_refresh_authority(authority_preflight)?;

        // The authority marker and validated plan establish the irreversible
        // boundary. The remaining replacement sequence has no recoverable
        // veto and preserves the established direct publication order.
        let replacement_commit = self
            .surface
            .commit_validated_widget_replacements(&surface, validated_replacement_plan);
        let mut terminal_messages = replacement_commit.terminal_messages;
        let retired_widget_ids = replacement_commit.retired_widget_ids;
        let wheel_focus_before_refresh = self.interaction.focus.focused_widget();
        let composition_focus_before_refresh = self.interaction.focus.focused_widget();
        // Preserve the old surface mapper long enough to deliver exactly one
        // terminal cancellation for incompatible ingress owners.
        self.reconcile_pointer_ingress_before_surface_replace(
            &surface,
            &previous_paths,
            &traversal.widget_paths,
            &retired_widget_ids,
            &mut terminal_messages,
        );
        let identity = self.discard_incompatible_widget_ownership(
            &surface,
            &traversal.widget_paint_order,
            &traversal.widget_paths,
            &previous_paths,
        );
        for widget_id in &retired_widget_ids {
            self.discard_widget_ownership(*widget_id);
        }

        self.reconcile_focused_key_capture_after_refresh(
            &surface,
            &previous_widget_order,
            &traversal.widget_paint_order,
            &previous_stateful_widget_order,
            &traversal.stateful_widget_order,
            &previous_paths,
            &traversal.widget_paths,
            &retired_widget_ids,
        );
        self.reconcile_managed_wheel_sequence_after_refresh(
            &surface,
            &previous_widget_order,
            &traversal.widget_paint_order,
            &previous_paths,
            &traversal.widget_paths,
            &retired_widget_ids,
            wheel_focus_before_refresh,
        );
        self.reconcile_managed_composition_after_refresh(
            &surface,
            &previous_widget_order,
            &traversal.widget_paint_order,
            &previous_paths,
            &traversal.widget_paths,
            &retired_widget_ids,
            composition_focus_before_refresh,
        );
        self.reconcile_managed_pointer_capture_after_refresh(
            &surface,
            &previous_widget_order,
            &traversal.widget_paint_order,
            &previous_paths,
            &traversal.widget_paths,
            &retired_widget_ids,
        );

        self.surface = surface;
        self.layout_state = layout_state;
        if prepared_layout
            .commit(&mut self.layout_engine, &mut self.layout, authority.input)
            .is_err()
        {
            std::process::abort();
        }
        self.replace_layout_root(layout_root);
        self.install_traversal_with_candidate(traversal, mounted_state);
        self.traversal.widgets.paths.previous = previous_paths;
        self.scratch.projection_source = source;
        self.scratch.view_delta = view_delta_scratch;
        self.sync_scroll_offsets();
        self.record_completed_layout();

        if self.interaction.pointer.managed_capture.is_some() {
            self.interaction.pointer.capture_state = None;
        }
        self.restore_pointer_capture_state();
        self.validate_managed_pointer_capture_authority();
        self.validate_managed_wheel_sequence_authority();
        self.validate_managed_composition_authority();
        if let Some(capture) = self.interaction.pointer.managed_capture
            && capture.state == RuntimeManagedPointerCaptureState::Active
        {
            self.capture_pointer_capture_state(capture.widget_id);
        }
        self.clear_stale_interaction_state();
        self.reconcile_pointer_ingress_sequences();
        if let Some(widget_id) = self.interaction.focus.focused_widget() {
            self.restore_focused_widget_state(widget_id);
        }
        self.validate_focused_key_capture_authority();
        self.install_declarative_owner_projection();
        self.interaction
            .wheel
            .pending_scroll_settlement
            .retain(|(id, _)| self.traversal.containers.scroll.contains(*id));
        if self.interaction.wheel.pending_scroll_settlement.is_empty() {
            self.interaction.wheel.scroll_settlement_deadline = None;
        }
        for (node_id, previous_offset) in scroll_settlements {
            let offset = self.layout_state.scroll_offset(node_id);
            if offset != previous_offset
                && let Some(message) = self.surface.root().offset_settled(node_id, offset)
            {
                terminal_messages.push(message);
            }
        }

        self.refresh_counters.application_projection = self
            .refresh_counters
            .application_projection
            .saturating_add(1);
        self.refresh_counters.runtime_projection =
            self.refresh_counters.runtime_projection.saturating_add(1);
        self.refresh_counters.widget_state_sync =
            self.refresh_counters.widget_state_sync.saturating_add(1);
        self.refresh_counters.layout = self.refresh_counters.layout.saturating_add(1);
        self.base_paint_plan_reuse_eligible = false;

        let mut view_delta_diagnostics = view_delta.diagnostics(timings.total());
        view_delta_diagnostics.damage = damage;
        self.record_refresh_diagnostics(
            crate::runtime::SurfaceRefreshDiagnostics {
                invalidation: crate::runtime::SurfaceInvalidation::from_repaint_scope(Some(
                    requested_scope,
                )),
                timings,
                identity,
                layout_state: self.last_layout_state_diagnostics,
            },
            timings.total(),
            view_delta_diagnostics,
            execution.effective_scope(),
        );
        self.enforce_identity_audit(identity);

        Some(PreparedSurfaceRefreshPublication {
            paint_plan: Some(paint_plan),
            appearance,
            terminal_messages,
            retired_candidate: None,
        })
    }

    pub(crate) fn finish_prepared_surface_refresh(&mut self, terminal_messages: Vec<Message>) {
        self.dispatch_deferred_surface_messages(terminal_messages);
        self.service_pending_current_surface_relayout();
    }

    fn preflight_fresh_surface_refresh_authority(
        &self,
        request: FreshSurfaceRefreshRequest,
    ) -> Option<FreshSurfaceRefreshAuthorityPreflight> {
        if self.fresh_surface_authority_exhausted {
            return None;
        }
        let stored_request = self.fresh_surface_request?;
        if !stored_request.exactly_matches(request)
            || !self.fresh_surface_request_is_current(request)
        {
            return None;
        }
        let next_active_surface_generation =
            next_checked_generation(self.fresh_surface_active_generation)?;
        Some(FreshSurfaceRefreshAuthorityPreflight {
            request,
            next_active_surface_generation,
        })
    }

    fn commit_fresh_surface_refresh_authority(
        &mut self,
        preflight: FreshSurfaceRefreshAuthorityPreflight,
    ) -> Option<FreshSurfaceRefreshAuthorityCommit> {
        if self.fresh_surface_authority_exhausted
            || self.fresh_surface_active_generation != preflight.request.active_surface_generation
            || next_checked_generation(self.fresh_surface_active_generation)
                != Some(preflight.next_active_surface_generation)
        {
            return None;
        }

        let stored_request = self.fresh_surface_request.take()?;
        if !stored_request.exactly_matches(preflight.request) {
            self.fresh_surface_request = Some(stored_request);
            return None;
        }

        self.fresh_surface_active_generation = preflight.next_active_surface_generation;
        Some(FreshSurfaceRefreshAuthorityCommit {
            request_revision: stored_request.request_revision,
            active_surface_generation: self.fresh_surface_active_generation,
        })
    }

    pub(in crate::runtime::controller) fn consume_fresh_surface_refresh_authority(
        &mut self,
        request: FreshSurfaceRefreshRequest,
    ) -> bool {
        let Some(preflight) = self.preflight_fresh_surface_refresh_authority(request) else {
            return false;
        };
        self.commit_fresh_surface_refresh_authority(preflight)
            .is_some()
    }

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

        if next_checked_generation(self.fresh_surface_active_generation).is_none() {
            self.fresh_surface_authority_exhausted = true;
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

    /// Return whether the active runtime state is supported by prepared
    /// refresh. Virtualized content requires the combined refresh path to
    /// perform its materialization pass before any prepared admission.
    pub(crate) fn prepared_surface_refresh_is_eligible(&self) -> bool {
        self.virtual_layout.is_empty()
            && self
                .traversal
                .containers
                .virtual_layout_registrations
                .is_empty()
    }

    pub(crate) const fn fresh_surface_active_generation(&self) -> u64 {
        self.fresh_surface_active_generation
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

    /// Prepare one private candidate-only synchronization and layout pass.
    ///
    /// The initial prepared pass is an owned preflight witness for the exact
    /// engine/cache/dirty/generation state observed before callbacks. It is
    /// discarded before the final pass is prepared against the synchronized
    /// successor root. No direct refresh, relayout, replacement, output, or
    /// publication path is entered here.
    pub(in crate::runtime::controller) fn prepare_fresh_surface_layout(
        &mut self,
        mut candidate: FreshSurfacePreparationCandidate<Message>,
    ) -> Result<Option<FreshSurfaceLayoutCandidate<Message>>, PreparedWidgetStateSyncVeto> {
        if !candidate.is_current(self)
            || candidate.layout_root.id() != self.layout_root.id()
            || self.layout_authority_exhausted
        {
            return Ok(None);
        }
        let mut candidate_root_authority = self.layout_root_authority;
        if !candidate_root_authority.advance_revision()
            || candidate_root_authority == self.layout_root_authority
        {
            return Ok(None);
        }

        let mounted_state = self.prepare_layout_container_state_candidate(&candidate.traversal);
        if !mounted_state.is_admissible() {
            return Ok(None);
        }
        let candidate_mounted_source_present = mounted_state.source_present();
        let candidate_direction = candidate.surface.resolved_environment().writing_direction();
        let Some(input) = self.runtime_layout_input_evidence_for_root_with_direction(
            candidate_root_authority,
            candidate_mounted_source_present,
            candidate_direction,
        ) else {
            return Ok(None);
        };

        let mut candidate_layout_state = self.layout_state.clone();
        let mut declarations = Vec::new();
        collect_scroll_declarations(&candidate.layout_root, &mut declarations);
        sync_declarative_scroll_inputs_for(
            declarations,
            &mut candidate_layout_state,
            self.layout_state_generation,
        );

        let preflight_layout = if candidate_mounted_source_present {
            let container_state_source = self.interaction.layout_state.read_source(&mounted_state);
            self.layout_engine.prepare_layout_with_direction_and_source(
                &candidate.layout_root,
                self.viewport,
                &candidate_layout_state,
                self.layout_debug_options,
                candidate_direction,
                Some(&container_state_source),
                input,
            )
        } else {
            self.layout_engine.prepare_layout_with_direction_and_source(
                &candidate.layout_root,
                self.viewport,
                &candidate_layout_state,
                self.layout_debug_options,
                candidate_direction,
                None,
                input,
            )
        };
        if !preflight_layout.is_usable()
            || preflight_layout
                .validate_for_engine(&self.layout_engine, input)
                .is_err()
        {
            preflight_layout.discard();
            return Ok(None);
        }

        let sync_evidence = PreparedWidgetStateSyncEvidence {
            stateful_widget_order: &candidate.traversal.stateful_widget_order,
            current_paths: &candidate.traversal.widget_paths,
            previous_paths: &self.traversal.widgets.paths.current,
            previous_widget_order: &self.traversal.widgets.hit_order,
            current_widget_order: &candidate.traversal.widget_paint_order,
            policy: self.widget_state_sync_policy(),
        };
        let widget_state_sync_started = Instant::now();
        let sync_witness = candidate
            .surface
            .preflight_prepared_widget_state_sync(&self.surface, sync_evidence)?;
        candidate.surface.synchronize_prepared_widget_state(
            &self.surface,
            sync_evidence,
            &sync_witness,
        )?;
        candidate
            .surface
            .prepared_widget_state_sync_is_current(&self.surface, &sync_witness)?;
        let widget_state_sync = widget_state_sync_started.elapsed();

        let SurfaceRuntimeProjection {
            layout_root,
            traversal,
            source,
        } = candidate.surface.runtime_projection();
        if layout_root.id() != candidate.layout_root.id()
            || !traversal_matches_runtime(&traversal, &self.traversal)
            || !source_indices_match(&source, &self.scratch.projection_source)
            || !traversal.virtual_layout_registrations.is_empty()
        {
            preflight_layout.discard();
            return Ok(None);
        }
        candidate.layout_root = layout_root;
        candidate.traversal = traversal;
        candidate.source = source;

        let mut declarations = Vec::new();
        collect_scroll_declarations(&candidate.layout_root, &mut declarations);
        let live: std::collections::BTreeSet<_> =
            declarations.iter().map(|(id, _, _)| *id).collect();
        candidate_layout_state
            .scroll_runtime
            .retain(|id, _| live.contains(id));
        candidate_layout_state
            .scroll_offsets
            .retain(|id, _| live.contains(id));
        sync_declarative_scroll_inputs_for(
            declarations,
            &mut candidate_layout_state,
            self.layout_state_generation,
        );

        if !candidate.is_current(self)
            || !preflight_layout.is_current_for_engine(&self.layout_engine)
        {
            preflight_layout.discard();
            return Ok(None);
        }
        preflight_layout.discard();

        let mut prepared_layout = if candidate_mounted_source_present {
            let container_state_source = self.interaction.layout_state.read_source(&mounted_state);
            self.layout_engine.prepare_layout_with_direction_and_source(
                &candidate.layout_root,
                self.viewport,
                &candidate_layout_state,
                self.layout_debug_options,
                candidate.surface.resolved_environment().writing_direction(),
                Some(&container_state_source),
                input,
            )
        } else {
            self.layout_engine.prepare_layout_with_direction_and_source(
                &candidate.layout_root,
                self.viewport,
                &candidate_layout_state,
                self.layout_debug_options,
                candidate.surface.resolved_environment().writing_direction(),
                None,
                input,
            )
        };
        if !prepared_layout.is_usable()
            || prepared_layout
                .validate_for_engine(&self.layout_engine, input)
                .is_err()
        {
            prepared_layout.discard();
            return Ok(None);
        }
        let mut scroll_settlements = Vec::new();
        if let Some(output) = prepared_layout.output().cloned() {
            // Normalize controlled offsets against the first candidate output
            // before resolving requests.  Request alignment must observe the
            // committed/clamped current offset, just like the direct route;
            // otherwise an out-of-range controlled value can make a no-op
            // reveal appear to have moved and emit a false settlement.
            sync_scroll_offsets_for(&output, &candidate.traversal, &mut candidate_layout_state);
            scroll_settlements = apply_declarative_scroll_requests_for(
                {
                    let mut requests = Vec::new();
                    collect_scroll_requests(&candidate.layout_root, &mut requests);
                    requests
                },
                &output,
                &mut candidate_layout_state,
                |owner, key| self.virtual_layout.materialized_key_payload(owner, key),
            );
            if !scroll_settlements.is_empty() {
                prepared_layout.discard();
                prepared_layout = if candidate_mounted_source_present {
                    let container_state_source =
                        self.interaction.layout_state.read_source(&mounted_state);
                    self.layout_engine.prepare_layout_with_state_and_source(
                        &candidate.layout_root,
                        self.viewport,
                        &candidate_layout_state,
                        self.layout_debug_options,
                        Some(&container_state_source),
                        input,
                    )
                } else {
                    self.layout_engine.prepare_layout_with_state_and_source(
                        &candidate.layout_root,
                        self.viewport,
                        &candidate_layout_state,
                        self.layout_debug_options,
                        None,
                        input,
                    )
                };
                if !prepared_layout.is_usable()
                    || prepared_layout
                        .validate_for_engine(&self.layout_engine, input)
                        .is_err()
                {
                    prepared_layout.discard();
                    return Ok(None);
                }
            }
        }
        if let Some(output) = prepared_layout.output() {
            sync_scroll_offsets_for(output, &candidate.traversal, &mut candidate_layout_state);
        }
        let Some(candidate_layout_output) = prepared_layout.output() else {
            prepared_layout.discard();
            return Ok(None);
        };
        let damage = SurfaceDamage::from_view_delta(
            &candidate.view_delta,
            &candidate.view_delta.reconciliation_plan(),
            &self.surface,
            &self.layout,
            self.viewport,
        )
        .finish(&candidate.surface, candidate_layout_output);

        let authority = FreshSurfaceLayoutAuthority {
            preparation: candidate.authority,
            candidate_root_authority,
            candidate_root_id: candidate.layout_root.id(),
            candidate_root_state_version: candidate.layout_root.state_version(),
            input,
            candidate_mounted_source_present,
        };
        let FreshSurfacePreparationCandidate {
            surface,
            layout_root,
            traversal,
            source,
            view_delta,
            execution,
            replacement_plan,
            view_delta_scratch,
            authority: _,
        } = candidate;
        Ok(Some(FreshSurfaceLayoutCandidate {
            surface,
            layout_root,
            traversal,
            source,
            view_delta,
            execution,
            replacement_plan,
            view_delta_scratch,
            mounted_state,
            layout_state: candidate_layout_state,
            scroll_settlements,
            prepared_layout,
            damage,
            widget_state_sync,
            authority,
        }))
    }

    /// Consume one current fresh layout candidate into an inert base-paint
    /// projection candidate.
    ///
    /// This helper is deliberately not called by production refresh, frame,
    /// native, or overlay paths. It only owns candidate storage: all runtime
    /// interaction and overlay context must be neutral before the first paint
    /// callback, and the layout candidate is revalidated again afterward.
    pub(in crate::runtime::controller) fn prepare_fresh_surface_paint(
        &self,
        candidate: FreshSurfaceLayoutCandidate<Message>,
        appearance: ResolvedAppearance,
    ) -> Result<Option<FreshSurfacePaintCandidate<Message>>, FreshSurfacePaintVeto> {
        if !candidate.is_current(self)
            || !candidate.prepared_layout.is_usable()
            || candidate.layout_output().is_none()
        {
            return Ok(None);
        }

        let projection_context =
            FreshSurfacePaintProjectionContext::capture(self, &candidate, appearance);
        if !projection_context.is_neutral() || !self.fresh_surface_paint_context_is_neutral() {
            return Ok(None);
        }

        if !candidate.is_current(self)
            || !candidate.prepared_layout.is_usable()
            || candidate.layout_output().is_none()
        {
            return Ok(None);
        }
        let Some(layout) = candidate.layout_output() else {
            return Ok(None);
        };
        let mut paint_plan = empty_paint_plan_for_layout(layout, &projection_context.theme);
        let traversal = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            candidate
                .surface
                .paint_plan_with_hover_and_environment_and_appearance_and_scroll_visibility_into(
                    layout,
                    &projection_context.theme,
                    projection_context.environment.clone(),
                    projection_context.appearance,
                    projection_context.hovered_container,
                    projection_context.active_scroll_affordance,
                    &projection_context.auto_scroll_visible,
                    &mut paint_plan,
                );
        }));
        if traversal.is_err() {
            drop(paint_plan);
            return Err(FreshSurfacePaintVeto::Panicked);
        }

        if !candidate.is_current(self)
            || !candidate.prepared_layout.is_usable()
            || candidate.layout_output().is_none()
            || !self.fresh_surface_paint_context_is_neutral()
            || !projection_context.is_current(self, &candidate, appearance)
        {
            return Ok(None);
        }

        Ok(Some(FreshSurfacePaintCandidate {
            layout_candidate: candidate,
            paint_plan,
            projection_context,
        }))
    }

    fn fresh_surface_paint_context_is_neutral(&self) -> bool {
        self.interaction.hover.widget.is_none()
            && self.interaction.hover.scroll_viewport.is_none()
            && self.interaction.wheel.scroll_activity.is_empty()
            && self.interaction.pointer.capture.is_none()
            && self.interaction.pointer.capture_state.is_none()
            && self.interaction.pointer.managed_capture.is_none()
            && self.interaction.pointer.scroll_drag_capture.is_none()
            && self.interaction.layout_capture.is_none()
            && self
                .interaction
                .drag
                .session
                .as_ref()
                .is_none_or(|session| !session.visible)
            && self.interaction.tooltip.target.is_none()
            && self.interaction.tooltip.deadline.is_none()
            && !self.interaction.tooltip.revealed
            && !self.devtools_overlay.enabled
    }

    /// Test-only stale-generation fixture.
    ///
    /// The checked fail-closed behavior remains so preparation tests can model
    /// a newer active-surface generation and verify stale candidates are
    /// rejected.  Production refresh paths must not call this helper.
    #[cfg(test)]
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
        fresh_surface_preparation_is_current(
            runtime,
            &self.surface,
            &self.layout_root,
            &self.traversal,
            &self.source,
            self.authority,
        )
    }

    /// Explicitly abandon the candidate.  Dropping the candidate is equally
    /// inert; this spelling makes the discard boundary testable without a
    /// consumer or commit operation.
    pub(in crate::runtime::controller) fn discard(self) {}
}

impl<Message> FreshSurfaceLayoutCandidate<Message> {
    /// Borrow the candidate-owned prepared layout output without exposing a
    /// mutable or active runtime layout value.
    pub(in crate::runtime::controller) fn layout_output(&self) -> Option<&LayoutOutput> {
        self.prepared_layout.output()
    }

    /// Return whether all captured fresh, candidate-root, mounted, and engine
    /// evidence remains current. This method is observational and never
    /// installs candidate state into the active runtime.
    pub(in crate::runtime::controller) fn is_current<Bridge>(
        &self,
        runtime: &SurfaceRuntime<Bridge, Message>,
    ) -> bool
    where
        Bridge: RuntimeBridge<Message>,
    {
        let authority = self.authority;
        authority.candidate_root_authority.is_valid()
            && authority.candidate_root_authority != runtime.layout_root_authority
            && authority.candidate_root_id == self.layout_root.id()
            && authority.candidate_root_state_version == self.layout_root.state_version()
            && authority.candidate_mounted_source_present == self.mounted_state.source_present()
            && self.mounted_state.is_admissible()
            && fresh_surface_preparation_is_current(
                runtime,
                &self.surface,
                &self.layout_root,
                &self.traversal,
                &self.source,
                authority.preparation,
            )
            && runtime
                .runtime_layout_input_evidence_for_root_with_direction(
                    authority.candidate_root_authority,
                    authority.candidate_mounted_source_present,
                    self.surface.resolved_environment().writing_direction(),
                )
                .is_some_and(|input| input == authority.input)
            && self
                .prepared_layout
                .validate_for_engine(&runtime.layout_engine, authority.input)
                .is_ok()
    }

    /// Explicitly abandon this candidate. Dropping it releases the prepared
    /// workspace, mounted candidate values, surface, and cleanup storage once.
    pub(in crate::runtime::controller) fn discard(self) {}
}

impl<Message> FreshSurfacePaintCandidate<Message> {
    /// Borrow the one candidate-owned backend-neutral base-paint plan.
    pub(in crate::runtime::controller) fn paint_plan(&self) -> &SurfacePaintPlan {
        &self.paint_plan
    }

    /// Borrow the consumed layout candidate, including its inherited damage.
    pub(in crate::runtime::controller) fn layout_candidate(
        &self,
    ) -> &FreshSurfaceLayoutCandidate<Message> {
        &self.layout_candidate
    }

    /// Return whether the consumed layout candidate remains current for a
    /// later private consumer.
    pub(in crate::runtime::controller) fn is_current<Bridge>(
        &self,
        runtime: &SurfaceRuntime<Bridge, Message>,
        appearance: ResolvedAppearance,
    ) -> bool
    where
        Bridge: RuntimeBridge<Message>,
    {
        self.layout_candidate.is_current(runtime)
            && runtime.fresh_surface_paint_context_is_neutral()
            && self
                .projection_context
                .is_current(runtime, &self.layout_candidate, appearance)
    }

    /// Explicitly abandon this candidate and all candidate-owned paint/layout
    /// state without touching the active runtime.
    pub(in crate::runtime::controller) fn discard(self) {}
}

fn fresh_surface_preparation_is_current<Bridge, Message>(
    runtime: &SurfaceRuntime<Bridge, Message>,
    surface: &UiSurface<Message>,
    layout_root: &LayoutNode,
    traversal: &ProjectedTraversalIndex<Message>,
    source: &SourceTraversalIndex,
    authority: FreshSurfacePreparationAuthority,
) -> bool
where
    Bridge: RuntimeBridge<Message>,
{
    !(runtime.fresh_surface_authority_exhausted
        || !valid_checked_generation(authority.request.request_revision)
        || !valid_checked_generation(authority.request.active_surface_generation)
        || !valid_checked_generation(authority.request.lifecycle_transition_sequence)
        || !runtime.fresh_surface_request_is_current(authority.request)
        || authority.request.runtime_identity != runtime.runtime_identity()
        || authority.request.lifecycle_phase != RuntimeLifecyclePhase::Running
        || authority.request.lifecycle_phase != runtime.lifecycle_phase()
        || authority.request.lifecycle_transition_sequence
            != runtime.lifecycle_transition_sequence()
        || authority.request.active_surface_generation != runtime.fresh_surface_active_generation
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
        || surface.window_environment() != authority.request.window_environment
        || layout_root != &surface.layout_node()
        || !traversal.virtual_layout_registrations.is_empty()
        || !source_indices_match(source, &runtime.scratch.projection_source)
        || !traversal_matches_runtime(traversal, &runtime.traversal))
}

fn valid_checked_generation(value: u64) -> bool {
    value != 0 && value != u64::MAX
}

fn next_checked_generation(value: u64) -> Option<u64> {
    value
        .checked_add(1)
        .filter(|next_generation| valid_checked_generation(*next_generation))
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
        application_environment: Option<crate::application::ApplicationEnvironment>,
    }

    type Fixture = (SurfaceRuntime<SpyBridge, ()>, CallCount, CallCount);

    impl RuntimeBridge<()> for SpyBridge {
        fn application_environment(
            &mut self,
        ) -> Option<crate::application::ApplicationEnvironment> {
            self.application_environment.clone()
        }

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
                application_environment: None,
            },
            Vector2::new(120.0, 80.0),
        );
        (runtime, pull_calls, project_calls)
    }

    fn runtime_for_surface_with_application_environment(
        surface: UiSurface<()>,
        application_environment: Option<crate::application::ApplicationEnvironment>,
    ) -> SurfaceRuntime<SpyBridge, ()> {
        SurfaceRuntime::new(
            SpyBridge {
                surface,
                pull_calls: Rc::new(Cell::new(0)),
                project_calls: Rc::new(Cell::new(0)),
                application_environment,
            },
            Vector2::new(120.0, 80.0),
        )
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

    #[test]
    fn startup_application_source_preserves_explicit_none_and_overrides_conflicting_some() {
        let explicit = crate::application::ApplicationEnvironment::new(
            crate::application::LocaleId::new("de").expect("valid test locale"),
        );
        let runtime = runtime_for_surface_with_application_environment(
            ordinary_surface(Vector2::new(24.0, 16.0))
                .with_application_environment(explicit.clone()),
            None,
        );
        assert_eq!(runtime.surface().application_environment(), &explicit);

        let source = crate::application::ApplicationEnvironment::new(
            crate::application::LocaleId::new("fr").expect("valid test locale"),
        )
        .with_text_scale(crate::application::TextScale::new(1.1).expect("valid test text scale"));
        let runtime = runtime_for_surface_with_application_environment(
            ordinary_surface(Vector2::new(24.0, 16.0)).with_application_environment(explicit),
            Some(source.clone()),
        );
        assert_eq!(runtime.surface().application_environment(), &source);
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

        fn supports_prepared_state_synchronization(&self) -> bool {
            true
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

    #[derive(Clone, Copy)]
    enum PreparedSyncBehavior {
        Qualified,
        Unsupported,
        Panic,
    }

    #[derive(Clone)]
    struct PreparedSyncWidget {
        common: WidgetCommon,
        local_state: u64,
        behavior: PreparedSyncBehavior,
        sync_calls: usize,
        drop_observation: Rc<Cell<usize>>,
    }

    impl PreparedSyncWidget {
        fn new(
            id: u64,
            initial_size: Vector2,
            local_state: u64,
            behavior: PreparedSyncBehavior,
            drop_observation: Rc<Cell<usize>>,
        ) -> Self {
            Self {
                common: WidgetCommon::fixed(id, initial_size.x, initial_size.y),
                local_state,
                behavior,
                sync_calls: 0,
                drop_observation,
            }
        }
    }

    impl Drop for PreparedSyncWidget {
        fn drop(&mut self) {
            self.drop_observation
                .set(self.drop_observation.get().saturating_add(self.sync_calls));
        }
    }

    impl Widget for PreparedSyncWidget {
        fn revision(&self) -> crate::widgets::WidgetRevision {
            crate::widgets::WidgetRevision::exact((), (), (), ())
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

        fn supports_prepared_state_synchronization(&self) -> bool {
            !matches!(self.behavior, PreparedSyncBehavior::Unsupported)
        }

        fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
            self.sync_calls = self.sync_calls.saturating_add(1);
            let previous = previous
                .as_any()
                .downcast_ref::<Self>()
                .expect("prepared fixture should synchronize like-for-like widgets");
            if matches!(self.behavior, PreparedSyncBehavior::Panic) {
                panic!("prepared synchronization fixture panic");
            }
            self.local_state = previous.local_state;
        }

        fn append_paint(
            &self,
            primitives: &mut Vec<crate::runtime::PaintPrimitive>,
            bounds: crate::gui::types::Rect,
            _layout: &LayoutOutput,
            theme: &crate::theme::ThemeTokens,
        ) {
            primitives.push(crate::runtime::PaintPrimitive::FillRect(
                crate::runtime::PaintFillRect {
                    widget_id: self.common.id,
                    rect: bounds,
                    color: if self.local_state == 41 {
                        theme.accent_mint
                    } else {
                        theme.accent_copper
                    },
                },
            ));
        }
    }

    fn prepared_sync_surface(widgets: Vec<PreparedSyncWidget>) -> UiSurface<()> {
        UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            widgets
                .into_iter()
                .map(|widget| SurfaceChild::fill(SurfaceNode::static_widget(widget)))
                .collect(),
        ))
    }

    #[derive(Clone, Copy)]
    enum PaintProbeBehavior {
        Emit,
        Panic,
    }

    #[derive(Clone)]
    struct PaintProbeWidget {
        common: WidgetCommon,
        behavior: PaintProbeBehavior,
        paint_calls: Rc<Cell<usize>>,
        drop_calls: Rc<Cell<usize>>,
    }

    impl PaintProbeWidget {
        fn new(
            id: u64,
            behavior: PaintProbeBehavior,
            paint_calls: Rc<Cell<usize>>,
            drop_calls: Rc<Cell<usize>>,
        ) -> Self {
            Self {
                common: WidgetCommon::fixed(id, 24.0, 16.0).without_default_chrome(),
                behavior,
                paint_calls,
                drop_calls,
            }
        }
    }

    impl Drop for PaintProbeWidget {
        fn drop(&mut self) {
            self.drop_calls.set(self.drop_calls.get().saturating_add(1));
        }
    }

    impl Widget for PaintProbeWidget {
        fn revision(&self) -> crate::widgets::WidgetRevision {
            crate::widgets::WidgetRevision::exact((), (), (), ())
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

        fn supports_prepared_state_synchronization(&self) -> bool {
            true
        }

        fn synchronize_from_previous(&mut self, _previous: &dyn Widget) {}

        fn append_paint(
            &self,
            primitives: &mut Vec<crate::runtime::PaintPrimitive>,
            bounds: crate::gui::types::Rect,
            _layout: &LayoutOutput,
            theme: &crate::theme::ThemeTokens,
        ) {
            self.paint_calls
                .set(self.paint_calls.get().saturating_add(1));
            primitives.push(crate::runtime::PaintPrimitive::FillRect(
                crate::runtime::PaintFillRect {
                    widget_id: self.common.id,
                    rect: bounds,
                    color: theme.accent_mint,
                },
            ));
            if matches!(self.behavior, PaintProbeBehavior::Panic) {
                panic!("fresh surface paint fixture panic");
            }
        }
    }

    fn paint_probe_surface(
        behaviors: &[PaintProbeBehavior],
        paint_calls: Rc<Cell<usize>>,
        drop_calls: Rc<Cell<usize>>,
    ) -> UiSurface<()> {
        UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            behaviors
                .iter()
                .copied()
                .enumerate()
                .map(|(index, behavior)| {
                    SurfaceChild::fill(SurfaceNode::static_widget(PaintProbeWidget::new(
                        index as u64 + 2,
                        behavior,
                        Rc::clone(&paint_calls),
                        Rc::clone(&drop_calls),
                    )))
                })
                .collect(),
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

    fn prepare_layout_candidate(
        runtime: &mut SurfaceRuntime<SpyBridge, ()>,
        candidate_surface: UiSurface<()>,
    ) -> FreshSurfaceLayoutCandidate<()> {
        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("ordinary candidate request");
        let preparation = runtime
            .prepare_fresh_surface(candidate_surface, request)
            .expect("ordinary candidate admission");
        runtime
            .prepare_fresh_surface_layout(preparation)
            .expect("candidate layout preparation should not veto")
            .expect("candidate layout should be present")
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
    fn qualified_state_sync_changes_only_candidate_layout_and_state() {
        let active_calls = Rc::new(Cell::new(0));
        let candidate_calls = Rc::new(Cell::new(0));
        let active = prepared_sync_surface(vec![PreparedSyncWidget::new(
            2,
            Vector2::new(24.0, 16.0),
            41,
            PreparedSyncBehavior::Qualified,
            Rc::clone(&active_calls),
        )]);
        let candidate_surface = prepared_sync_surface(vec![PreparedSyncWidget::new(
            2,
            Vector2::new(64.0, 28.0),
            0,
            PreparedSyncBehavior::Qualified,
            Rc::clone(&candidate_calls),
        )]);
        let (mut runtime, _, _) = runtime_for_surface(active);
        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("request");
        let before = active_snapshot(&runtime);
        let preparation = runtime
            .prepare_fresh_surface(candidate_surface, request)
            .expect("qualified ordinary candidate should prepare");
        let candidate = runtime
            .prepare_fresh_surface_layout(preparation)
            .expect("qualified synchronization should not veto")
            .expect("candidate layout should be prepared");

        assert!(candidate.is_current(&runtime));
        assert!(candidate.layout_output().is_some());
        assert_eq!(candidate.view_delta.effect, ViewDeltaEffect::Unchanged);
        assert_eq!(candidate.damage.candidate_count, 0);
        assert!(!candidate.damage.full_viewport);
        assert_ne!(candidate.layout_root, runtime.layout_root);
        assert_ne!(
            candidate.authority.candidate_root_authority,
            candidate.authority.preparation.layout_root_authority
        );
        let synchronized = candidate
            .surface
            .find_widget(2)
            .expect("candidate widget")
            .widget()
            .as_any()
            .downcast_ref::<PreparedSyncWidget>()
            .expect("prepared fixture widget");
        assert_eq!(synchronized.local_state, 41);
        assert_eq!(
            synchronized.common.sizing.preferred,
            Vector2::new(64.0, 28.0)
        );
        assert_eq!(synchronized.sync_calls, 1);
        assert_eq!(candidate_calls.get(), 0);
        assert_eq!(active_calls.get(), 0);
        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));

        candidate.discard();
        assert_eq!(candidate_calls.get(), 1);
        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
    }

    #[test]
    fn synchronized_successor_state_changes_candidate_paint_and_preserves_damage() {
        let active = prepared_sync_surface(vec![PreparedSyncWidget::new(
            2,
            Vector2::new(24.0, 16.0),
            41,
            PreparedSyncBehavior::Qualified,
            Rc::new(Cell::new(0)),
        )]);
        let candidate_surface = prepared_sync_surface(vec![PreparedSyncWidget::new(
            2,
            Vector2::new(64.0, 28.0),
            0,
            PreparedSyncBehavior::Qualified,
            Rc::new(Cell::new(0)),
        )]);
        let (mut runtime, _, _) = runtime_for_surface(active);
        let candidate = prepare_layout_candidate(&mut runtime, candidate_surface);
        let appearance = ResolvedAppearance::fixed(ThemeTokens::default());
        let context = FreshSurfacePaintProjectionContext::capture(&runtime, &candidate, appearance);
        let layout = candidate
            .layout_output()
            .expect("candidate layout should be available");
        let mut direct = empty_paint_plan_for_layout(layout, &context.theme);
        candidate
            .surface
            .paint_plan_with_hover_and_environment_and_appearance_into(
                layout,
                &context.theme,
                context.environment.clone(),
                context.appearance,
                context.hovered_container,
                context.active_scroll_affordance,
                &mut direct,
            );
        let damage = candidate.damage;
        let before = active_snapshot(&runtime);

        let candidate = runtime
            .prepare_fresh_surface_paint(candidate, appearance)
            .expect("candidate paint should not veto")
            .expect("candidate paint should be present");

        assert!(candidate.is_current(&runtime, appearance));
        assert_eq!(candidate.paint_plan(), &direct);
        assert_eq!(candidate.projection_context, context);
        assert_eq!(candidate.layout_candidate().damage, damage);
        assert_eq!(
            candidate.layout_candidate().view_delta.effect,
            ViewDeltaEffect::Unchanged
        );
        assert!(candidate.paint_plan().primitives.iter().any(|primitive| {
            matches!(
                primitive,
                crate::runtime::PaintPrimitive::FillRect(fill)
                    if fill.widget_id == 2 && fill.color == ThemeTokens::default().accent_mint
            )
        }));
        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
        candidate.discard();
        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
    }

    #[test]
    fn paint_candidate_currentness_rejects_runtime_context_change_without_active_mutation() {
        let active = paint_probe_surface(
            &[PaintProbeBehavior::Emit],
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(0)),
        );
        let candidate_calls = Rc::new(Cell::new(0));
        let candidate_drops = Rc::new(Cell::new(0));
        let candidate_surface = paint_probe_surface(
            &[PaintProbeBehavior::Emit],
            Rc::clone(&candidate_calls),
            Rc::clone(&candidate_drops),
        );
        let (mut runtime, _, _) = runtime_for_surface(active);
        let candidate = prepare_layout_candidate(&mut runtime, candidate_surface);
        let appearance = ResolvedAppearance::fixed(ThemeTokens::default());
        let candidate = runtime
            .prepare_fresh_surface_paint(candidate, appearance)
            .expect("candidate paint should not veto")
            .expect("candidate paint should be present");

        assert!(candidate.is_current(&runtime, appearance));
        let before = active_snapshot(&runtime);
        runtime.interaction.hover.widget = Some(2);
        assert!(!candidate.is_current(&runtime, appearance));

        candidate.discard();
        assert_eq!(candidate_calls.get(), 1);
        assert_eq!(candidate_drops.get(), 1);
        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
    }

    #[test]
    fn post_projection_context_revalidation_rejects_changed_inputs() {
        let active = paint_probe_surface(
            &[PaintProbeBehavior::Emit],
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(0)),
        );
        let candidate_surface = paint_probe_surface(
            &[PaintProbeBehavior::Emit],
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(0)),
        );
        let (mut runtime, _, _) = runtime_for_surface(active);
        let mut candidate = prepare_layout_candidate(&mut runtime, candidate_surface);
        let appearance = ResolvedAppearance::fixed(ThemeTokens::default());
        let context = FreshSurfacePaintProjectionContext::capture(&runtime, &candidate, appearance);

        assert!(context.is_current(&runtime, &candidate, appearance));
        assert!(!context.is_current(
            &runtime,
            &candidate,
            ResolvedAppearance::fixed(ThemeTokens::light())
        ));

        candidate
            .surface
            .set_window_environment(crate::runtime::WindowEnvironment::new(
                crate::theme::DpiScale::new(2.0),
                None,
                false,
                false,
            ));
        assert!(!context.is_current(&runtime, &candidate, appearance));
        candidate.discard();
    }

    #[test]
    fn unsupported_runtime_paint_context_vetoes_before_any_candidate_callback() {
        #[derive(Clone, Copy)]
        enum ContextCase {
            Hover,
            Capture,
            ScrollAffordance,
            DragPreview,
            Tooltip,
            Devtools,
        }

        for case in [
            ContextCase::Hover,
            ContextCase::Capture,
            ContextCase::ScrollAffordance,
            ContextCase::DragPreview,
            ContextCase::Tooltip,
            ContextCase::Devtools,
        ] {
            let active = paint_probe_surface(
                &[PaintProbeBehavior::Emit],
                Rc::new(Cell::new(0)),
                Rc::new(Cell::new(0)),
            );
            let candidate_calls = Rc::new(Cell::new(0));
            let candidate_drops = Rc::new(Cell::new(0));
            let candidate_surface = paint_probe_surface(
                &[PaintProbeBehavior::Emit],
                Rc::clone(&candidate_calls),
                Rc::clone(&candidate_drops),
            );
            let (mut runtime, _, _) = runtime_for_surface(active);
            match case {
                ContextCase::Hover => {
                    runtime.interaction.hover.widget = Some(2);
                    runtime.interaction.hover.container = Some(1);
                }
                ContextCase::Capture => runtime.interaction.pointer.capture = Some(2),
                ContextCase::ScrollAffordance => {
                    runtime.interaction.hover.scroll_affordance = Some(1)
                }
                ContextCase::DragPreview => {
                    runtime.interaction.drag.session = Some(crate::runtime::DragSession::new(
                        crate::runtime::DragRequest::new(
                            crate::runtime::DragPreview::sized("preview", Vector2::new(24.0, 16.0)),
                            crate::gui::types::Point::new(1.0, 1.0),
                        ),
                    ));
                }
                ContextCase::Tooltip => runtime.interaction.tooltip.target = Some(2),
                ContextCase::Devtools => {
                    runtime.devtools_overlay = crate::runtime::DevtoolsOverlayOptions::enabled()
                }
            }
            let candidate = prepare_layout_candidate(&mut runtime, candidate_surface);
            let before = active_snapshot(&runtime);
            assert!(
                runtime
                    .prepare_fresh_surface_paint(
                        candidate,
                        ResolvedAppearance::fixed(ThemeTokens::default()),
                    )
                    .expect("unsupported context should be an inert veto")
                    .is_none()
            );
            assert_eq!(candidate_calls.get(), 0);
            assert_eq!(candidate_drops.get(), 1);
            assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
        }
    }

    #[test]
    fn stale_candidate_paint_is_inert_without_callbacks_or_active_mutation() {
        let active = paint_probe_surface(
            &[PaintProbeBehavior::Emit],
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(0)),
        );
        let candidate_calls = Rc::new(Cell::new(0));
        let candidate_drops = Rc::new(Cell::new(0));
        let candidate_surface = paint_probe_surface(
            &[PaintProbeBehavior::Emit],
            Rc::clone(&candidate_calls),
            Rc::clone(&candidate_drops),
        );
        let (mut runtime, _, _) = runtime_for_surface(active);
        let candidate = prepare_layout_candidate(&mut runtime, candidate_surface);
        let before = active_snapshot(&runtime);
        runtime.advance_fresh_surface_active_generation();
        assert!(
            runtime
                .prepare_fresh_surface_paint(
                    candidate,
                    ResolvedAppearance::fixed(ThemeTokens::default()),
                )
                .expect("stale candidate should be an inert veto")
                .is_none()
        );
        assert_eq!(candidate_calls.get(), 0);
        assert_eq!(candidate_drops.get(), 1);
        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
    }

    #[test]
    fn candidate_paint_panics_stop_later_callbacks_and_drop_partial_state_once() {
        let cases = [
            vec![PaintProbeBehavior::Panic, PaintProbeBehavior::Emit],
            vec![
                PaintProbeBehavior::Emit,
                PaintProbeBehavior::Panic,
                PaintProbeBehavior::Emit,
            ],
        ];

        for behaviors in cases {
            let active = paint_probe_surface(
                &vec![PaintProbeBehavior::Emit; behaviors.len()],
                Rc::new(Cell::new(0)),
                Rc::new(Cell::new(0)),
            );
            let candidate_calls = Rc::new(Cell::new(0));
            let candidate_drops = Rc::new(Cell::new(0));
            let candidate_surface = paint_probe_surface(
                &behaviors,
                Rc::clone(&candidate_calls),
                Rc::clone(&candidate_drops),
            );
            let (mut runtime, _, _) = runtime_for_surface(active);
            let candidate = prepare_layout_candidate(&mut runtime, candidate_surface);
            let before = active_snapshot(&runtime);
            let result = runtime.prepare_fresh_surface_paint(
                candidate,
                ResolvedAppearance::fixed(ThemeTokens::default()),
            );
            assert!(matches!(result, Err(FreshSurfacePaintVeto::Panicked)));
            let panic_index = behaviors
                .iter()
                .position(|behavior| matches!(behavior, PaintProbeBehavior::Panic))
                .expect("panic fixture");
            assert_eq!(candidate_calls.get(), panic_index + 1);
            assert_eq!(candidate_drops.get(), behaviors.len());
            assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
        }
    }

    #[test]
    fn unsupported_and_mixed_state_sync_veto_before_any_callback() {
        let cases = [
            (
                vec![PreparedSyncBehavior::Unsupported],
                PreparedWidgetStateSyncVeto::Unsupported,
            ),
            (
                vec![
                    PreparedSyncBehavior::Qualified,
                    PreparedSyncBehavior::Unsupported,
                ],
                PreparedWidgetStateSyncVeto::Unsupported,
            ),
        ];

        for (behaviors, expected) in cases {
            let active_calls = Rc::new(Cell::new(0));
            let candidate_calls = behaviors
                .iter()
                .map(|_| Rc::new(Cell::new(0)))
                .collect::<Vec<_>>();
            let active = prepared_sync_surface(
                behaviors
                    .iter()
                    .enumerate()
                    .map(|(index, _)| {
                        PreparedSyncWidget::new(
                            2 + index as u64,
                            Vector2::new(24.0, 16.0),
                            10 + index as u64,
                            PreparedSyncBehavior::Qualified,
                            Rc::clone(&active_calls),
                        )
                    })
                    .collect(),
            );
            let candidate_surface = prepared_sync_surface(
                behaviors
                    .iter()
                    .enumerate()
                    .map(|(index, behavior)| {
                        PreparedSyncWidget::new(
                            2 + index as u64,
                            Vector2::new(24.0, 16.0),
                            0,
                            *behavior,
                            Rc::clone(&candidate_calls[index]),
                        )
                    })
                    .collect(),
            );
            let (mut runtime, _, _) = runtime_for_surface(active);
            let request = runtime
                .issue_fresh_surface_refresh_request(RepaintScope::Projection)
                .expect("request");
            let before = active_snapshot(&runtime);
            let preparation = runtime
                .prepare_fresh_surface(candidate_surface, request)
                .expect("ordinary candidate should prepare");
            let result = runtime.prepare_fresh_surface_layout(preparation);
            assert!(matches!(result, Err(veto) if veto == expected));
            assert_eq!(active_calls.get(), 0);
            assert!(candidate_calls.iter().all(|calls| calls.get() == 0));
            assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
        }
    }

    #[test]
    fn first_and_mid_batch_panics_stop_later_callbacks_and_preserve_active_state() {
        let cases = [
            vec![PreparedSyncBehavior::Panic, PreparedSyncBehavior::Qualified],
            vec![
                PreparedSyncBehavior::Qualified,
                PreparedSyncBehavior::Panic,
                PreparedSyncBehavior::Qualified,
            ],
        ];

        for behaviors in cases {
            let candidate_calls = behaviors
                .iter()
                .map(|_| Rc::new(Cell::new(0)))
                .collect::<Vec<_>>();
            let active = prepared_sync_surface(
                behaviors
                    .iter()
                    .enumerate()
                    .map(|(index, _)| {
                        PreparedSyncWidget::new(
                            2 + index as u64,
                            Vector2::new(24.0, 16.0),
                            20 + index as u64,
                            PreparedSyncBehavior::Qualified,
                            Rc::new(Cell::new(0)),
                        )
                    })
                    .collect(),
            );
            let candidate_surface = prepared_sync_surface(
                behaviors
                    .iter()
                    .enumerate()
                    .map(|(index, behavior)| {
                        PreparedSyncWidget::new(
                            2 + index as u64,
                            Vector2::new(24.0, 16.0),
                            0,
                            *behavior,
                            Rc::clone(&candidate_calls[index]),
                        )
                    })
                    .collect(),
            );
            let (mut runtime, _, _) = runtime_for_surface(active);
            let request = runtime
                .issue_fresh_surface_refresh_request(RepaintScope::Projection)
                .expect("request");
            let before = active_snapshot(&runtime);
            let preparation = runtime
                .prepare_fresh_surface(candidate_surface, request)
                .expect("ordinary candidate should prepare");
            let result = runtime.prepare_fresh_surface_layout(preparation);
            assert!(matches!(result, Err(PreparedWidgetStateSyncVeto::Panicked)));

            let panic_index = behaviors
                .iter()
                .position(|behavior| matches!(behavior, PreparedSyncBehavior::Panic))
                .expect("panic fixture");
            assert_eq!(candidate_calls[panic_index].get(), 1);
            assert!(
                candidate_calls
                    .iter()
                    .skip(panic_index + 1)
                    .all(|calls| calls.get() == 0)
            );
            assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
        }
    }

    #[test]
    fn repeated_candidate_prepare_discard_reuses_bounded_workspace() {
        let active = prepared_sync_surface(vec![PreparedSyncWidget::new(
            2,
            Vector2::new(24.0, 16.0),
            41,
            PreparedSyncBehavior::Qualified,
            Rc::new(Cell::new(0)),
        )]);
        let (mut runtime, _, _) = runtime_for_surface(active);
        let mut capacities = None;
        for iteration in 0..3 {
            let candidate_surface = prepared_sync_surface(vec![PreparedSyncWidget::new(
                2,
                Vector2::new(40.0 + iteration as f32, 20.0),
                iteration,
                PreparedSyncBehavior::Qualified,
                Rc::new(Cell::new(0)),
            )]);
            let request = runtime
                .issue_fresh_surface_refresh_request(RepaintScope::Projection)
                .expect("request");
            let preparation = runtime
                .prepare_fresh_surface(candidate_surface, request)
                .expect("ordinary candidate should prepare");
            let candidate = runtime
                .prepare_fresh_surface_layout(preparation)
                .expect("qualified synchronization should not veto")
                .expect("candidate layout should be prepared");
            candidate.discard();
            let next = runtime
                .layout_engine
                .prepared_workspace_capacity_signature()
                .expect("prepared workspace should be retained");
            if let Some(previous) = capacities {
                assert_eq!(previous, next);
            } else {
                capacities = Some(next);
            }
        }
    }

    #[test]
    fn candidate_root_revision_exhaustion_vetoes_before_callbacks() {
        let candidate_calls = Rc::new(Cell::new(0));
        let active = prepared_sync_surface(vec![PreparedSyncWidget::new(
            2,
            Vector2::new(24.0, 16.0),
            41,
            PreparedSyncBehavior::Qualified,
            Rc::new(Cell::new(0)),
        )]);
        let candidate_surface = prepared_sync_surface(vec![PreparedSyncWidget::new(
            2,
            Vector2::new(24.0, 16.0),
            0,
            PreparedSyncBehavior::Qualified,
            Rc::clone(&candidate_calls),
        )]);
        let (mut runtime, _, _) = runtime_for_surface(active);
        runtime.layout_root_authority = LayoutAuthorityEvidence::new(1, u64::MAX - 1);
        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("request");
        let before = active_snapshot(&runtime);
        let preparation = runtime
            .prepare_fresh_surface(candidate_surface, request)
            .expect("candidate preparation should reach the root fence");
        assert!(
            runtime
                .prepare_fresh_surface_layout(preparation)
                .expect("root exhaustion should be an inert veto")
                .is_none()
        );
        assert_eq!(candidate_calls.get(), 0);
        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
    }

    #[test]
    fn complete_geometry_evidence_is_allowed_without_active_mutation() {
        let (mut runtime, _, _) = runtime_fixture();
        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("request");
        let before = active_snapshot(&runtime);
        let preparation = runtime
            .prepare_fresh_surface(ordinary_surface(Vector2::new(32.0, 16.0)), request)
            .expect("exact geometry evidence should prepare");
        let candidate = runtime
            .prepare_fresh_surface_layout(preparation)
            .expect("geometry preparation should not veto")
            .expect("geometry candidate should be prepared");
        assert_eq!(candidate.view_delta.effect, ViewDeltaEffect::Geometry);
        assert!(candidate.is_current(&runtime));
        assert!(!candidate.damage.full_viewport);
        assert_eq!(candidate.damage.candidate_count, 1);
        let damage = candidate.damage.candidates[0].expect("geometry damage candidate");
        assert_eq!(damage.node_id, 2);
        assert_eq!(damage.effect, ViewDeltaEffect::Geometry);
        assert_eq!(damage.old_bounds, runtime.layout.rects.get(&2).copied());
        assert_eq!(
            damage.new_bounds,
            candidate
                .layout_output()
                .and_then(|layout| layout.rects.get(&2).copied())
        );
        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
        candidate.discard();
        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
    }

    #[test]
    fn prepared_refresh_publishes_one_owned_plan_and_runtime_transaction() {
        let (mut runtime, pull_calls, _) = runtime_fixture();
        let before = runtime.refresh_counters();
        let pull_before = pull_calls.get();
        let appearance = ResolvedAppearance::fixed(ThemeTokens::dark());
        let active_generation_before = runtime.fresh_surface_active_generation;
        let request_revision_before = runtime.fresh_surface_request_revision;
        let prepared = runtime
            .prepare_fresh_surface_refresh(RepaintScope::Surface, appearance)
            .expect("ordinary neutral refresh should prepare");
        let old_request = runtime
            .fresh_surface_request
            .expect("prepared refresh should retain its request");
        let old_candidate = runtime
            .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), old_request)
            .expect("old request should admit a candidate before publication");
        assert!(old_candidate.is_current(&runtime));
        assert_eq!(
            old_request.active_surface_generation,
            active_generation_before
        );
        assert_eq!(old_request.request_revision, request_revision_before + 1);
        let publication = runtime
            .publish_prepared_surface_refresh(prepared)
            .expect("prepared refresh should remain current through publication");
        let (paint_plan, published_appearance, terminal_messages, retired_candidate) =
            publication.into_parts();

        assert_eq!(published_appearance, appearance);
        assert!(terminal_messages.is_empty());
        assert!(retired_candidate.is_none());
        assert!(
            !paint_plan
                .expect("full refresh should carry a paint plan")
                .primitives
                .is_empty()
        );
        assert_eq!(pull_calls.get(), pull_before + 1);
        assert_eq!(runtime.fresh_surface_request, None);
        assert_eq!(
            runtime.fresh_surface_active_generation,
            active_generation_before + 1
        );
        assert_eq!(
            runtime.fresh_surface_request_revision,
            old_request.request_revision
        );
        assert!(!runtime.fresh_surface_request_is_current(old_request));
        assert!(!old_candidate.is_current(&runtime));
        assert!(
            runtime
                .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), old_request)
                .is_none()
        );
        old_candidate.discard();

        let next_request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Surface)
            .expect("next request should remain admissible");
        assert_eq!(
            next_request.request_revision,
            old_request.request_revision + 1
        );
        assert_eq!(
            next_request.active_surface_generation,
            active_generation_before + 1
        );
        assert_eq!(
            runtime.fresh_surface_active_generation,
            active_generation_before + 1
        );
        let after = runtime.refresh_counters();
        assert_eq!(
            after.application_projection,
            before.application_projection + 1
        );
        assert_eq!(after.runtime_projection, before.runtime_projection + 1);
        assert_eq!(after.widget_state_sync, before.widget_state_sync + 1);
        assert_eq!(after.layout, before.layout + 1);
        assert_eq!(runtime.layout_root, runtime.surface.layout_node());
    }

    #[test]
    fn prepared_refresh_hold_and_discard_preserve_active_state() {
        let (mut runtime, _, _) = runtime_fixture();
        let before = active_snapshot(&runtime);
        let prepared = runtime
            .prepare_fresh_surface_refresh(
                RepaintScope::Projection,
                ResolvedAppearance::fixed(ThemeTokens::dark()),
            )
            .expect("ordinary neutral refresh should prepare");

        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
        prepared.discard();
        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
    }

    #[test]
    fn replacement_plan_veto_preserves_authority_before_callbacks() {
        let prepare_calls = Rc::new(Cell::new(0));
        let (mut runtime, _, _) =
            runtime_for_surface(retiring_surface(1, Rc::clone(&prepare_calls)));
        runtime.bridge.surface = retiring_surface(2, Rc::clone(&prepare_calls));
        let mut prepared = runtime
            .prepare_fresh_surface_refresh(
                RepaintScope::Projection,
                ResolvedAppearance::fixed(ThemeTokens::dark()),
            )
            .expect("retiring successor should prepare");
        let request = runtime
            .fresh_surface_request
            .expect("prepared refresh should retain its request");
        let active_generation = runtime.fresh_surface_active_generation;
        let request_revision = runtime.fresh_surface_request_revision;
        assert!(prepared.is_current(&mut runtime));

        let PreparedSurfaceRefresh::Full {
            paint_candidate, ..
        } = &mut prepared
        else {
            panic!("ordinary replacement fixture should use full preparation")
        };
        paint_candidate
            .layout_candidate
            .surface
            .find_widget_mut(2)
            .expect("candidate retiring widget")
            .widget_mut();
        assert!(prepared.is_current(&mut runtime));

        assert!(runtime.publish_prepared_surface_refresh(prepared).is_none());
        assert_eq!(runtime.fresh_surface_request, Some(request));
        assert_eq!(runtime.fresh_surface_active_generation, active_generation);
        assert_eq!(runtime.fresh_surface_request_revision, request_revision);
        assert_eq!(prepare_calls.get(), 0);
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
    fn stale_or_vetoed_preparation_exposes_no_damage_candidate() {
        let (mut runtime, _, _) = runtime_fixture();
        let stale_request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("stale request");
        let current_request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("current request");
        assert!(
            runtime
                .prepare_fresh_surface(ordinary_surface(Vector2::new(32.0, 16.0)), stale_request)
                .is_none()
        );
        runtime.advance_fresh_surface_active_generation();
        assert!(
            runtime
                .prepare_fresh_surface(ordinary_surface(Vector2::new(32.0, 16.0)), current_request)
                .is_none()
        );

        let active = prepared_sync_surface(vec![PreparedSyncWidget::new(
            2,
            Vector2::new(24.0, 16.0),
            41,
            PreparedSyncBehavior::Qualified,
            Rc::new(Cell::new(0)),
        )]);
        let candidate_surface = prepared_sync_surface(vec![PreparedSyncWidget::new(
            2,
            Vector2::new(32.0, 16.0),
            0,
            PreparedSyncBehavior::Unsupported,
            Rc::new(Cell::new(0)),
        )]);
        let (mut runtime, _, _) = runtime_for_surface(active);
        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("veto request");
        let before = active_snapshot(&runtime);
        let preparation = runtime
            .prepare_fresh_surface(candidate_surface, request)
            .expect("candidate admission should precede the state-sync veto");
        assert!(matches!(
            runtime.prepare_fresh_surface_layout(preparation),
            Err(PreparedWidgetStateSyncVeto::Unsupported)
        ));
        assert_active_snapshot_unchanged(&before, &active_snapshot(&runtime));
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
        runtime.interaction.focus.owner = Some(
            super::super::interaction_state::RuntimeFocusOwner::Widget(2),
        );
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
        let (mut exhausted_runtime, exhausted_pull_calls, exhausted_project_calls) =
            runtime_fixture();
        exhausted_runtime.fresh_surface_active_generation = u64::MAX - 1;
        let pull_before = exhausted_pull_calls.get();
        let project_before = exhausted_project_calls.get();
        let request_revision_before = exhausted_runtime.fresh_surface_request_revision;
        assert!(
            exhausted_runtime
                .issue_fresh_surface_refresh_request(RepaintScope::Projection)
                .is_none()
        );
        assert_eq!(exhausted_pull_calls.get(), pull_before);
        assert_eq!(exhausted_project_calls.get(), project_before);
        assert_eq!(
            exhausted_runtime.fresh_surface_request_revision,
            request_revision_before
        );
        assert_eq!(exhausted_runtime.fresh_surface_request, None);
        assert!(exhausted_runtime.fresh_surface_authority_exhausted);
        assert!(
            exhausted_runtime
                .prepare_fresh_surface_refresh(
                    RepaintScope::Projection,
                    ResolvedAppearance::fixed(ThemeTokens::dark()),
                )
                .is_none()
        );
        assert_eq!(exhausted_pull_calls.get(), pull_before);
        assert_eq!(exhausted_project_calls.get(), project_before);

        let (mut runtime, pull_calls, project_calls) = runtime_fixture();
        runtime.fresh_surface_active_generation = u64::MAX - 2;
        let request = runtime
            .issue_fresh_surface_refresh_request(RepaintScope::Projection)
            .expect("request");
        let candidate = runtime
            .prepare_fresh_surface(ordinary_surface(Vector2::new(24.0, 16.0)), request)
            .expect("candidate");
        assert!(candidate.is_current(&runtime));

        let fresh_surface_state_before = (
            runtime.fresh_surface_active_generation,
            runtime.fresh_surface_request_revision,
            runtime.fresh_surface_request,
            runtime.fresh_surface_authority_exhausted,
        );
        let layout_root_authority_before = runtime.layout_root_authority;

        let pull_before = pull_calls.get();
        let project_before = project_calls.get();
        let counters_before = runtime.refresh_counters();
        let owner_reconciliations_before = runtime.declarative_owner_ledger.reconciliation_count();
        runtime.refresh_with_scope(RepaintScope::Projection);
        assert_eq!(pull_calls.get(), pull_before + 1);
        assert_eq!(project_calls.get(), project_before);
        assert_eq!(
            runtime.refresh_counters().application_projection,
            counters_before.application_projection + 1
        );
        assert_eq!(
            runtime.declarative_owner_ledger.reconciliation_count(),
            owner_reconciliations_before + 1
        );
        assert_ne!(
            (
                runtime.fresh_surface_active_generation,
                runtime.fresh_surface_request_revision,
                runtime.fresh_surface_request,
                runtime.fresh_surface_authority_exhausted,
            ),
            fresh_surface_state_before
        );
        assert_eq!(runtime.fresh_surface_active_generation, u64::MAX - 1);
        assert_eq!(runtime.fresh_surface_request, None);
        assert!(!runtime.fresh_surface_authority_exhausted);
        assert_ne!(runtime.layout_root_authority, layout_root_authority_before);
        assert_eq!(
            candidate.authority.layout_root_authority,
            layout_root_authority_before
        );
        assert!(!candidate.is_current(&runtime));

        let counters_before_relayout = runtime.refresh_counters();
        let owner_reconciliations_before_relayout =
            runtime.declarative_owner_ledger.reconciliation_count();
        runtime.relayout();
        assert_eq!(runtime.refresh_counters(), counters_before_relayout);
        assert_eq!(
            runtime.declarative_owner_ledger.reconciliation_count(),
            owner_reconciliations_before_relayout + 1
        );
        assert_eq!(runtime.layout_root, runtime.surface.layout_node());
        assert!(!runtime.scratch.projection_source.records.is_empty());
        candidate.discard();
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
