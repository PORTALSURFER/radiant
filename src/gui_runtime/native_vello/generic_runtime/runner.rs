//! Runner state and redraw coordination for the generic native Vello runtime.

use super::frame_stage_admission::WindowStageOwner;
#[cfg(target_os = "macos")]
use super::native_semantic_accessibility::NativeSemanticAccessibilityAdapter;
use super::recovery::{
    NativeRecoveryCandidate, NativeRecoveryCoordinator, NativeRecoveryEpisodeToken,
    NativeRecoveryRequest,
};
use super::renderer_recovery::{
    NativeRendererRecoveryCommitFacts, NativeRendererRecoveryPolicy,
    NativeRendererRecoveryWindowKind, construct_renderer_recovery_candidate,
    preflight_renderer_recovery, renderer_recovery_commit_is_valid,
};
use super::{
    ActivationRevealController, ApplicationReopenRegistration, AuxiliaryNativeWindow,
    CpuFrameFairnessLedger, CpuFrameObservationAdmission, CpuFrameObservationCapture,
    CpuFrameObservationLedger, CpuFrameObservationOwner, CpuFramePendingRedrawAge,
    DeviceLossRegistration, FrameScheduleKey, FrameWork, FrameWorkReason,
    GenericNativeAdapterOwner, GenericNativeRuntimeCore, GenericRouteOutcome,
    NativeAdapterGeneration, NativeAutomationTargetExporter, NativeClosingProgress,
    NativeFrameDiagnosticsPublication, NativeFrameScheduler, NativeGenericRunError,
    NativeLifecycle, NativeRenderDeviceErrorKind, NativeResourceMaintenanceTurn,
    NativeRunnerInputState, NativeRunnerTimingState, NativeRunnerWindowState,
    NativeVelloFrameState, PaintPlanCacheDecision, RuntimeWakeup, SceneRebuildMode,
    SurfaceSceneEncodeContext, TimedFrameCadence, animation_frame_interval,
    animation_frame_interval_for_normalized_fps, encode_native_paint_segment_payloads,
    encode_surface_paint_plan_to_scene, slow_render_profile_enabled, timed_frame_cadence,
    timed_frame_target_fps,
};
use super::{
    frame_state::NativeSceneValidityFingerprint,
    retained_paint_segments::NativePaintSegmentEligibilityPlan,
    runner_state::{NativeTargetGeneration, NativeWindowDiagnosticIdentityAllocator},
    scene::{
        ArtifactFeasibilityObservation, NativePaintSegmentPayload,
        materialize_native_paint_segment_artifacts,
    },
    scene_texture::NativeFrameRenderFailure,
};
use crate::{
    gui::types::Vector2,
    gui_runtime::native_vello::NativeTextRenderer,
    runtime::{
        FrameProfile, NativeCpuFrameFairnessDiagnostics, NativeCpuFrameObservationDiagnostics,
        NativeFrameDiagnostics, NativeRunOptions, NativeWindowDiagnosticIdentity,
        RuntimeAnimationActivity, RuntimeBridge,
    },
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};
use vello::Scene;
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy},
};

pub(super) struct GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) options: NativeRunOptions,
    pub(super) core: GenericNativeRuntimeCore<Bridge, Message>,
    pub(super) runtime_wakeup: RuntimeWakeup,
    pub(super) activation_reveal: ActivationRevealController,
    pub(super) application_reopen_proxy: Option<EventLoopProxy<super::RuntimeUserEvent>>,
    pub(super) application_reopen_events: Option<ApplicationReopenRegistration>,
    /// One application-level adapter shared by the primary and auxiliary
    /// generic-native windows. Auxiliary runners borrow it at event boundaries.
    pub(super) adapter: Option<GenericNativeAdapterOwner>,
    #[cfg(target_os = "macos")]
    pub(super) native_semantic_accessibility: Option<NativeSemanticAccessibilityAdapter>,
    pub(super) window: NativeRunnerWindowState,
    pub(super) frame: NativeVelloFrameState,
    pub(super) input: NativeRunnerInputState,
    pub(super) timing: NativeRunnerTimingState,
    pub(super) native_window_diagnostic_identity_allocator: NativeWindowDiagnosticIdentityAllocator,
    pub(super) frame_scheduler: NativeFrameScheduler,
    pub(super) frame_stage_owner: WindowStageOwner,
    pub(super) cpu_frame_fairness: Option<CpuFrameFairnessLedger>,
    pub(super) cpu_frame_observation: Option<CpuFrameObservationLedger>,
    pub(super) cpu_frame_observation_capture: CpuFrameObservationCapture,
    pub(super) frame_diagnostics_enabled: bool,
    pub(super) frame_profile_enabled: bool,
    pub(super) frame_observation_enabled: bool,
    pub(super) frame_diagnostics_publication: NativeFrameDiagnosticsPublication,
    pub(super) automation_targets: NativeAutomationTargetExporter,
    pub(super) auxiliary_windows: Vec<AuxiliaryNativeWindow<Message>>,
    native_lifecycle: NativeLifecycle,
    auxiliary_owner: bool,
    terminal_cause: Option<NativeGenericRunError>,
    pub(super) recovery: NativeRecoveryCoordinator,
    pub(super) renderer_recovery: NativeRendererRecoveryPolicy,
    pub(super) recovery_cause: Option<NativeGenericRunError>,
    pub(super) recovery_primary_was_visible: bool,
    pub(super) recovery_auxiliary_followup_pending: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct AppliedRouteOutcome {
    pub(super) exit_requested: bool,
    pub(super) sync_auxiliary_windows_now: bool,
}

const fn recovery_completion_is_admissible(recovery_expired: bool) -> bool {
    !recovery_expired
}

/// One-shot admission for materializing artifacts from one completed scene encode.
///
/// The runner is the only production owner that can construct this token. Its
/// private fields keep the authoritative scene, evidence, eligibility plan,
/// payloads, and target generation together until the materializer consumes it.
pub(super) struct NativePaintSegmentArtifactAdmission<'a> {
    scene: &'a Scene,
    feasibility: ArtifactFeasibilityObservation,
    plan: NativePaintSegmentEligibilityPlan,
    payloads: Vec<NativePaintSegmentPayload>,
    scene_validity: NativeSceneValidityFingerprint,
    target_generation: NativeTargetGeneration,
}

impl<'a> NativePaintSegmentArtifactAdmission<'a> {
    pub(super) fn into_parts(
        self,
    ) -> (
        &'a Scene,
        ArtifactFeasibilityObservation,
        NativePaintSegmentEligibilityPlan,
        Vec<NativePaintSegmentPayload>,
        NativeSceneValidityFingerprint,
        NativeTargetGeneration,
    ) {
        let Self {
            scene,
            feasibility,
            plan,
            payloads,
            scene_validity,
            target_generation,
        } = self;
        (
            scene,
            feasibility,
            plan,
            payloads,
            scene_validity,
            target_generation,
        )
    }
}

