//! Runner state and redraw coordination for the generic native Vello runtime.

use super::{
    ActivationRevealController, ApplicationReopenRegistration, AuxiliaryNativeWindow,
    DeviceLossRegistration, FrameWork, FrameWorkReason, GenericNativeRuntimeCore,
    GenericRouteOutcome, NativeAutomationTargetExporter, NativeGenericRunError,
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
    runner_state::NativeTargetGeneration,
    scene::{
        ArtifactFeasibilityObservation, NativePaintSegmentPayload,
        materialize_native_paint_segment_artifacts,
    },
};
use crate::{
    gui::types::Vector2,
    gui_runtime::native_vello::NativeTextRenderer,
    runtime::{NativeRunOptions, RuntimeAnimationActivity, RuntimeBridge},
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};
use vello::Scene;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};

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
    pub(super) window: NativeRunnerWindowState,
    pub(super) frame: NativeVelloFrameState,
    pub(super) input: NativeRunnerInputState,
    pub(super) timing: NativeRunnerTimingState,
    pub(super) frame_diagnostics_enabled: bool,
    pub(super) automation_targets: NativeAutomationTargetExporter,
    pub(super) auxiliary_windows: Vec<AuxiliaryNativeWindow<Message>>,
    terminal_cause: Option<NativeGenericRunError>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct AppliedRouteOutcome {
    pub(super) exit_requested: bool,
    pub(super) sync_auxiliary_windows_now: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DeviceLossEventSource {
    Primary,
    Auxiliary,
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
        Self {
            options,
            core,
            runtime_wakeup: RuntimeWakeup::default(),
            activation_reveal,
            application_reopen_proxy: None,
            application_reopen_events: None,
            window: NativeRunnerWindowState::default(),
            frame: NativeVelloFrameState::new(text_renderer, retained_surface_cache),
            input: NativeRunnerInputState::default(),
            timing: NativeRunnerTimingState::default(),
            frame_diagnostics_enabled,
            automation_targets: NativeAutomationTargetExporter::from_env(),
            auxiliary_windows: Vec::new(),
            terminal_cause: None,
        }
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
        self.window.window.is_none() && !self.has_terminal_cause()
    }

    pub(super) fn should_admit_auxiliary_sync(&self) -> bool {
        !self.has_terminal_cause()
    }

    pub(super) fn handle_device_lost_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        registration: Arc<DeviceLossRegistration>,
        message: String,
    ) {
        let Some(source) = self.device_loss_event_source(&registration) else {
            return;
        };
        let cause = NativeGenericRunError::RenderDeviceLost(message);
        match source {
            DeviceLossEventSource::Primary => {
                self.record_render_device_lost_and_exit(event_loop, cause)
            }
            DeviceLossEventSource::Auxiliary => {
                self.record_auxiliary_terminal_cause_and_exit(event_loop, cause)
            }
        }
    }

    pub(super) fn device_loss_event_source(
        &self,
        registration: &Arc<DeviceLossRegistration>,
    ) -> Option<DeviceLossEventSource> {
        if self
            .window
            .device_loss_registration
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, registration))
        {
            return Some(DeviceLossEventSource::Primary);
        }
        if self
            .auxiliary_windows
            .iter()
            .any(|window| window.accepts_device_loss(registration))
        {
            return Some(DeviceLossEventSource::Auxiliary);
        }
        None
    }

    pub(super) fn record_initialization_error_and_exit(
        &mut self,
        event_loop: &ActiveEventLoop,
        cause: NativeGenericRunError,
    ) {
        if self.record_terminal_cause(cause.clone()) {
            error!(error = %cause, "radiant generic native vello: initialization failed");
            event_loop.exit();
        }
    }

    pub(super) fn record_frame_render_error_and_exit(
        &mut self,
        event_loop: &ActiveEventLoop,
        cause: NativeGenericRunError,
    ) {
        if self.record_terminal_cause(cause.clone()) {
            error!(error = %cause, "radiant generic native vello: frame rendering failed");
            event_loop.exit();
        }
    }

    pub(super) fn record_render_device_lost_and_exit(
        &mut self,
        event_loop: &ActiveEventLoop,
        cause: NativeGenericRunError,
    ) {
        if self.record_terminal_cause(cause.clone()) {
            error!(error = %cause, "radiant generic native vello: render device lost");
            event_loop.exit();
        }
    }

    pub(super) fn record_auxiliary_terminal_cause_and_exit(
        &mut self,
        event_loop: &ActiveEventLoop,
        cause: NativeGenericRunError,
    ) {
        if self.record_terminal_cause(cause.clone()) {
            error!(error = %cause, "radiant generic native vello: auxiliary runtime failed");
            event_loop.exit();
        }
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
        self.record_frame_work(frame_work);
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
        if !self.frame_diagnostics_enabled {
            return;
        }
        self.timing.pending_frame_work = self.timing.pending_frame_work.merge(frame_work);
    }

    pub(super) fn take_pending_frame_work(&mut self) -> FrameWork {
        if !self.frame_diagnostics_enabled {
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
        self.timing.last_timed_frame_drain = now;
        self.core
            .drain_timed_frame(animation_activity, needs_text_caret_animation)
    }

    pub(super) fn merge_due_timed_frame_for_route(&mut self, outcome: &mut GenericRouteOutcome) {
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
        let assembly_attempted = matches!(
            self.frame.last_native_paint_segment_eligibility.outcome,
            super::retained_paint_segments::NativePaintSegmentEligibilityOutcome::Plan
        );
        let mut assembly_vetoed = false;
        if assembly_attempted {
            match self.frame.assemble_mixed_native_scene(
                viewport,
                paint,
                scene_validity,
                self.window.target_generation,
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
        let eligibility = self.frame.last_native_paint_segment_eligibility;
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
        self.frame.refresh_gpu_surface_interaction_regions();
        self.frame.refresh_post_gpu_overlay_cache();
        self.restore_native_hover_cursor_overlay();
        self.frame.mark_scene_content_dirty();
        self.export_automation_targets();
    }

    pub(super) fn export_automation_targets(&mut self) {
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

    #[cfg(target_os = "macos")]
    pub(super) fn queue_accessibility_display_snapshot(
        &mut self,
        next: super::window_environment::AccessibilityDisplaySnapshot,
    ) {
        let previous = self.window.accessibility_display;
        self.window.accessibility_display = next;
        let environment = super::window_environment::environment_for_native_state(
            self.window.dpi_scale,
            self.window.environment.color_scheme(),
            next,
        );
        let changed = self.update_window_environment(environment);
        for change in super::window_environment::accessibility_display_changes(previous, next) {
            if changed {
                self.queue_window_environment_change(change);
            }
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
        let pending_redraw_at_route_start = self.pending_redraw_elapsed(Instant::now());
        let applied = self.apply_route_outcome(outcome);
        if applied.exit_requested {
            event_loop.exit();
            return;
        }
        if applied.sync_auxiliary_windows_now
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
                self.redraw_and_exit_on_error(event_loop);
            }
        }
    }

    pub(super) fn apply_route_outcome(
        &mut self,
        mut outcome: GenericRouteOutcome,
    ) -> AppliedRouteOutcome {
        self.merge_due_timed_frame_for_route(&mut outcome);
        if outcome.exit_requested {
            return AppliedRouteOutcome {
                exit_requested: true,
                sync_auxiliary_windows_now: false,
            };
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