#[cfg(test)]
pub(super) fn materialize_native_paint_segment_artifacts_for_test(
    scene: &Scene,
    feasibility: ArtifactFeasibilityObservation,
    plan: NativePaintSegmentEligibilityPlan,
    payloads: Vec<NativePaintSegmentPayload>,
    scene_validity: NativeSceneValidityFingerprint,
    target_generation: NativeTargetGeneration,
) -> super::scene::NativePaintSegmentArtifactMaterialization {
    materialize_native_paint_segment_artifacts(NativePaintSegmentArtifactAdmission {
        scene,
        feasibility,
        plan,
        payloads,
        scene_validity,
        target_generation,
    })
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    const REDRAW_REISSUE_AFTER: Duration = Duration::from_millis(16);
    const REDRAW_REISSUE_LOG_AFTER: Duration = Duration::from_millis(32);

    pub(super) fn new(options: NativeRunOptions, bridge: Bridge, viewport: Vector2) -> Self {
        let (native_window_diagnostic_identity_allocator, native_window_diagnostic_identity) =
            NativeWindowDiagnosticIdentityAllocator::for_primary();
        Self::new_with_diagnostic_identity(
            options,
            bridge,
            viewport,
            native_window_diagnostic_identity,
            native_window_diagnostic_identity_allocator,
        )
    }

    pub(super) fn new_with_diagnostic_identity(
        options: NativeRunOptions,
        bridge: Bridge,
        viewport: Vector2,
        native_window_diagnostic_identity: Option<NativeWindowDiagnosticIdentity>,
        native_window_diagnostic_identity_allocator: NativeWindowDiagnosticIdentityAllocator,
    ) -> Self {
        let activation_reveal = ActivationRevealController::new(&options);
        let text_renderer = NativeTextRenderer::with_options(&options.text);
        let debug_layout = options.frame.debug_layout;
        let devtools_overlay = options.frame.devtools;
        let retained_surface_cache = options.frame.retained_surface_cache;
        let core = GenericNativeRuntimeCore::new_with_frame_options(
            bridge,
            viewport,
            debug_layout,
            devtools_overlay,
        );
        let frame_diagnostics_enabled = core.has_frame_diagnostics_observer();
        let frame_profile_enabled =
            options.frame.profiling.is_frame() && core.has_frame_profile_observer();
        let frame_observation_enabled = frame_diagnostics_enabled || frame_profile_enabled;
        Self {
            options,
            core,
            runtime_wakeup: RuntimeWakeup::default(),
            activation_reveal,
            application_reopen_proxy: None,
            application_reopen_events: None,
            adapter: None,
            #[cfg(target_os = "macos")]
            native_semantic_accessibility: None,
            window: NativeRunnerWindowState::default(),
            frame: NativeVelloFrameState::new(text_renderer, retained_surface_cache),
            input: NativeRunnerInputState::default(),
            timing: NativeRunnerTimingState::new(native_window_diagnostic_identity),
            native_window_diagnostic_identity_allocator,
            frame_scheduler: NativeFrameScheduler::default(),
            frame_stage_owner: WindowStageOwner::new(FrameScheduleKey::Primary),
            cpu_frame_fairness: Some(CpuFrameFairnessLedger::default()),
            cpu_frame_observation: frame_diagnostics_enabled
                .then(CpuFrameObservationLedger::default)
                .or_else(|| frame_profile_enabled.then(CpuFrameObservationLedger::default)),
            cpu_frame_observation_capture: CpuFrameObservationCapture::default(),
            frame_diagnostics_enabled,
            frame_profile_enabled,
            frame_observation_enabled,
            frame_diagnostics_publication: NativeFrameDiagnosticsPublication::default(),
            automation_targets: NativeAutomationTargetExporter::from_env(),
            auxiliary_windows: Vec::new(),
            native_lifecycle: NativeLifecycle::default(),
            auxiliary_owner: false,
            terminal_cause: None,
            recovery: NativeRecoveryCoordinator::default(),
            renderer_recovery: NativeRendererRecoveryPolicy::default(),
            recovery_cause: None,
            recovery_primary_was_visible: false,
            recovery_auxiliary_followup_pending: false,
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn republish_native_semantic_accessibility_passively(&mut self) {
        let Some(mut adapter) = self.native_semantic_accessibility.take() else {
            return;
        };
        match adapter.publish_passive(&self.core.runtime, self.window.native_window_focused) {
            Ok(()) => self.native_semantic_accessibility = Some(adapter),
            Err(error) => self.discard_failed_native_semantic_accessibility(adapter, error),
        }
    }

    #[cfg(target_os = "macos")]
    fn discard_failed_native_semantic_accessibility(
        &mut self,
        mut adapter: NativeSemanticAccessibilityAdapter,
        error: String,
    ) {
        adapter.close_lease(&mut self.core.runtime);
        adapter.retire();
        warn!(
            error = %error,
            "radiant native semantic accessibility adapter retired after host publication failure"
        );
    }

    pub(super) fn mark_as_auxiliary(&mut self) {
        self.auxiliary_owner = true;
        self.native_window_diagnostic_identity_allocator =
            NativeWindowDiagnosticIdentityAllocator::exhausted();
        self.cpu_frame_fairness = None;
        self.cpu_frame_observation = None;
    }

    pub(super) fn require_primary_frame_diagnostics_schedule_admission(&mut self) {
        if self.frame_observation_enabled && !self.auxiliary_owner {
            self.frame_diagnostics_publication
                .require_schedule_admission();
        }
    }

    pub(super) fn stage_frame_diagnostics(&mut self, diagnostics: NativeFrameDiagnostics) {
        if !self.frame_observation_enabled {
            return;
        }
        if self.auxiliary_owner {
            // Auxiliary runners retain the existing one-slot child-to-parent
            // bridge handoff. This is not the application callback; the
            // parent drains and publishes it at the event boundary.
            self.core
                .runtime
                .host_observe_frame_diagnostics(diagnostics);
        } else {
            self.frame_diagnostics_publication.stage(diagnostics);
        }
    }

    pub(super) fn publish_staged_frame_diagnostics(&mut self) {
        if !self.frame_observation_enabled {
            return;
        }
        if let Some(mut diagnostics) = self.frame_diagnostics_publication.take_ready() {
            diagnostics.cpu_fairness = self
                .cpu_frame_fairness
                .as_ref()
                .map_or_else(NativeCpuFrameFairnessDiagnostics::default, |ledger| {
                    ledger.project_frame_diagnostics(&FrameScheduleKey::Primary)
                });
            diagnostics.cpu_observation = self
                .cpu_frame_observation
                .as_ref()
                .map_or_else(NativeCpuFrameObservationDiagnostics::default, |ledger| {
                    ledger.project_frame_diagnostics(&FrameScheduleKey::Primary)
                });
            if self.frame_diagnostics_enabled {
                self.core
                    .runtime
                    .host_observe_frame_diagnostics(diagnostics);
            }
            if self.frame_profile_enabled {
                self.core
                    .runtime
                    .host_observe_frame_profile(FrameProfile::from(diagnostics));
            }
        }
    }

    pub(super) fn allocate_auxiliary_window_diagnostic_identity(
        &mut self,
    ) -> Option<NativeWindowDiagnosticIdentity> {
        self.native_window_diagnostic_identity_allocator.allocate()
    }

    pub(super) fn record_frame_schedule_admission(&mut self, key: FrameScheduleKey) {
        let is_primary = matches!(&key, FrameScheduleKey::Primary);
        if let Some(ledger) = self.cpu_frame_fairness.as_mut() {
            ledger.mark_admitted(&key);
        }
        self.frame_scheduler.record_admission(key);
        if self.frame_observation_enabled && !self.auxiliary_owner && is_primary {
            self.frame_diagnostics_publication
                .mark_schedule_admission_recorded();
        }
    }

    pub(super) fn begin_cpu_frame_observation(
        &mut self,
        key: FrameScheduleKey,
        now: Instant,
    ) -> Option<CpuFrameObservationAdmission> {
        let (frame_work, cadence_target_fps, pending_redraw_age) =
            self.cpu_frame_observation_snapshot(now);
        self.cpu_frame_observation
            .as_mut()
            .map(|ledger| ledger.begin(key, frame_work, cadence_target_fps, pending_redraw_age))
    }

    fn cpu_frame_observation_snapshot(
        &mut self,
        now: Instant,
    ) -> (FrameWork, Option<u32>, CpuFramePendingRedrawAge) {
        let frame_work = self.timing.pending_frame_work;
        let cadence_target_fps = Some(timed_frame_target_fps(
            self.options.normalized_target_fps(),
            self.core.animation_activity(),
            self.core.has_focused_text_input(),
        ));
        let pending_redraw_age = self.pending_redraw_age(now);
        (frame_work, cadence_target_fps, pending_redraw_age)
    }

    pub(super) fn begin_cpu_frame_observation_with_owner(
        &mut self,
        owner: &mut CpuFrameObservationOwner<'_>,
        now: Instant,
    ) -> CpuFrameObservationAdmission {
        let (frame_work, cadence_target_fps, pending_redraw_age) =
            self.cpu_frame_observation_snapshot(now);
        owner.begin(frame_work, cadence_target_fps, pending_redraw_age)
    }

    pub(super) fn finish_cpu_frame_observation(
        &mut self,
        admission: Option<CpuFrameObservationAdmission>,
        redraw_failed: bool,
    ) {
        let capture = std::mem::take(&mut self.cpu_frame_observation_capture);
        self.finish_cpu_frame_observation_with_capture(admission, capture, redraw_failed);
    }

    pub(super) fn finish_cpu_frame_observation_with_capture(
        &mut self,
        admission: Option<CpuFrameObservationAdmission>,
        capture: CpuFrameObservationCapture,
        redraw_failed: bool,
    ) {
        let (Some(ledger), Some(admission)) = (self.cpu_frame_observation.as_mut(), admission)
        else {
            return;
        };
        ledger.finish(admission, capture, redraw_failed);
        if self.frame_observation_enabled && !self.auxiliary_owner {
            self.frame_diagnostics_publication
                .mark_observation_finalized();
        }
    }

    pub(super) fn finish_cpu_frame_observation_with_owner(
        &mut self,
        owner: &mut CpuFrameObservationOwner<'_>,
        admission: CpuFrameObservationAdmission,
        redraw_failed: bool,
    ) {
        let capture = std::mem::take(&mut self.cpu_frame_observation_capture);
        owner.finish(admission, capture, redraw_failed);
        if self.frame_observation_enabled && !self.auxiliary_owner {
            self.frame_diagnostics_publication
                .mark_observation_finalized();
        }
    }

    pub(super) fn take_cpu_frame_observation_capture(&mut self) -> CpuFrameObservationCapture {
        std::mem::take(&mut self.cpu_frame_observation_capture)
    }

    pub(super) fn mark_cpu_frame_observation_recovery(&mut self) {
        self.cpu_frame_observation_capture.mark_recovery_triggered();
    }

    pub(super) fn remove_cpu_frame_observation(&mut self, key: &FrameScheduleKey) {
        if let Some(ledger) = self.cpu_frame_fairness.as_mut() {
            ledger.remove(key);
        }
        if let Some(ledger) = self.cpu_frame_observation.as_mut() {
            ledger.remove(key);
        }
    }

    pub(super) const fn is_running(&self) -> bool {
        self.native_lifecycle.is_running()
    }

    pub(super) const fn is_closing(&self) -> bool {
        self.native_lifecycle.is_closing()
    }

    pub(super) const fn is_recovering(&self) -> bool {
        self.native_lifecycle.is_recovering()
    }

    #[cfg(target_os = "macos")]
    pub(super) fn attach_native_semantic_accessibility(
        &mut self,
        proxy: EventLoopProxy<super::RuntimeUserEvent>,
    ) {
        if self.auxiliary_owner || self.native_semantic_accessibility.is_some() {
            return;
        }
        let Some(window) = self.window.window.as_ref().cloned() else {
            return;
        };
        match NativeSemanticAccessibilityAdapter::attach(&window, proxy) {
            Ok(mut adapter) => match adapter
                .publish_passive(&self.core.runtime, self.window.native_window_focused)
            {
                Ok(()) => self.native_semantic_accessibility = Some(adapter),
                Err(error) => self.discard_failed_native_semantic_accessibility(adapter, error),
            },
            Err(error) => {
                warn!(error = %error, "radiant native semantic accessibility attachment withheld");
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn handle_native_semantic_accessibility_query(
        &mut self,
        query: super::super::runtime_event::NativeSemanticAccessibilityQuery,
    ) {
        let Some(mut adapter) = self.native_semantic_accessibility.take() else {
            return;
        };
        adapter.handle_query(&mut self.core.runtime, query);
        if adapter.is_attached() {
            self.native_semantic_accessibility = Some(adapter);
        } else {
            self.discard_failed_native_semantic_accessibility(
                adapter,
                String::from("native semantic query publication failed"),
            );
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn handle_native_numeric_accessibility_action(
        &mut self,
        token: u64,
        target: crate::gui::automation::AutomationTarget,
        action: super::super::runtime_event::NativeNumericAccessibilityAction,
    ) {
        let request = {
            let Some(adapter) = self.native_semantic_accessibility.as_mut() else {
                return;
            };
            adapter.finish_numeric_action();
            adapter.numeric_accessibility_request(token, target, action)
        };
        let Some(request) = request else {
            return;
        };
        let _ = self
            .core
            .runtime
            .dispatch_numeric_accessibility_action(request);
        self.republish_native_semantic_accessibility_passively();
    }

    #[cfg(target_os = "macos")]
    pub(super) fn invalidate_native_semantic_accessibility_geometry(&mut self) {
        let Some(window) = self.window.window.as_ref().cloned() else {
            return;
        };
        let Some(mut adapter) = self.native_semantic_accessibility.take() else {
            return;
        };
        {
            adapter.invalidate_window_generation(&window);
        }
        match adapter.publish_passive(&self.core.runtime, self.window.native_window_focused) {
            Ok(()) => self.native_semantic_accessibility = Some(adapter),
            Err(error) => self.discard_failed_native_semantic_accessibility(adapter, error),
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn close_native_semantic_accessibility(&mut self) {
        if let Some(mut adapter) = self.native_semantic_accessibility.take() {
            adapter.close_lease(&mut self.core.runtime);
            adapter.retire();
        }
    }

    pub(super) const fn is_stopped(&self) -> bool {
        self.native_lifecycle.is_stopped()
    }

    pub(super) fn recovery_deadline(&self) -> Option<Instant> {
        self.native_lifecycle.recovery_deadline()
    }

    pub(super) fn recovery_expired(&self, now: Instant) -> bool {
        self.native_lifecycle.recovery_expired(now)
    }

    pub(super) fn admit_device_recovery(&mut self) -> bool {
        if !self.native_lifecycle.admit_recovery() {
            return false;
        }
        if !self.core.begin_native_recovery() {
            let _ = self.native_lifecycle.finish_recovery();
            return false;
        }
        self.clear_cpu_frame_observation();
        self.fence_native_presentation();
        true
    }

    pub(super) fn clear_cpu_frame_observation(&mut self) {
        self.clear_cpu_frame_fairness();
        if let Some(ledger) = self.cpu_frame_observation.as_mut() {
            ledger.clear();
        }
    }

    pub(super) fn clear_cpu_frame_fairness(&mut self) {
        if let Some(ledger) = self.cpu_frame_fairness.as_mut() {
            ledger.clear();
        }
    }

    pub(super) fn finish_device_recovery(&mut self) -> bool {
        if !self.native_lifecycle.is_recovering() {
            return false;
        }
        if !self.core.finish_native_recovery() {
            return false;
        }
        if self.native_lifecycle.finish_recovery() {
            return true;
        }
        let _ = self.core.begin_native_recovery();
        false
    }

    pub(super) const fn native_shutdown_requested(&self) -> bool {
        !self.native_lifecycle.is_running()
    }

    pub(super) fn record_terminal_cause(&mut self, cause: NativeGenericRunError) -> bool {
        if self.terminal_cause.is_some() {
            return false;
        }
        self.terminal_cause = Some(cause);
        true
    }

    pub(super) fn has_terminal_cause(&self) -> bool {
        self.terminal_cause.is_some()
    }

    pub(super) fn should_initialize_runtime(&self) -> bool {
        self.is_running() && self.window.window.is_none() && !self.has_terminal_cause()
    }

    pub(super) fn admit_native_shutdown(
        &mut self,
        event_loop: &ActiveEventLoop,
        cause: Option<NativeGenericRunError>,
    ) {
        if self.is_closing() || self.native_lifecycle.is_stopped() {
            return;
        }
        let cause = if self.is_recovering() {
            self.recovery_cause.take().or(cause)
        } else {
            self.recovery_cause.take();
            self.recovery_auxiliary_followup_pending = false;
            cause
        };
        self.recovery.cancel();
        self.recovery_auxiliary_followup_pending = false;
        let now = Instant::now();
        if !self.native_lifecycle.admit_closing(now) {
            return;
        }
        if let Some(cause) = cause
            && self.record_terminal_cause(cause.clone())
        {
            error!(
                error = %cause,
                "radiant generic native vello: native shutdown admitted after terminal failure"
            );
        }
        #[cfg(target_os = "macos")]
        self.close_native_semantic_accessibility();
        let _ = self.core.runtime.begin_closing();
        self.fence_native_presentation();
        self.clear_cpu_frame_fairness();
        self.application_reopen_events.take();
        self.application_reopen_proxy.take();
        self.runtime_wakeup.clear_pending();
        let retiring_auxiliary_keys = self
            .auxiliary_windows
            .iter()
            .map(|window| FrameScheduleKey::Auxiliary(window.key().to_owned()))
            .collect::<Vec<_>>();
        for key in retiring_auxiliary_keys {
            self.remove_cpu_frame_observation(&key);
        }
        for window in &mut self.auxiliary_windows {
            window.begin_whole_run_retiring(event_loop);
        }
        if self.native_resource_ownership_is_empty() {
            self.stop_native_event_loop(event_loop);
        } else {
            self.schedule_native_closing(event_loop, now);
        }
    }

    pub(super) fn advance_native_closing(&mut self, event_loop: &ActiveEventLoop, now: Instant) {
        if !self.is_closing() {
            return;
        }
        let mut turn = NativeResourceMaintenanceTurn::new();
        let native_ownership_empty = self.retire_all_native_resources_with_turn(&mut turn)
            && !self.recovery.has_in_flight_candidate();
        let Some(progress) = self
            .native_lifecycle
            .observe_closing_opportunity(now, native_ownership_empty)
        else {
            return;
        };
        match progress {
            NativeClosingProgress::Complete | NativeClosingProgress::Cutoff => {
                self.stop_native_event_loop(event_loop);
            }
            NativeClosingProgress::Continue => self.schedule_native_closing(event_loop, now),
        }
    }

    fn schedule_native_closing(&self, event_loop: &ActiveEventLoop, now: Instant) {
        if let Some(deadline) = self.native_lifecycle.closing_deadline(now) {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        }
    }

    fn stop_native_event_loop(&mut self, event_loop: &ActiveEventLoop) {
        let _ = self.native_lifecycle.finish_closing();
        if !self.auxiliary_owner {
            event_loop.exit();
        }
    }

    pub(super) fn native_resource_ownership_is_empty(&self) -> bool {
        self.window.native_resources.is_none()
            && self.window.quarantined_native_resources.is_empty()
            && self
                .auxiliary_windows
                .iter()
                .all(AuxiliaryNativeWindow::native_resource_ownership_is_empty)
            && !self.recovery.has_in_flight_candidate()
    }

    pub(super) fn begin_native_resource_maintenance(&mut self) -> NativeResourceMaintenanceTurn {
        let mut turn = NativeResourceMaintenanceTurn::new();
        self.maintain_native_resources_with_turn(&mut turn);
        turn
    }

    pub(super) fn begin_native_resource_maintenance_and_wake_primary(
        &mut self,
    ) -> NativeResourceMaintenanceTurn {
        let mut turn = NativeResourceMaintenanceTurn::new();
        if self.maintain_native_resources_with_turn(&mut turn) {
            self.request_redraw_for_frame_work(FrameWork::None);
        }
        turn
    }

    pub(super) fn maintain_native_resources_with_turn(
        &mut self,
        turn: &mut NativeResourceMaintenanceTurn,
    ) -> bool {
        self.window.maintain_native_resources(turn);
        let retiring_auxiliary_keys = self
            .auxiliary_windows
            .iter()
            .filter(|window| window.is_retiring())
            .map(|window| FrameScheduleKey::Auxiliary(window.key().to_owned()))
            .collect::<Vec<_>>();
        for key in retiring_auxiliary_keys {
            self.remove_cpu_frame_observation(&key);
        }
        let auxiliary_count = self.auxiliary_windows.len();
        self.auxiliary_windows
            .retain_mut(|window| !window.maintain_native_resources_with_turn(turn));
        let removed_auxiliary = self.auxiliary_windows.len() != auxiliary_count;
        if removed_auxiliary {
            self.timing.deferred_auxiliary_window_sync = true;
        }
        removed_auxiliary
    }

    pub(super) fn retire_native_resources_with_turn(
        &mut self,
        turn: &mut NativeResourceMaintenanceTurn,
    ) -> bool {
        self.window.retire_native_resources(turn)
    }

    pub(super) fn retire_all_native_resources_with_turn(
        &mut self,
        turn: &mut NativeResourceMaintenanceTurn,
    ) -> bool {
        let primary_empty = self.retire_native_resources_with_turn(turn);
        let retiring_auxiliary_keys = self
            .auxiliary_windows
            .iter()
            .filter(|window| window.is_retiring())
            .map(|window| FrameScheduleKey::Auxiliary(window.key().to_owned()))
            .collect::<Vec<_>>();
        for key in retiring_auxiliary_keys {
            self.remove_cpu_frame_observation(&key);
        }
        let auxiliary_count = self.auxiliary_windows.len();
        self.auxiliary_windows
            .retain_mut(|window| !window.maintain_native_resources_with_turn(turn));
        if self.auxiliary_windows.len() != auxiliary_count {
            self.timing.deferred_auxiliary_window_sync = true;
        }
        primary_empty && self.auxiliary_windows.is_empty()
    }

    pub(super) fn record_successful_native_submission(&mut self) {
        if let Some(resources) = self.window.native_resources.as_mut() {
            resources.record_successful_native_submission();
        }
    }

    /// Recover one eligible FrameRender failure after the failed redraw has
    /// returned and dropped its acquired SurfaceTexture. A veto or candidate
    /// failure converges on the existing bounded whole-run Closing policy with
    /// the original FrameRender as the first cause.
    pub(super) fn recover_frame_render_failure(
        &mut self,
        event_loop: &ActiveEventLoop,
        adapter: &GenericNativeAdapterOwner,
        failure: NativeFrameRenderFailure,
        kind: NativeRendererRecoveryWindowKind,
    ) -> Result<(), NativeGenericRunError> {
        let cause = failure.into_error();
        match self.try_recover_frame_render(adapter, kind) {
            Ok(()) => Ok(()),
            Err(reason) => {
                warn!(
                    reason,
                    "radiant generic native vello: renderer recovery was vetoed"
                );
                self.admit_native_shutdown(event_loop, Some(cause.clone()));
                Err(cause)
            }
        }
    }

    fn try_recover_frame_render(
        &mut self,
        adapter: &GenericNativeAdapterOwner,
        kind: NativeRendererRecoveryWindowKind,
    ) -> Result<(), String> {
        let active_generation = self
            .window
            .native_resources
            .as_ref()
            .map(|resources| resources.generation);
        let current_generation = adapter.capture_generation();
        let window_identity = self.window.window.as_ref().zip(self.window.id);
        let admission = preflight_renderer_recovery(
            &self.renderer_recovery,
            active_generation,
            current_generation,
            window_identity,
            self.window.can_publish_native_resources(),
            self.window.target_generation,
            self.is_running() && !self.has_terminal_cause(),
        )
        .map_err(|veto| format!("renderer recovery preflight vetoed: {veto:?}"))?;

        // This is deliberately before event-proxy lookup and all candidate GPU
        // construction. Candidate failure therefore consumes the generation's
        // one bounded allowance just like a successful candidate.
        self.renderer_recovery.record_attempt(admission.generation);
        let event_proxy = self
            .runtime_wakeup
            .event_loop_proxy()
            .ok_or_else(|| String::from("native event-loop proxy was not installed"))?;
        let candidate = construct_renderer_recovery_candidate(
            &self.options,
            adapter,
            &admission,
            event_proxy,
            kind,
        )
        .map_err(|error| error.to_string())?;

        if !renderer_recovery_commit_is_valid(
            &self.renderer_recovery,
            &admission,
            &candidate,
            NativeRendererRecoveryCommitFacts {
                active_generation: self
                    .window
                    .native_resources
                    .as_ref()
                    .map(|resources| resources.generation),
                current_generation: adapter.capture_generation(),
                current_window: self.window.window.as_ref().zip(self.window.id),
                publication_available: self.window.can_publish_native_resources(),
                target_generation: self.window.target_generation,
                run_admissible: self.is_running() && !self.has_terminal_cause(),
            },
        ) {
            return Err(String::from(
                "renderer recovery candidate failed final identity, generation, lifecycle, or publication validation",
            ));
        }
        let Some(publication) = self.window.reserve_native_resource_publication() else {
            return Err(String::from(
                "renderer recovery publication capacity changed before commit",
            ));
        };
        publication.publish(candidate.bundle);
        self.window.target_generation = admission.next_target_generation;
        self.window.native_surface_target_fenced = false;
        self.frame.invalidate_native_resources_for_recovery();
        self.rebuild_scene();
        self.timing.last_redraw = Instant::now();
        self.request_redraw_for_frame_work(FrameWork::RebuildScene {
            reason: FrameWorkReason::RuntimeSurfaceRepaint,
            mode: SceneRebuildMode::Immediate,
        });
        Ok(())
    }

    pub(super) fn should_admit_auxiliary_sync(&self) -> bool {
        self.is_running() && !self.has_terminal_cause()
    }

    pub(super) fn handle_device_lost_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        generation: NativeAdapterGeneration,
        registration: Arc<DeviceLossRegistration>,
        message: String,
    ) {
        if !self.is_running() {
            return;
        }
        if !self.device_loss_event_is_current(generation, &registration) {
            return;
        }
        let cause = NativeGenericRunError::RenderDeviceLost(message);
        self.begin_device_recovery(event_loop, generation, cause);
    }

    fn can_prepare_device_recovery(&self, generation: NativeAdapterGeneration) -> bool {
        self.window
            .native_resources
            .as_ref()
            .is_none_or(|resources| resources.generation == generation)
            && self.window.can_publish_native_resources()
            && self
                .auxiliary_windows
                .iter()
                .all(|window| window.can_prepare_device_recovery(generation))
    }

    fn begin_device_recovery(
        &mut self,
        event_loop: &ActiveEventLoop,
        generation: NativeAdapterGeneration,
        cause: NativeGenericRunError,
    ) {
        let Some(adapter) = self.adapter.as_ref() else {
            self.admit_native_shutdown(event_loop, Some(cause));
            return;
        };
        if adapter.capture_generation() != Some(generation)
            || !self.can_prepare_device_recovery(generation)
        {
            self.admit_native_shutdown(event_loop, Some(cause));
            return;
        }
        let Some(previous_device_identity) = adapter.selected_device_identity() else {
            self.admit_native_shutdown(event_loop, Some(cause));
            return;
        };
        let Some(next_generation) = adapter.next_recovery_generation() else {
            self.admit_native_shutdown(event_loop, Some(cause));
            return;
        };
        if !next_generation.is_strictly_newer_than(generation) {
            self.admit_native_shutdown(event_loop, Some(cause));
            return;
        }
        let Some(window) = self.window.window.clone() else {
            self.admit_native_shutdown(event_loop, Some(cause));
            return;
        };
        let Some(instance) = adapter.instance().cloned() else {
            self.admit_native_shutdown(event_loop, Some(cause));
            return;
        };
        let size = window.inner_size();
        let surface = match instance.create_surface(window.clone()) {
            Ok(surface) => surface,
            Err(error) => {
                warn!(error = %error, "radiant generic native vello: recovery surface creation failed");
                self.admit_native_shutdown(event_loop, Some(cause));
                return;
            }
        };
        let Some(event_proxy) = self.runtime_wakeup.event_loop_proxy() else {
            self.admit_native_shutdown(event_loop, Some(cause));
            return;
        };
        if !self.admit_device_recovery() {
            return;
        }
        #[cfg(target_os = "macos")]
        self.close_native_semantic_accessibility();
        self.recovery_cause = Some(cause);
        self.recovery_primary_was_visible = window.is_visible().unwrap_or(true);
        if self
            .auxiliary_windows
            .iter_mut()
            .any(|window| !window.admit_device_recovery())
        {
            self.admit_native_shutdown(event_loop, None);
            return;
        }
        let request = NativeRecoveryRequest {
            instance,
            surface,
            width: size.width.max(1),
            height: size.height.max(1),
            target_fps: self.options.normalized_target_fps(),
            generation: next_generation,
            previous_device_identity,
            event_proxy,
        };
        if let Err(error) = self.recovery.start(request) {
            warn!(error = %error, "radiant generic native vello: recovery candidate could not start");
            self.admit_native_shutdown(event_loop, None);
        }
    }

    pub(super) fn handle_device_recovery_ready(
        &mut self,
        event_loop: &ActiveEventLoop,
        episode: NativeRecoveryEpisodeToken,
    ) {
        if !self.is_recovering() {
            let _ = self.recovery.acknowledge(episode);
            return;
        }
        if !recovery_completion_is_admissible(self.recovery_expired(Instant::now())) {
            if self.recovery.acknowledge(episode) {
                self.admit_native_shutdown(event_loop, None);
            }
            return;
        }
        let Some(result) = self.recovery.take_ready(episode) else {
            return;
        };
        match result {
            Ok(candidate) => {
                if let Err(error) = self.commit_device_recovery_candidate(candidate) {
                    warn!(error = %error, "radiant generic native vello: recovery candidate publication failed");
                    self.admit_native_shutdown(event_loop, None);
                }
            }
            Err(error) => {
                warn!(error = %error, "radiant generic native vello: recovery candidate preparation failed");
                self.admit_native_shutdown(event_loop, None);
            }
        }
    }

    fn commit_device_recovery_candidate(
        &mut self,
        candidate: NativeRecoveryCandidate,
    ) -> Result<(), String> {
        if !self.is_recovering() {
            return Err(String::from(
                "native recovery lifecycle is no longer recovering",
            ));
        }
        let Some(previous_generation) = self
            .adapter
            .as_ref()
            .and_then(|adapter| adapter.capture_generation())
        else {
            return Err(String::from(
                "native recovery lost its previous adapter generation",
            ));
        };
        let NativeRecoveryCandidate {
            adapter,
            mut primary,
        } = candidate;
        if !primary
            .generation
            .is_strictly_newer_than(previous_generation)
            || adapter.capture_generation() != Some(primary.generation)
            || !self.can_prepare_device_recovery(previous_generation)
        {
            return Err(String::from(
                "native recovery candidate did not retain exact newer-generation evidence",
            ));
        }
        if let Some(window) = self.window.window.as_ref() {
            let size = window.inner_size();
            if !adapter.resize_surface(
                &mut primary.render_surface,
                size.width.max(1),
                size.height.max(1),
            ) {
                return Err(String::from(
                    "native recovery candidate could not match the current primary geometry",
                ));
            }
        } else {
            return Err(String::from("native recovery primary window disappeared"));
        }
        for window in &mut self.auxiliary_windows {
            if !window.quarantine_device_recovery_resources() {
                return Err(String::from(
                    "native recovery auxiliary quarantine capacity changed during commit",
                ));
            }
        }
        let Some(publication) = self.window.reserve_native_resource_publication() else {
            return Err(String::from(
                "native recovery primary quarantine capacity changed during commit",
            ));
        };
        publication.publish(primary);
        self.adapter = Some(adapter);
        self.complete_native_recovery_target_transition();
        self.frame.invalidate_native_resources_for_recovery();
        if !self.finish_device_recovery() {
            return Err(String::from(
                "native recovery lifecycle completion was vetoed",
            ));
        }
        #[cfg(target_os = "macos")]
        if let Some(proxy) = self.runtime_wakeup.event_loop_proxy() {
            self.attach_native_semantic_accessibility(proxy);
        }
        for window in &mut self.auxiliary_windows {
            if !window.finish_device_recovery_if_no_rebuild() {
                return Err(String::from(
                    "native recovery auxiliary lifecycle completion was vetoed",
                ));
            }
        }
        self.rebuild_scene();
        if self.recovery_primary_was_visible
            && let Some(window) = self.window.window.as_ref()
        {
            window.set_visible(true);
        }
        self.recovery_primary_was_visible = false;
        self.recovery_auxiliary_followup_pending = true;
        self.timing.deferred_auxiliary_window_sync = true;
        self.timing.last_redraw = Instant::now();
        self.request_redraw_for_frame_work(FrameWork::RebuildScene {
            reason: FrameWorkReason::RuntimeSurfaceRepaint,
            mode: SceneRebuildMode::Immediate,
        });
        Ok(())
    }

    pub(super) fn handle_render_device_error_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        generation: NativeAdapterGeneration,
        registration: Arc<DeviceLossRegistration>,
        kind: NativeRenderDeviceErrorKind,
        message: String,
    ) {
        if !self.is_running() {
            return;
        }
        if !self.device_loss_event_is_current(generation, &registration) {
            return;
        }
        let cause = NativeGenericRunError::RenderDeviceError { kind, message };
        self.record_render_device_error_and_exit(event_loop, cause);
    }

    pub(super) fn device_loss_event_is_current(
        &self,
        generation: NativeAdapterGeneration,
        registration: &Arc<DeviceLossRegistration>,
    ) -> bool {
        self.adapter
            .as_ref()
            .is_some_and(|adapter| adapter.accepts_device_loss(generation, registration))
    }

    pub(super) fn record_initialization_error_and_exit(
        &mut self,
        event_loop: &ActiveEventLoop,
        cause: NativeGenericRunError,
    ) {
        self.admit_native_shutdown(event_loop, Some(cause));
    }

    pub(super) fn record_render_device_error_and_exit(
        &mut self,
        event_loop: &ActiveEventLoop,
        cause: NativeGenericRunError,
    ) {
        self.admit_native_shutdown(event_loop, Some(cause));
    }

    pub(super) fn record_auxiliary_terminal_cause_and_exit(
        &mut self,
        event_loop: &ActiveEventLoop,
        cause: NativeGenericRunError,
    ) {
        self.admit_native_shutdown(event_loop, Some(cause));
    }

    pub(super) fn take_terminal_cause(&mut self) -> Option<NativeGenericRunError> {
        self.terminal_cause.take()
    }

    pub(super) fn resolve_run_result(
        &mut self,
        run_result: Result<(), NativeGenericRunError>,
    ) -> Result<(), NativeGenericRunError> {
        let Some(terminal_cause) = self.take_terminal_cause() else {
            return run_result;
        };
        if let Err(event_loop_error) = &run_result {
            warn!(
                terminal_cause = %terminal_cause,
                event_loop_error = %event_loop_error,
                "native terminal cause superseded the event-loop error"
            );
        }
        Err(terminal_cause)
    }

    pub(super) fn request_redraw_for_frame_work(&mut self, frame_work: FrameWork) {
        if !self.is_running() {
            return;
        }
        self.record_frame_work(frame_work);
        if self.window.native_resources.is_none() {
            return;
        }
        let now = Instant::now();
        if self.timing.redraw_requested && !self.pending_redraw_request_is_stale(now) {
            return;
        }
        if let Some(window) = self.window.window.as_ref() {
            if self.timing.redraw_requested
                && let Some(requested_at) = self.timing.redraw_requested_at
            {
                let pending = now.duration_since(requested_at);
                if slow_render_profile_enabled() && pending >= Self::REDRAW_REISSUE_LOG_AFTER {
                    warn!(
                        target: "radiant::debug::frame_profile",
                        event = "radiant.redraw_request.reissued",
                        pending_us = pending.as_micros(),
                        stale_after_us = Self::REDRAW_REISSUE_AFTER.as_micros(),
                        "Reissued stale redraw request"
                    );
                }
            }
            window.request_redraw();
            self.timing.redraw_requested = true;
            self.timing.redraw_requested_at = Some(now);
        }
    }

    pub(super) fn record_frame_work(&mut self, frame_work: FrameWork) {
        if !self.frame_observation_enabled {
            return;
        }
        self.timing.pending_frame_work = self.timing.pending_frame_work.merge(frame_work);
    }

    pub(super) fn record_native_interactive_arrival(&mut self, arrived_at: Instant) {
        self.timing.record_native_interactive_arrival_if_enabled(
            self.frame_observation_enabled,
            arrived_at,
        );
    }

    pub(super) fn take_pending_frame_work(&mut self) -> FrameWork {
        if !self.frame_observation_enabled {
            return FrameWork::None;
        }
        let frame_work = self.timing.pending_frame_work;
        self.timing.pending_frame_work = FrameWork::None;
        frame_work
    }

    pub(super) fn pending_redraw_request_is_stale(&self, now: Instant) -> bool {
        self.timing.redraw_requested_at.is_none_or(|requested_at| {
            now.duration_since(requested_at) >= Self::REDRAW_REISSUE_AFTER
        })
    }

    pub(super) fn should_defer_timed_frame_drain_for_pending_redraw(&self, now: Instant) -> bool {
        self.timing.redraw_requested && !self.pending_redraw_request_is_stale(now)
    }

    pub(super) fn pending_redraw_retry_deadline(&self) -> Option<Instant> {
        if !self.timing.redraw_requested {
            return None;
        }
        self.timing
            .redraw_requested_at
            .and_then(|requested_at| requested_at.checked_add(Self::REDRAW_REISSUE_AFTER))
    }

    pub(super) fn frame_wait_deadline(&self, scheduled: Instant) -> Instant {
        self.pending_redraw_retry_deadline()
            .map_or(scheduled, |deadline| scheduled.min(deadline))
    }

    pub(super) fn pending_redraw_elapsed(&self, now: Instant) -> Option<Duration> {
        if !self.timing.redraw_requested {
            return None;
        }
        let requested_at = self.timing.redraw_requested_at?;
        Some(now.duration_since(requested_at))
    }

    pub(super) fn pending_redraw_age(&self, now: Instant) -> CpuFramePendingRedrawAge {
        if !self.timing.redraw_requested {
            CpuFramePendingRedrawAge::NotRequested
        } else {
            self.timing
                .redraw_requested_at
                .map(|requested_at| {
                    CpuFramePendingRedrawAge::Known(now.saturating_duration_since(requested_at))
                })
                .unwrap_or(CpuFramePendingRedrawAge::Unknown)
        }
    }

    pub(super) fn pending_interactive_scroll_flush_is_due(&self, now: Instant) -> bool {
        self.timing.redraw_requested && self.pending_redraw_request_is_stale(now)
    }

    pub(super) fn should_flush_pending_redraw_after_route(
        &self,
        pending: Duration,
        _since_last_present: Duration,
    ) -> bool {
        pending >= Self::REDRAW_REISSUE_AFTER
    }

    fn should_log_pending_redraw_route_flush(
        &self,
        pending: Duration,
        since_last_present: Duration,
    ) -> bool {
        slow_render_profile_enabled()
            && (pending >= Self::REDRAW_REISSUE_LOG_AFTER
                || since_last_present >= Self::REDRAW_REISSUE_LOG_AFTER)
    }

    pub(super) fn drain_timed_frame_now(
        &mut self,
        now: Instant,
        animation_activity: RuntimeAnimationActivity,
        needs_text_caret_animation: bool,
    ) -> GenericRouteOutcome {
        if !self.is_running() {
            return GenericRouteOutcome::default();
        }
        self.timing.last_timed_frame_drain = now;
        self.core
            .drain_timed_frame(animation_activity, needs_text_caret_animation)
    }

    pub(super) fn merge_due_timed_frame_for_route(&mut self, outcome: &mut GenericRouteOutcome) {
        if !self.is_running() {
            return;
        }
        let now = Instant::now();
        let native_target_fps = self.options.normalized_target_fps();
        let native_frame_interval = animation_frame_interval_for_normalized_fps(native_target_fps);
        if now.duration_since(self.timing.last_timed_frame_drain) < native_frame_interval {
            return;
        }
        if self.should_defer_timed_frame_drain_for_pending_redraw(now) {
            return;
        }
        let animation_activity = self.core.animation_activity();
        let needs_text_caret_animation = self.core.has_focused_text_input();
        if !animation_activity.needs_animation() && !needs_text_caret_animation {
            return;
        }
        let frame_target_fps = timed_frame_target_fps(
            native_target_fps,
            animation_activity,
            needs_text_caret_animation,
        );
        let cadence = timed_frame_cadence(
            now,
            self.timing.last_timed_frame_drain,
            frame_target_fps,
            true,
        );
        if !matches!(cadence, TimedFrameCadence::DrainNow { .. }) {
            return;
        }
        outcome.merge(self.drain_timed_frame_now(
            now,
            animation_activity,
            needs_text_caret_animation,
        ));
    }

    pub(super) fn request_runtime_wakeup_if_needed(&self, outcome: GenericRouteOutcome) {
        if !self.is_running() {
            return;
        }
        if self.core.runtime.interactive_pointer_route_active() {
            return;
        }
        self.runtime_wakeup
            .request_if(outcome.runtime_work_remaining);
    }

    pub(super) fn rebuild_scene(&mut self) {
        self.rebuild_scene_with_refresh_evidence(false);
    }

    pub(super) fn rebuild_scene_after_surface_refresh(&mut self) {
        self.rebuild_scene_with_refresh_evidence(true);
    }

    fn rebuild_scene_with_refresh_evidence(&mut self, freshly_refreshed: bool) {
        self.timing.deferred_scene_rebuild = false;
        self.timing.deferred_scene_rebuild_requires_encode = false;
        self.frame.reset_scene_build_outcome();
        let _ = self.apply_pending_viewport_resize_if_needed();
        let paint_plan_decision = self.core.paint_plan_into(&mut self.frame.last_paint_plan);
        self.publish_native_ime_cursor_area();
        let viewport = self.core.runtime.viewport();
        let scene_validity = self.frame.native_scene_validity_fingerprint(
            self.core.base_paint_plan_context(),
            self.core.resolved_appearance(),
            self.window.dpi_scale,
        );
        if freshly_refreshed
            && matches!(paint_plan_decision, PaintPlanCacheDecision::Reused)
            && self.frame.can_reuse_native_scene(scene_validity)
            && !self.timing.surface_resize_applied_this_frame
        {
            // The scene remains valid, but this refresh still has visible work:
            // make the cached base texture/composited frame available for the
            // presentation pass and continue transient/native overlays below.
            self.frame.mark_scene_texture_dirty();
            self.frame.record_scene_reuse();
            self.restore_native_hover_cursor_overlay();
            self.export_automation_targets();
            return;
        }
        #[cfg(test)]
        self.frame.begin_test_phase_trace();
        let retained_surface = self.core.runtime.retained_surface_capability();
        let paint = self.core.paint_segment_observation();
        self.frame.observe_native_paint_segment_eligibility(
            paint,
            self.frame.last_scene_stats.artifact_feasibility,
            self.window.target_generation,
        );
        self.frame.derive_native_paint_segment_render_selection(
            scene_validity,
            self.window.target_generation,
        );
        let render_selection = self.frame.native_paint_segment_render_selection();
        let assembly_attempted = render_selection.should_attempt_mixed_assembly();
        let mut assembly_vetoed = false;
        if assembly_attempted {
            match self.frame.assemble_mixed_native_scene(
                viewport,
                paint,
                scene_validity,
                self.window.target_generation,
                render_selection.full_encode_plan(),
            ) {
                Ok(bundle) => {
                    if self
                        .frame
                        .commit_native_scene_assembly(bundle, scene_validity)
                        .is_err()
                    {
                        assembly_vetoed = true;
                    } else {
                        self.frame.refresh_gpu_surface_interaction_regions();
                        self.frame.refresh_post_gpu_overlay_cache();
                        self.restore_native_hover_cursor_overlay();
                        self.frame.mark_scene_content_dirty();
                        self.export_automation_targets();
                        return;
                    }
                }
                Err(_) => {
                    assembly_vetoed = true;
                }
            }
        }
        // Any attempted assembly that did not return above falls through to
        // the existing authoritative encoder for conservative repair.
        #[cfg(test)]
        self.frame.record_scene_encode_boundary();
        self.frame.last_scene_stats = encode_surface_paint_plan_to_scene(
            &self.frame.last_paint_plan,
            SurfaceSceneEncodeContext {
                scene: &mut self.frame.scene,
                text_renderer: &mut self.frame.text_renderer,
                bridge: self.core.runtime.bridge_mut(),
                retained_surface,
                viewport,
                retained_cache: &mut self.frame.retained_surface_cache,
                text_runs: &mut self.frame.scene_text_runs,
                animation_time: self.timing.animation_origin.elapsed(),
            },
        );
        let eligibility = render_selection.full_encode_plan();
        let payloads = encode_native_paint_segment_payloads(
            &self.frame.last_paint_plan.primitives,
            viewport,
            paint,
            eligibility,
            scene_validity,
            self.window.target_generation,
            &self.frame.native_paint_segment_artifact_store,
        )
        .into_parts()
        .0;
        let materialization =
            materialize_native_paint_segment_artifacts(NativePaintSegmentArtifactAdmission {
                scene: &self.frame.scene,
                feasibility: self.frame.last_scene_stats.artifact_feasibility,
                plan: eligibility,
                payloads,
                scene_validity,
                target_generation: self.window.target_generation,
            });
        self.frame
            .reconcile_native_paint_segment_artifacts(materialization);
        self.frame.reconcile_native_paint_segments(
            paint,
            self.frame.last_scene_stats.segment_encoding,
            self.window.target_generation,
        );
        if assembly_vetoed {
            self.frame
                .record_scene_encode_after_assembly_veto(scene_validity);
        } else {
            self.frame.record_scene_encode(scene_validity);
        }
        self.frame.record_native_paint_segment_full_encode(
            paint,
            self.frame.last_scene_stats.segment_encoding,
            self.frame.last_scene_stats.artifact_feasibility,
            self.window.target_generation,
            assembly_vetoed,
        );
        self.frame.refresh_gpu_surface_interaction_regions();
        self.frame.refresh_post_gpu_overlay_cache();
        self.restore_native_hover_cursor_overlay();
        self.frame.mark_scene_content_dirty();
        self.export_automation_targets();
    }

    pub(super) fn export_automation_targets(&mut self) {
        #[cfg(target_os = "macos")]
        self.republish_native_semantic_accessibility_passively();
        let snapshot = self.core.runtime.automation_target_snapshot();
        match self.automation_targets.export(&snapshot) {
            Ok(true) => {
                if let Some(path) = self.automation_targets.path() {
                    info!(
                        "radiant generic native vello: exported automation target snapshot to {}",
                        path.display()
                    );
                }
            }
            Ok(false) => {}
            Err(err) => {
                if self.automation_targets.has_warned_after_failure() {
                    return;
                }
                self.automation_targets.mark_warned_after_failure();
                if let Some(path) = err.path() {
                    warn!(
                        "radiant generic native vello: failed to export automation target snapshot to {}: {}",
                        path.display(),
                        err
                    );
                } else {
                    warn!(
                        "radiant generic native vello: failed to export automation target snapshot: {}",
                        err
                    );
                }
            }
        }
    }

    pub(super) fn rebuild_scene_for_interactive_route_now(&mut self) {
        self.timing.deferred_scene_rebuild = false;
        self.timing.last_interactive_scene_rebuild = Instant::now();
        self.rebuild_scene();
    }

    pub(super) fn rebuild_scene_for_interactive_route_now_after_surface_refresh(&mut self) {
        self.timing.deferred_scene_rebuild = false;
        self.timing.last_interactive_scene_rebuild = Instant::now();
        self.rebuild_scene_after_surface_refresh();
    }

    pub(super) fn refresh_and_rebuild_scene_now_with_scope(
        &mut self,
        scope: crate::runtime::RepaintScope,
    ) {
        let scope = self
            .take_deferred_surface_refresh_scope()
            .map_or(scope, |pending| pending.merge(scope));
        self.core.refresh_surface_with_scope(scope);
        self.rebuild_scene_after_surface_refresh();
    }

    pub(super) fn refresh_and_rebuild_scene_for_interactive_route_now_with_scope(
        &mut self,
        scope: crate::runtime::RepaintScope,
    ) {
        let scope = self
            .take_deferred_surface_refresh_scope()
            .map_or(scope, |pending| pending.merge(scope));
        self.core.refresh_surface_with_scope(scope);
        self.rebuild_scene_for_interactive_route_now_after_surface_refresh();
    }

    pub(super) fn defer_surface_refresh_with_scope(&mut self, scope: crate::runtime::RepaintScope) {
        self.timing.deferred_surface_refresh = true;
        self.timing.deferred_surface_refresh_scope = Some(
            self.timing
                .deferred_surface_refresh_scope
                .map_or(scope, |pending| pending.merge(scope)),
        );
    }

    pub(super) fn take_deferred_surface_refresh_scope(
        &mut self,
    ) -> Option<crate::runtime::RepaintScope> {
        if !self.timing.deferred_surface_refresh {
            return None;
        }
        self.timing.deferred_surface_refresh = false;
        Some(
            self.timing
                .deferred_surface_refresh_scope
                .take()
                .unwrap_or(crate::runtime::RepaintScope::Surface),
        )
    }

    pub(super) fn should_rebuild_interactive_scene_now(&self, now: Instant) -> bool {
        let interval = animation_frame_interval(self.options.normalized_target_fps());
        now.duration_since(self.timing.last_interactive_scene_rebuild) >= interval
    }

    pub(super) fn defer_scene_rebuild(&mut self) {
        self.timing.deferred_scene_rebuild = true;
        self.timing.deferred_scene_rebuild_requires_encode = true;
    }

    #[cfg(test)]
    pub(super) fn defer_viewport_resize(&mut self, viewport: Vector2) {
        self.defer_viewport_resize_with_reason(viewport, FrameWorkReason::NativeResize);
    }

    pub(super) fn defer_viewport_resize_with_reason(
        &mut self,
        viewport: Vector2,
        reason: FrameWorkReason,
    ) {
        self.timing.pending_viewport_resize = Some(viewport);
        self.timing.pending_viewport_resize_reason = Some(reason);
        self.timing.deferred_scene_rebuild = true;
    }

    pub(super) fn apply_pending_viewport_resize_if_needed(&mut self) -> Option<bool> {
        let viewport = self.timing.pending_viewport_resize.take()?;
        let reason = self
            .timing
            .pending_viewport_resize_reason
            .take()
            .unwrap_or(FrameWorkReason::NativeResize);
        let relayout = self.core.set_viewport(viewport);
        if relayout {
            self.record_frame_work(FrameWork::ResizeAndRebuild { reason });
        }
        Some(relayout)
    }

    pub(super) fn defer_interactive_scene_rebuild(&mut self) {
        self.defer_surface_refresh_with_scope(crate::runtime::RepaintScope::Surface);
        self.defer_scene_rebuild();
    }

    pub(super) fn defer_interactive_scene_rebuild_with_scope(
        &mut self,
        scope: crate::runtime::RepaintScope,
    ) {
        self.defer_surface_refresh_with_scope(scope);
        self.defer_scene_rebuild();
    }

    pub(super) fn queue_window_environment_change(
        &mut self,
        change: crate::runtime::WindowEnvironmentChange,
    ) {
        self.queue_window_environment_change_with_reason(
            change,
            FrameWorkReason::NativeWindowEnvironment,
        );
    }

    pub(super) fn queue_window_environment_change_with_reason(
        &mut self,
        change: crate::runtime::WindowEnvironmentChange,
        reason: FrameWorkReason,
    ) {
        self.defer_interactive_scene_rebuild_with_scope(change.repaint_scope());
        self.request_redraw_for_frame_work(FrameWork::RebuildScene {
            reason,
            mode: SceneRebuildMode::Interactive,
        });
    }

    pub(super) fn update_window_environment(
        &mut self,
        environment: crate::runtime::WindowEnvironment,
    ) -> bool {
        if self.window.environment == environment {
            return false;
        }
        self.window.environment = environment;
        self.core.runtime.set_window_environment(environment)
    }

    pub(super) fn observe_monitor_move(&mut self) {
        let Some(window) = self.window.window.as_ref() else {
            return;
        };
        let Some(next) = super::window_environment::current_monitor_fingerprint(window) else {
            return;
        };
        if self.window.monitor_fingerprint.as_ref() == Some(&next) {
            return;
        }
        self.window.monitor_fingerprint = Some(next);
        self.queue_window_environment_change(
            crate::runtime::WindowEnvironmentChange::DisplayScaleOrMonitor,
        );
    }

    pub(super) fn observe_theme_change(&mut self, theme: Option<winit::window::Theme>) {
        let environment = super::window_environment::environment_for_native_state(
            self.window.dpi_scale,
            super::window_environment::window_color_scheme(theme),
            self.window.accessibility_display,
        );
        if self.update_window_environment(environment) {
            self.queue_window_environment_change(
                crate::runtime::WindowEnvironmentChange::ColorSchemeOrContrast,
            );
        }
    }

    fn restore_native_hover_cursor_overlay(&mut self) {
        let Some(position) = self.input.last_cursor else {
            return;
        };
        if self.can_fast_path_native_hover_move(position) {
            self.update_gpu_surface_cursor_overlay(position);
        }
    }

    pub(super) fn handle_route_outcome(
        &mut self,
        event_loop: &ActiveEventLoop,
        outcome: GenericRouteOutcome,
    ) {
        self.handle_route_outcome_inner(event_loop, outcome, None, None, true, true);
    }

    pub(super) fn sync_native_ime_allowed(&self) {
        if let Some(window) = self.window.window.as_ref() {
            window.set_ime_allowed(self.core.has_focused_text_input());
        }
    }

    pub(super) fn publish_native_ime_cursor_area(&mut self) {
        let candidate = self.frame.native_ime_cursor_area();
        let Some(window) = self.window.window.as_ref().cloned() else {
            self.window.ime_cursor_area_cache.invalidate();
            return;
        };
        let window_id = window.id();
        let native_scale_generation = self.window.target_generation;
        let native_dpi_scale = self.window.native_dpi_scale;
        let Some(area) = self.window.ime_cursor_area_cache.candidate_to_publish(
            window_id,
            native_scale_generation,
            native_dpi_scale,
            candidate,
        ) else {
            return;
        };
        window.set_ime_cursor_area(
            LogicalPosition::new(area.min.x as f64, area.min.y as f64),
            LogicalSize::new(area.width() as f64, area.height() as f64),
        );
        self.window.ime_cursor_area_cache.record(
            window_id,
            native_scale_generation,
            native_dpi_scale,
            area,
        );
    }

    pub(super) fn handle_route_outcome_without_timed_frame(
        &mut self,
        event_loop: &ActiveEventLoop,
        outcome: GenericRouteOutcome,
    ) {
        self.handle_route_outcome_inner(event_loop, outcome, None, None, false, true);
    }

    pub(super) fn handle_route_outcome_with_adapter(
        &mut self,
        event_loop: &ActiveEventLoop,
        outcome: GenericRouteOutcome,
        adapter: &mut GenericNativeAdapterOwner,
        observation: Option<&mut CpuFrameObservationOwner<'_>>,
    ) {
        self.handle_route_outcome_inner(
            event_loop,
            outcome,
            Some(adapter),
            observation,
            true,
            false,
        );
    }

    pub(super) fn handle_route_outcome_deferred_publication(
        &mut self,
        event_loop: &ActiveEventLoop,
        outcome: GenericRouteOutcome,
    ) {
        self.handle_route_outcome_inner(event_loop, outcome, None, None, false, false);
    }

    fn handle_route_outcome_inner(
        &mut self,
        event_loop: &ActiveEventLoop,
        outcome: GenericRouteOutcome,
        adapter: Option<&mut GenericNativeAdapterOwner>,
        observation: Option<&mut CpuFrameObservationOwner<'_>>,
        merge_due_timed_frame: bool,
        publish_frame_diagnostics: bool,
    ) {
        if !self.is_running() {
            return;
        }
        let pending_redraw_at_route_start = self.pending_redraw_elapsed(Instant::now());
        let applied = if merge_due_timed_frame {
            self.apply_route_outcome(outcome)
        } else {
            self.apply_route_outcome_with_timed_frame(outcome, false)
        };
        if applied.exit_requested {
            self.admit_native_shutdown(event_loop, None);
            return;
        }
        self.sync_native_ime_allowed();
        if applied.sync_auxiliary_windows_now
            && adapter.is_none()
            && let Some(event_proxy) = self.runtime_wakeup.event_loop_proxy()
            && self
                .sync_auxiliary_windows(event_loop, event_proxy)
                .is_err()
        {
            return;
        }
        if let Some(pending) = pending_redraw_at_route_start
            && self.timing.redraw_requested
        {
            let since_last_present = Instant::now().duration_since(self.timing.last_redraw);
            if self.should_flush_pending_redraw_after_route(pending, since_last_present) {
                if self.should_log_pending_redraw_route_flush(pending, since_last_present) {
                    warn!(
                        target: "radiant::debug::frame_profile",
                        event = "radiant.redraw_request.flushed_pending",
                        pending_us = pending.as_micros(),
                        since_last_present_us = since_last_present.as_micros(),
                        stale = pending >= Self::REDRAW_REISSUE_AFTER,
                        "Flushed pending redraw request after route"
                    );
                }
                match adapter {
                    Some(adapter) => {
                        self.redraw_and_exit_on_error_with_adapter(event_loop, adapter, observation)
                    }
                    None => self.redraw_and_exit_on_error(event_loop),
                }
            }
        }
        if publish_frame_diagnostics {
            self.publish_staged_frame_diagnostics();
        }
    }

    pub(super) fn apply_route_outcome(
        &mut self,
        outcome: GenericRouteOutcome,
    ) -> AppliedRouteOutcome {
        self.apply_route_outcome_with_timed_frame(outcome, true)
    }

    pub(super) fn apply_route_outcome_with_timed_frame(
        &mut self,
        mut outcome: GenericRouteOutcome,
        merge_due_timed_frame: bool,
    ) -> AppliedRouteOutcome {
        if !self.is_running() {
            return AppliedRouteOutcome::default();
        }
        if outcome.exit_requested {
            return AppliedRouteOutcome {
                exit_requested: true,
                sync_auxiliary_windows_now: false,
            };
        }
        if merge_due_timed_frame {
            self.merge_due_timed_frame_for_route(&mut outcome);
        }
        if let Some(scale) = outcome.dpi_scale_override {
            self.set_dpi_scale_override(scale);
        }
        if let Some(size) = outcome.window_logical_size {
            self.set_window_logical_size(size);
        }
        let mut sync_auxiliary_windows_now = false;
        match outcome.frame_work() {
            FrameWork::None
            | FrameWork::PaintOnly { .. }
            | FrameWork::ResizeSurface { .. }
            | FrameWork::Exit { .. } => {}
            FrameWork::RefreshSurface { .. } => {
                self.defer_surface_refresh_with_scope(outcome.surface_refresh_scope_or_surface());
            }
            FrameWork::ResizeAndRebuild { .. } => {
                self.rebuild_scene();
                sync_auxiliary_windows_now = true;
            }
            FrameWork::RebuildScene { mode, .. } => match mode {
                SceneRebuildMode::InteractiveWithSurfaceRefresh => {
                    self.refresh_and_rebuild_scene_for_interactive_route_now_with_scope(
                        outcome.surface_refresh_scope_or_surface(),
                    );
                    self.defer_auxiliary_window_sync();
                }
                SceneRebuildMode::ImmediateWithSurfaceRefresh => {
                    self.refresh_and_rebuild_scene_now_with_scope(
                        outcome.surface_refresh_scope_or_surface(),
                    );
                    sync_auxiliary_windows_now = true;
                }
                SceneRebuildMode::Interactive => {
                    let now = Instant::now();
                    if self.should_rebuild_interactive_scene_now(now) {
                        self.rebuild_scene_for_interactive_route_now();
                        self.defer_auxiliary_window_sync();
                    } else {
                        self.defer_interactive_scene_rebuild();
                        self.defer_auxiliary_window_sync();
                    }
                }
                SceneRebuildMode::Immediate => {
                    self.rebuild_scene();
                    sync_auxiliary_windows_now = true;
                }
            },
        }
        if outcome.needs_redraw() {
            self.request_redraw_for_frame_work(outcome.frame_work());
        }
        self.request_runtime_wakeup_if_needed(outcome);
        AppliedRouteOutcome {
            exit_requested: false,
            sync_auxiliary_windows_now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        FrameScheduleDeadlines, FrameScheduleDemand, FrameScheduleRedrawEvidence,
        assess_cpu_frame_fairness,
    };
    use super::{
        FrameScheduleKey, FrameWork, FrameWorkReason, GenericNativeVelloRunner, NativeLifecycle,
        TimedFrameCadence, recovery_completion_is_admissible,
    };
    use crate::{
        application::empty,
        gui::types::Vector2,
        gui_runtime::NativeRunOptions,
        prelude::IntoView,
        runtime::{
            FrameProfile, NativeCpuFrameCompletionOutcome, NativeCpuFrameFairnessDiagnostics,
            NativeCpuFrameFairnessDisposition, NativeCpuFrameObservationDiagnostics,
            NativeFrameDiagnostics, NativeWindowDiagnosticIdentity, ProfilingOptions,
            RuntimeAnimationActivity, RuntimeBridge, RuntimeFrameDiagnosticsHost,
            RuntimeFrameProfileHost, RuntimeHostCapabilities, UiSurface,
        },
    };
    use std::{
        sync::{Arc, Mutex},
        time::Instant,
    };

    struct EmptyBridge;

    impl RuntimeBridge<()> for EmptyBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(empty::<()>().into_surface())
        }
    }

    type PublishedFrameEvents = Arc<Mutex<Vec<NativeFrameDiagnostics>>>;

    struct RecordingFrameDiagnosticsBridge {
        published: PublishedFrameEvents,
    }

    impl RuntimeBridge<()> for RecordingFrameDiagnosticsBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(empty::<()>().into_surface())
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
            RuntimeHostCapabilities::new().with_frame_diagnostics()
        }
    }

    impl RuntimeFrameDiagnosticsHost for RecordingFrameDiagnosticsBridge {
        fn observe_frame_diagnostics(&mut self, diagnostics: NativeFrameDiagnostics) {
            self.published
                .lock()
                .expect("publication test events should not be poisoned")
                .push(diagnostics);
        }
    }

    type PublishedFrameProfiles = Arc<Mutex<Vec<FrameProfile>>>;

    struct RecordingFrameProfileBridge {
        published: PublishedFrameProfiles,
    }

    impl RuntimeBridge<()> for RecordingFrameProfileBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(empty::<()>().into_surface())
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
            RuntimeHostCapabilities::new().with_frame_profile()
        }
    }

    impl RuntimeFrameProfileHost for RecordingFrameProfileBridge {
        fn observe_frame_profile(&mut self, profile: FrameProfile) {
            self.published
                .lock()
                .expect("profile publication test events should not be poisoned")
                .push(profile);
        }
    }

    fn staged_diagnostics() -> NativeFrameDiagnostics {
        NativeFrameDiagnostics {
            window_identity: Some(NativeWindowDiagnosticIdentity::from_runtime_value(1)),
            frame_sequence: Some(7),
            ..NativeFrameDiagnostics::default()
        }
    }

    fn primary_publication_for_boundary(scheduled: bool) {
        let published = Arc::new(Mutex::new(Vec::new()));
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            RecordingFrameDiagnosticsBridge {
                published: Arc::clone(&published),
            },
            Vector2::new(320.0, 240.0),
        );
        let diagnostics = staged_diagnostics();

        if scheduled {
            runner.require_primary_frame_diagnostics_schedule_admission();
        }
        runner.stage_frame_diagnostics(diagnostics);
        assert!(
            published
                .lock()
                .expect("publication test events should not be poisoned")
                .is_empty()
        );
        runner
            .frame_diagnostics_publication
            .mark_observation_finalized();
        if scheduled {
            runner.publish_staged_frame_diagnostics();
            assert!(
                published
                    .lock()
                    .expect("publication test events should not be poisoned")
                    .is_empty()
            );
        }
        let now = Instant::now();
        let primary_key = FrameScheduleKey::Primary;
        let demand = FrameScheduleDemand::from_cadence_with_requested_target_fps(
            primary_key.clone(),
            TimedFrameCadence::DrainNow {
                due_at: now - std::time::Duration::from_millis(5),
                next_wake: now + std::time::Duration::from_millis(16),
            },
            120,
            24,
            RuntimeAnimationActivity::paint_only_at(24),
            false,
            FrameScheduleRedrawEvidence::default(),
        );
        let demands = [demand];
        let plan = runner
            .frame_scheduler
            .observe(now, &demands, FrameScheduleDeadlines::default());
        assert_eq!(plan.selected, Some(primary_key.clone()));
        assess_cpu_frame_fairness(now, &demands, None)
            .record_turn(runner.cpu_frame_fairness.as_mut().unwrap(), &plan);
        if scheduled {
            runner.record_frame_schedule_admission(primary_key);
        }
        runner.publish_staged_frame_diagnostics();
        runner.publish_staged_frame_diagnostics();

        let fairness = NativeCpuFrameFairnessDiagnostics {
            available: true,
            latest_disposition: NativeCpuFrameFairnessDisposition::Selected,
            requested_target_fps: 120,
            effective_target_fps: 24,
            latest_due_lateness_us: Some(5_000),
            selected_turns: 1,
            cursor_admissions: u64::from(scheduled),
            latest_selected_was_admitted: scheduled,
            ..NativeCpuFrameFairnessDiagnostics::default()
        };
        let expected = NativeFrameDiagnostics {
            cpu_fairness: fairness,
            ..diagnostics
        };
        assert_eq!(
            *published
                .lock()
                .expect("publication test events should not be poisoned"),
            vec![expected]
        );
    }

    #[test]
    fn primary_direct_redraw_publishes_once_after_staging() {
        primary_publication_for_boundary(false);
    }

    #[test]
    fn primary_route_time_flush_publishes_once_after_staging() {
        primary_publication_for_boundary(false);
    }

    #[test]
    fn primary_scheduled_route_time_flush_publishes_after_admission_record() {
        primary_publication_for_boundary(true);
    }

    #[test]
    fn diagnostics_disabled_staging_does_not_create_publication_state() {
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            EmptyBridge,
            Vector2::new(320.0, 240.0),
        );

        runner.stage_frame_diagnostics(staged_diagnostics());
        runner.publish_staged_frame_diagnostics();

        assert!(!runner.frame_diagnostics_enabled);
        assert!(runner.cpu_frame_observation.is_none());
        assert_eq!(runner.frame_diagnostics_publication.take(), None);
    }

    #[test]
    fn profiling_off_suppresses_profile_publication() {
        let published = Arc::new(Mutex::new(Vec::new()));
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            RecordingFrameProfileBridge {
                published: Arc::clone(&published),
            },
            Vector2::new(320.0, 240.0),
        );

        runner.stage_frame_diagnostics(staged_diagnostics());
        runner
            .frame_diagnostics_publication
            .mark_observation_finalized();
        runner.publish_staged_frame_diagnostics();

        assert!(!runner.frame_profile_enabled);
        assert!(!runner.frame_observation_enabled);
        assert!(
            published
                .lock()
                .expect("profile publication test events should not be poisoned")
                .is_empty()
        );
    }

    #[test]
    fn frame_profiling_delivers_successful_present_profiles_even_without_sequence() {
        let published = Arc::new(Mutex::new(Vec::new()));
        let mut options = NativeRunOptions::default();
        options.frame.profiling = ProfilingOptions::frame();
        let mut runner = GenericNativeVelloRunner::new(
            options,
            RecordingFrameProfileBridge {
                published: Arc::clone(&published),
            },
            Vector2::new(320.0, 240.0),
        );
        let diagnostics = staged_diagnostics();

        runner.stage_frame_diagnostics(diagnostics);
        runner
            .frame_diagnostics_publication
            .mark_observation_finalized();
        runner.publish_staged_frame_diagnostics();

        assert!(runner.frame_profile_enabled);
        assert_eq!(
            *published
                .lock()
                .expect("profile publication test events should not be poisoned"),
            vec![FrameProfile::from(diagnostics)]
        );

        let mut runner = GenericNativeVelloRunner::new(
            {
                let mut options = NativeRunOptions::default();
                options.frame.profiling = ProfilingOptions::frame();
                options
            },
            RecordingFrameProfileBridge {
                published: Arc::clone(&published),
            },
            Vector2::new(320.0, 240.0),
        );
        runner.stage_frame_diagnostics(NativeFrameDiagnostics::default());
        runner
            .frame_diagnostics_publication
            .mark_observation_finalized();
        runner.publish_staged_frame_diagnostics();

        let published = published
            .lock()
            .expect("profile publication test events should not be poisoned");
        assert_eq!(published.len(), 2);
        assert_eq!(published[1].frame_sequence, None);
    }

    #[test]
    fn primary_publication_projects_finalized_cpu_observation() {
        let published = Arc::new(Mutex::new(Vec::new()));
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            RecordingFrameDiagnosticsBridge {
                published: Arc::clone(&published),
            },
            Vector2::new(320.0, 240.0),
        );
        let admission = runner
            .begin_cpu_frame_observation(FrameScheduleKey::Primary, Instant::now())
            .expect("enabled diagnostics should retain the primary observation ledger");
        runner
            .cpu_frame_observation_capture
            .record_frame_work(FrameWork::PaintOnly {
                reason: FrameWorkReason::RoutedInput,
            });
        runner
            .cpu_frame_observation_capture
            .mark_successful_presentation();
        let diagnostics = staged_diagnostics();

        runner.stage_frame_diagnostics(diagnostics);
        runner.finish_cpu_frame_observation(Some(admission), false);
        runner.publish_staged_frame_diagnostics();

        assert_eq!(
            *published
                .lock()
                .expect("publication test events should not be poisoned"),
            vec![NativeFrameDiagnostics {
                cpu_observation: NativeCpuFrameObservationDiagnostics {
                    available: true,
                    latest_outcome: NativeCpuFrameCompletionOutcome::SuccessfulPresentation,
                    latest_exact_interaction: true,
                    admitted_redraws: 1,
                    successful_presentations: 1,
                    ..NativeCpuFrameObservationDiagnostics::default()
                },
                ..diagnostics
            }]
        );
    }

    fn runner() -> GenericNativeVelloRunner<EmptyBridge, ()> {
        GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            EmptyBridge,
            Vector2::new(320.0, 240.0),
        )
    }

    #[test]
    fn parent_admission_boundary_marks_fairness_before_cursor_progresses() {
        let mut runner = runner();
        let now = Instant::now();
        let primary_key = FrameScheduleKey::Primary;
        let auxiliary_key = FrameScheduleKey::Auxiliary("settings".to_owned());
        let demands = [
            FrameScheduleDemand::from_cadence(
                primary_key.clone(),
                TimedFrameCadence::DrainNow {
                    due_at: now,
                    next_wake: now + std::time::Duration::from_millis(16),
                },
                60,
                RuntimeAnimationActivity::paint_only(),
                false,
                FrameScheduleRedrawEvidence::default(),
            ),
            FrameScheduleDemand::from_cadence(
                auxiliary_key.clone(),
                TimedFrameCadence::DrainNow {
                    due_at: now,
                    next_wake: now + std::time::Duration::from_millis(16),
                },
                60,
                RuntimeAnimationActivity::paint_only(),
                false,
                FrameScheduleRedrawEvidence::default(),
            ),
        ];
        let plan = runner
            .frame_scheduler
            .observe(now, &demands, FrameScheduleDeadlines::default());
        assess_cpu_frame_fairness(now, &demands, None)
            .record_turn(runner.cpu_frame_fairness.as_mut().unwrap(), &plan);

        runner.record_frame_schedule_admission(primary_key.clone());

        let primary_sample = runner
            .cpu_frame_fairness
            .as_ref()
            .unwrap()
            .projection()
            .window(&primary_key)
            .unwrap()
            .latest_sample()
            .unwrap();
        assert!(primary_sample.cursor_admitted);
        assert_eq!(
            runner
                .frame_scheduler
                .observe(now, &demands, FrameScheduleDeadlines::default())
                .selected,
            Some(auxiliary_key)
        );
    }

    #[test]
    fn parent_fairness_history_uses_existing_removal_and_recovery_fences() {
        let mut runner = runner();
        let now = Instant::now();
        let key = FrameScheduleKey::Auxiliary("settings".to_owned());
        let demands = [FrameScheduleDemand::from_cadence(
            key.clone(),
            TimedFrameCadence::Idle,
            60,
            RuntimeAnimationActivity::idle(),
            false,
            FrameScheduleRedrawEvidence::default(),
        )];
        let plan = runner
            .frame_scheduler
            .observe(now, &demands, FrameScheduleDeadlines::default());
        assess_cpu_frame_fairness(now, &demands, None)
            .record_turn(runner.cpu_frame_fairness.as_mut().unwrap(), &plan);
        assert!(
            runner
                .cpu_frame_fairness
                .as_ref()
                .unwrap()
                .projection()
                .window(&key)
                .is_some()
        );

        runner.remove_cpu_frame_observation(&key);
        assert!(
            runner
                .cpu_frame_fairness
                .as_ref()
                .unwrap()
                .projection()
                .window(&key)
                .is_none()
        );

        assess_cpu_frame_fairness(now, &demands, None)
            .record_turn(runner.cpu_frame_fairness.as_mut().unwrap(), &plan);
        runner.clear_cpu_frame_observation();
        assert!(
            runner
                .cpu_frame_fairness
                .as_ref()
                .unwrap()
                .projection()
                .window(&key)
                .is_none()
        );
    }

    #[test]
    fn auxiliary_runner_omits_parent_fairness_ledger() {
        let mut runner = runner();
        runner.mark_as_auxiliary();
        assert!(runner.cpu_frame_fairness.is_none());
    }

    #[test]
    fn native_closing_fences_runner_admission_predicates() {
        let mut runner = runner();
        assert!(runner.is_running());
        assert!(runner.should_initialize_runtime());
        assert!(runner.should_admit_auxiliary_sync());

        assert!(runner.native_lifecycle.admit_closing(Instant::now()));

        assert!(!runner.is_running());
        assert!(runner.is_closing());
        assert!(!runner.should_initialize_runtime());
        assert!(!runner.should_admit_auxiliary_sync());
        assert!(runner.native_shutdown_requested());
    }

    #[test]
    fn native_recovery_round_trip_fences_without_terminal_cause() {
        let mut runner = runner();

        assert!(runner.admit_device_recovery());
        assert!(runner.is_recovering());
        assert!(!runner.is_running());
        assert!(!runner.is_closing());
        assert!(!runner.has_terminal_cause());
        assert!(!runner.should_admit_auxiliary_sync());
        let diagnostics = runner.core.runtime.runtime_diagnostics();
        assert_eq!(
            diagnostics.lifecycle.phase,
            crate::runtime::RuntimeLifecyclePhase::Recovering
        );
        assert_eq!(diagnostics.lifecycle.transition_count, 2);

        assert!(runner.finish_device_recovery());
        assert!(runner.is_running());
        assert!(!runner.has_terminal_cause());
        let diagnostics = runner.core.runtime.runtime_diagnostics();
        assert_eq!(
            diagnostics.lifecycle.phase,
            crate::runtime::RuntimeLifecyclePhase::Running
        );
        assert_eq!(diagnostics.lifecycle.transition_count, 3);
        assert_eq!(
            diagnostics.lifecycle.history,
            vec![
                crate::runtime::RuntimeLifecycleTransition {
                    sequence: 1,
                    from: crate::runtime::RuntimeLifecyclePhase::Starting,
                    to: crate::runtime::RuntimeLifecyclePhase::Running,
                },
                crate::runtime::RuntimeLifecycleTransition {
                    sequence: 2,
                    from: crate::runtime::RuntimeLifecyclePhase::Running,
                    to: crate::runtime::RuntimeLifecyclePhase::Recovering,
                },
                crate::runtime::RuntimeLifecycleTransition {
                    sequence: 3,
                    from: crate::runtime::RuntimeLifecyclePhase::Recovering,
                    to: crate::runtime::RuntimeLifecyclePhase::Running,
                },
            ]
        );
    }

    #[test]
    fn native_recovery_completion_preserves_controller_closing_veto() {
        let mut runner = runner();

        assert!(runner.admit_device_recovery());
        assert!(runner.core.runtime.begin_closing());
        let diagnostics = runner.core.runtime.runtime_diagnostics();
        assert_eq!(
            diagnostics.lifecycle.phase,
            crate::runtime::RuntimeLifecyclePhase::Closing
        );
        assert!(runner.is_recovering());
        assert!(!runner.finish_device_recovery());
        assert!(runner.is_recovering());
    }

    #[test]
    fn overdue_recovery_completion_is_not_admissible() {
        assert!(!recovery_completion_is_admissible(true));
        assert!(recovery_completion_is_admissible(false));
    }

    #[test]
    fn stopped_runner_cannot_resume_normal_admission() {
        let mut runner = runner();
        assert!(runner.native_lifecycle.admit_closing(Instant::now()));
        assert!(runner.native_lifecycle.finish_closing());
        assert!(!runner.is_running());
        assert!(!runner.is_closing());
        assert!(runner.native_shutdown_requested());
        assert!(!runner.native_lifecycle.admit_closing(Instant::now()));
        assert!(matches!(runner.native_lifecycle, NativeLifecycle::Stopped));
    }
}
