use super::frame_scheduler_policy::{
    DiscreteInputCompletion, ImmediateTransientCompletion, NativeInputStageDisposition,
    discrete_input_completion_disposition, immediate_transient_completion_disposition,
};
use super::lifecycle_pointer::{
    NativeCursorLeftRoute, NativeCursorMovedRoute, finalize_native_immediate_transient_route,
};
#[cfg(test)]
use super::native_discrete_input_stage::complete_native_discrete_input_at;
use super::native_discrete_input_stage::{NativeDiscreteInputKind, NativeDiscreteInputStageTicket};
use super::native_immediate_transient_stage::{
    NativeImmediateTransientKind, NativeImmediateTransientStageTicket,
};
#[cfg(test)]
use super::native_lifecycle_stage::NativeLifecycleStageEvidence;
use super::native_lifecycle_stage::NativeLifecycleStageTicket;
use super::native_pointer::NativeWheelRoute;
use super::native_pointer_ingress::GestureInput;
use super::native_visual_packet::{NativeVisualRequestBegin, NativeVisualRequestDisposition};
use super::renderer_recovery::NativeRendererRecoveryWindowKind;
use super::runner_state::{
    NativeWindowDiagnosticIdentityAllocator, NativeWindowGpuTimingConfig,
    NativeWindowResourceBundle,
};
#[cfg(test)]
use super::scene_texture::NativeFrameRenderFailure;
use super::{
    AuxiliaryScheduleEligibility, CpuFrameObservationCapture, CpuFrameObservationOwner,
    FrameScheduleDemand, FrameScheduleKey, FrameScheduleRedrawEvidence, FrameWork, FrameWorkReason,
    GenericNativeAdapterOwner, GenericNativeVelloRunner, GenericRouteOutcome,
    NativeAdapterGeneration, NativeGenericRunError, NativeResourceMaintenanceTurn,
    RuntimeUserEvent, SceneRebuildMode, initial_viewport, owner_window_handle,
};
use crate::gui::input::InputTimestamp;
use crate::gui_runtime::native_vello::{select_present_mode, startup_renderer_options};
use crate::runtime::{
    AuxiliaryFocusRequest, AuxiliaryWindowOwner, FrameGpuTimingSample, RuntimeAnimationActivity,
};
#[cfg(test)]
use crate::runtime::{
    AuxiliaryWindow, NativeFrameDiagnostics, NativeRunOptions, NativeWindowDiagnosticIdentity,
    RuntimeBridge,
};
#[cfg(not(test))]
use crate::runtime::{
    AuxiliaryWindow, NativeRunOptions, NativeWindowDiagnosticIdentity, RuntimeBridge,
};
use crate::runtime::{ExternalDragIdentity, ExternalDragOutcome};
pub(super) use bridge::AuxiliaryFrameDiagnostics;
use bridge::AuxiliarySurfaceBridge;
use placement::centered_position;
use std::time::Instant;
use winit::{
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

mod bridge;
#[cfg(test)]
mod command_projection_tests;
mod placement;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AuxiliaryNativeWindowLifecycle {
    Admitted,
    Retiring,
}

pub(super) struct AuxiliaryNativeDiscreteInputRoute {
    pub(super) ticket: NativeDiscreteInputStageTicket,
    pub(super) outcome: Option<GenericRouteOutcome>,
}

pub(super) struct AuxiliaryNativeDiscreteInputResolution {
    disposition: NativeInputStageDisposition,
    child_outcome: Option<GenericRouteOutcome>,
}

pub(super) struct AuxiliaryNativeImmediateTransientRoute {
    pub(super) ticket: NativeImmediateTransientStageTicket,
    pub(super) kind: AuxiliaryNativeImmediateTransientRouteKind,
}

pub(super) enum AuxiliaryNativeImmediateTransientRouteKind {
    Focused {
        outcome: GenericRouteOutcome,
        launch_external_drag: bool,
    },
    CursorEntered,
    CursorMoved(NativeCursorMovedRoute),
    CursorLeft(NativeCursorLeftRoute),
    MouseWheel(NativeWheelRoute),
}

pub(super) struct AuxiliaryNativeImmediateTransientResolution {
    disposition: NativeInputStageDisposition,
    child_route: AuxiliaryNativeImmediateTransientResolvedRoute,
}

pub(super) enum AuxiliaryNativeImmediateTransientResolvedRoute {
    None,
    Outcome(GenericRouteOutcome),
    CursorMoved(NativeCursorMovedRoute),
    MouseWheel(NativeWheelRoute),
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RetiringResourceTestState {
    PendingSubmission,
    Completed,
}

pub(super) struct AuxiliaryNativeWindow<Message> {
    key: String,
    owner: AuxiliaryWindowOwner,
    close_message: Option<Message>,
    cache_on_close: bool,
    runner: GenericNativeVelloRunner<AuxiliarySurfaceBridge<Message>, Message>,
    active: bool,
    lifecycle: AuxiliaryNativeWindowLifecycle,
    recovery_rebuild_pending: bool,
    #[cfg(test)]
    retiring_resource_test_state: Option<RetiringResourceTestState>,
}

impl<Message> AuxiliaryNativeWindow<Message> {
    #[cfg(test)]
    pub(super) fn new(
        projection: AuxiliaryWindow<Message>,
        parent_options: &NativeRunOptions,
        native_window_diagnostic_identity: Option<NativeWindowDiagnosticIdentity>,
        frame_diagnostics_enabled: bool,
        frame_profile_host_enabled: bool,
    ) -> Self {
        let owner = AuxiliaryWindowOwner::new(&projection.key);
        Self::new_with_owner(
            projection,
            parent_options,
            native_window_diagnostic_identity,
            frame_diagnostics_enabled,
            frame_profile_host_enabled,
            false,
            owner,
        )
    }

    pub(super) fn new_with_owner(
        projection: AuxiliaryWindow<Message>,
        parent_options: &NativeRunOptions,
        native_window_diagnostic_identity: Option<NativeWindowDiagnosticIdentity>,
        frame_diagnostics_enabled: bool,
        frame_profile_host_enabled: bool,
        frame_gpu_timing_host_enabled: bool,
        owner: AuxiliaryWindowOwner,
    ) -> Self {
        let viewport = initial_viewport(&projection.options);
        let cache_on_close = projection.caches_on_close();
        let mut options = projection.options;
        options.frame.debug_layout |= parent_options.frame.debug_layout;
        if options.text.embedded_fonts.is_empty() && options.text.font_paths.is_empty() {
            options.text = parent_options.text.clone();
        }
        let frame_profile_enabled =
            options.frame.profiling.is_frame() && frame_profile_host_enabled;
        let bridge = AuxiliarySurfaceBridge::new_with_gpu_timing(
            projection.surface,
            frame_diagnostics_enabled,
            frame_profile_enabled,
            frame_gpu_timing_host_enabled,
        );
        let runner = GenericNativeVelloRunner::new_auxiliary_with_diagnostic_identity(
            options,
            bridge,
            viewport,
            native_window_diagnostic_identity,
            NativeWindowDiagnosticIdentityAllocator::exhausted(),
            projection.key.clone(),
        );
        Self {
            key: projection.key,
            owner,
            close_message: projection.close_message,
            cache_on_close,
            runner,
            active: true,
            lifecycle: AuxiliaryNativeWindowLifecycle::Admitted,
            recovery_rebuild_pending: false,
            #[cfg(test)]
            retiring_resource_test_state: None,
        }
    }

    pub(super) fn key(&self) -> &str {
        &self.key
    }

    pub(super) fn effect_owner(&self) -> AuxiliaryWindowOwner {
        self.owner.clone()
    }

    fn can_apply_auxiliary_focus_request(&self, request: &AuxiliaryFocusRequest) -> bool {
        self.is_admitted()
            && self.active
            && self.runner.is_running()
            && request.owner().is_open()
            && self.owner.is_open()
            && self.owner.is_same_generation(request.owner())
    }

    fn execute_auxiliary_focus_request(
        &mut self,
        request: AuxiliaryFocusRequest,
    ) -> GenericRouteOutcome {
        let outcome = self
            .runner
            .core
            .runtime
            .execute_command(request.into_command());
        self.runner.core.route_command_outcome(outcome)
    }

    fn apply_auxiliary_focus_request(
        &mut self,
        event_loop: &ActiveEventLoop,
        request: AuxiliaryFocusRequest,
        adapter: &mut GenericNativeAdapterOwner,
    ) {
        let outcome = self.execute_auxiliary_focus_request(request);
        self.runner
            .handle_route_outcome_with_adapter(event_loop, outcome, adapter, None);
    }

    pub(super) fn take_cpu_frame_observation_capture(&mut self) -> CpuFrameObservationCapture {
        self.runner.take_cpu_frame_observation_capture()
    }

    pub(super) fn require_scheduled_frame_admission(&mut self) {
        self.runner
            .core
            .runtime
            .bridge_mut()
            .require_schedule_admission();
    }

    pub(super) fn mark_parent_observation_finalized(&mut self) {
        self.runner
            .core
            .runtime
            .bridge_mut()
            .mark_observation_finalized();
    }

    pub(super) fn mark_scheduled_frame_admission_recorded(&mut self) {
        self.runner
            .core
            .runtime
            .bridge_mut()
            .mark_schedule_admission_recorded();
    }

    #[cfg(test)]
    pub(super) fn take_ready_frame_diagnostics(&mut self) -> Option<NativeFrameDiagnostics> {
        self.runner
            .core
            .runtime
            .bridge_mut()
            .take_ready_frame_diagnostics()
    }

    pub(super) fn take_ready_frame_observation(&mut self) -> Option<AuxiliaryFrameDiagnostics> {
        self.runner
            .core
            .runtime
            .bridge_mut()
            .take_ready_frame_observation()
    }

    pub(super) fn finalize_parent_frame_observation(
        &mut self,
        scheduled_admission_recorded: bool,
    ) -> Option<AuxiliaryFrameDiagnostics> {
        self.mark_parent_observation_finalized();
        if scheduled_admission_recorded {
            self.mark_scheduled_frame_admission_recorded();
        }
        self.take_ready_frame_observation()
    }

    #[cfg(test)]
    pub(super) fn stage_frame_diagnostics_for_test(&mut self, diagnostics: NativeFrameDiagnostics) {
        self.runner
            .core
            .runtime
            .host_observe_frame_diagnostics(diagnostics);
    }

    pub(super) fn discard_frame_diagnostics(&mut self) {
        self.runner
            .core
            .runtime
            .bridge_mut()
            .discard_frame_diagnostics();
    }

    pub(super) fn take_ready_frame_gpu_timing(&mut self) -> Option<FrameGpuTimingSample> {
        self.runner
            .core
            .runtime
            .bridge_mut()
            .take_ready_frame_gpu_timing()
    }

    pub(super) fn discard_frame_gpu_timing(&mut self) {
        self.runner
            .core
            .runtime
            .bridge_mut()
            .discard_frame_gpu_timing();
    }

    pub(super) fn process_native_gpu_timing_ready(
        &mut self,
        generation: NativeAdapterGeneration,
        resource_identity: u64,
        slot: u8,
        token: u64,
    ) -> Option<FrameGpuTimingSample> {
        if !self.is_admitted() || !self.runner.is_running() {
            self.discard_native_gpu_timing_ready(generation, resource_identity, slot, token);
            return None;
        }
        self.runner
            .process_native_gpu_timing_ready(generation, resource_identity, slot, token);
        self.take_ready_frame_gpu_timing()
    }

    pub(super) fn discard_native_gpu_timing_ready(
        &mut self,
        generation: NativeAdapterGeneration,
        resource_identity: u64,
        slot: u8,
        token: u64,
    ) {
        self.runner
            .discard_native_gpu_timing_ready(generation, resource_identity, slot, token);
        self.discard_frame_gpu_timing();
    }

    pub(super) fn record_native_interactive_arrival(&mut self, arrived_at: Instant) {
        self.runner.record_native_interactive_arrival(arrived_at);
    }

    fn event_result(
        &mut self,
        terminal_cause: Option<NativeGenericRunError>,
        visual_deadline_completed: bool,
    ) -> AuxiliaryWindowEventResult<Message> {
        self.event_result_with_native_discrete_input(
            terminal_cause,
            visual_deadline_completed,
            None,
        )
    }

    fn event_result_with_native_discrete_input(
        &mut self,
        terminal_cause: Option<NativeGenericRunError>,
        visual_deadline_completed: bool,
        native_discrete_input_route: Option<AuxiliaryNativeDiscreteInputRoute>,
    ) -> AuxiliaryWindowEventResult<Message> {
        self.event_result_with_native_routes(
            terminal_cause,
            visual_deadline_completed,
            native_discrete_input_route,
            None,
        )
    }

    fn event_result_with_native_routes(
        &mut self,
        terminal_cause: Option<NativeGenericRunError>,
        visual_deadline_completed: bool,
        native_discrete_input_route: Option<AuxiliaryNativeDiscreteInputRoute>,
        native_immediate_transient_route: Option<AuxiliaryNativeImmediateTransientRoute>,
    ) -> AuxiliaryWindowEventResult<Message> {
        AuxiliaryWindowEventResult {
            messages: self.take_messages(),
            message_origin: Some(self.owner.clone()),
            close_admission: None,
            terminal_cause,
            shutdown_requested: self.runner.native_shutdown_requested(),
            visual_deadline_completed,
            native_discrete_input_route,
            native_immediate_transient_route,
        }
    }

    pub(super) fn resolve_native_discrete_input_route(
        &mut self,
        pending: AuxiliaryNativeDiscreteInputRoute,
    ) -> Option<AuxiliaryNativeDiscreteInputResolution> {
        let AuxiliaryNativeDiscreteInputRoute { ticket, outcome } = pending;
        Self::resolve_native_discrete_input_outcome(
            outcome,
            self.runner.complete_native_discrete_input(ticket),
        )
    }

    pub(super) fn resolve_native_immediate_transient_route(
        &mut self,
        pending: AuxiliaryNativeImmediateTransientRoute,
    ) -> Option<AuxiliaryNativeImmediateTransientResolution> {
        let AuxiliaryNativeImmediateTransientRoute { ticket, kind } = pending;
        let completion = self.runner.complete_native_immediate_transient(ticket);
        self.resolve_native_immediate_transient_route_with_completion(kind, completion)
    }

    fn resolve_native_immediate_transient_route_with_completion(
        &mut self,
        kind: AuxiliaryNativeImmediateTransientRouteKind,
        completion: ImmediateTransientCompletion,
    ) -> Option<AuxiliaryNativeImmediateTransientResolution> {
        let disposition = immediate_transient_completion_disposition(completion)?;
        let child_route = match kind {
            AuxiliaryNativeImmediateTransientRouteKind::Focused {
                outcome,
                launch_external_drag,
            } => AuxiliaryNativeImmediateTransientResolvedRoute::Outcome(
                finalize_native_immediate_transient_route(
                    completion,
                    outcome,
                    launch_external_drag,
                    || self.runner.launch_external_drag_if_armed(),
                )?,
            ),
            AuxiliaryNativeImmediateTransientRouteKind::CursorEntered => {
                AuxiliaryNativeImmediateTransientResolvedRoute::None
            }
            AuxiliaryNativeImmediateTransientRouteKind::CursorMoved(mut route) => {
                route.outcome = route
                    .outcome
                    .with_native_input_stage_disposition(disposition);
                AuxiliaryNativeImmediateTransientResolvedRoute::CursorMoved(route)
            }
            AuxiliaryNativeImmediateTransientRouteKind::CursorLeft(route) => {
                AuxiliaryNativeImmediateTransientResolvedRoute::Outcome(
                    finalize_native_immediate_transient_route(
                        completion,
                        route.outcome,
                        route.launch_external_drag,
                        || self.runner.launch_external_drag_if_armed(),
                    )?,
                )
            }
            AuxiliaryNativeImmediateTransientRouteKind::MouseWheel(mut route) => {
                route.outcome = route
                    .outcome
                    .with_native_input_stage_disposition(disposition);
                AuxiliaryNativeImmediateTransientResolvedRoute::MouseWheel(route)
            }
        };
        Some(AuxiliaryNativeImmediateTransientResolution {
            disposition,
            child_route,
        })
    }

    #[cfg(test)]
    pub(super) fn resolve_native_immediate_transient_route_at(
        &mut self,
        pending: AuxiliaryNativeImmediateTransientRoute,
        completed_at: Option<Instant>,
    ) -> Option<AuxiliaryNativeImmediateTransientResolution> {
        let AuxiliaryNativeImmediateTransientRoute { ticket, kind } = pending;
        let completion =
            super::native_immediate_transient_stage::complete_native_immediate_transient_at(
                &mut self.runner.frame_stage_owner,
                ticket,
                completed_at,
            );
        self.resolve_native_immediate_transient_route_with_completion(kind, completion)
    }

    fn resolve_native_discrete_input_outcome(
        outcome: Option<GenericRouteOutcome>,
        completion: DiscreteInputCompletion,
    ) -> Option<AuxiliaryNativeDiscreteInputResolution> {
        let disposition = discrete_input_completion_disposition(completion)?;
        let child_outcome =
            outcome.map(|outcome| outcome.with_native_input_stage_disposition(disposition));
        Some(AuxiliaryNativeDiscreteInputResolution {
            disposition,
            child_outcome,
        })
    }

    #[cfg(test)]
    pub(super) fn resolve_native_discrete_input_route_at(
        &mut self,
        pending: AuxiliaryNativeDiscreteInputRoute,
        completed_at: Option<Instant>,
    ) -> Option<AuxiliaryNativeDiscreteInputResolution> {
        let AuxiliaryNativeDiscreteInputRoute { ticket, outcome } = pending;
        Self::resolve_native_discrete_input_outcome(
            outcome,
            complete_native_discrete_input_at(
                &mut self.runner.frame_stage_owner,
                ticket,
                completed_at,
            ),
        )
    }

    pub(super) fn cancel_native_discrete_input_route(
        &mut self,
        pending: AuxiliaryNativeDiscreteInputRoute,
    ) -> bool {
        self.runner.veto_native_discrete_input(pending.ticket)
    }

    pub(super) fn cancel_native_immediate_transient_route(
        &mut self,
        pending: AuxiliaryNativeImmediateTransientRoute,
    ) -> bool {
        self.runner.veto_native_immediate_transient(pending.ticket)
    }

    pub(super) fn apply_native_discrete_input_route_with_adapter(
        &mut self,
        event_loop: &ActiveEventLoop,
        outcome: GenericRouteOutcome,
        adapter: &mut GenericNativeAdapterOwner,
    ) {
        self.runner
            .handle_route_outcome_with_adapter(event_loop, outcome, adapter, None);
    }

    pub(super) fn apply_native_immediate_transient_route_with_adapter(
        &mut self,
        event_loop: &ActiveEventLoop,
        resolution: AuxiliaryNativeImmediateTransientResolution,
        adapter: &mut GenericNativeAdapterOwner,
    ) {
        match resolution.child_route {
            AuxiliaryNativeImmediateTransientResolvedRoute::None => {}
            AuxiliaryNativeImmediateTransientResolvedRoute::Outcome(outcome) => {
                self.runner
                    .handle_route_outcome_with_adapter(event_loop, outcome, adapter, None);
            }
            AuxiliaryNativeImmediateTransientResolvedRoute::CursorMoved(route) => {
                self.runner.apply_cursor_moved_route(route);
            }
            AuxiliaryNativeImmediateTransientResolvedRoute::MouseWheel(route) => {
                let outcome = route.outcome;
                self.runner.apply_native_mouse_wheel_route(route);
                self.runner
                    .handle_route_outcome_with_adapter(event_loop, outcome, adapter, None);
            }
        }
    }

    pub(super) fn is_admitted(&self) -> bool {
        matches!(self.lifecycle, AuxiliaryNativeWindowLifecycle::Admitted)
    }

    pub(super) fn is_retiring(&self) -> bool {
        matches!(self.lifecycle, AuxiliaryNativeWindowLifecycle::Retiring)
    }

    pub(super) fn native_surface_target_retirement_deadline(&self) -> Option<Instant> {
        self.is_admitted()
            .then(|| self.runner.native_surface_target_retirement_deadline())
            .flatten()
    }

    pub(super) fn wake_native_surface_target_retirement_maintenance(&mut self) {
        if self.is_admitted() {
            self.runner
                .wake_native_surface_target_retirement_maintenance();
        }
    }

    pub(super) fn maintain_native_surface_target_retirement_if_due_with_turn(
        &mut self,
        now: Instant,
        parent_adapter_generation: NativeAdapterGeneration,
        turn: &mut NativeResourceMaintenanceTurn,
    ) {
        if self.is_admitted() {
            self.runner
                .maintain_native_surface_target_retirement_if_due_with_turn(
                    now,
                    parent_adapter_generation,
                    turn,
                );
        }
    }

    #[cfg(test)]
    pub(super) fn install_retiring_resource_test(&mut self) {
        assert!(self.is_retiring());
        self.retiring_resource_test_state = Some(RetiringResourceTestState::PendingSubmission);
    }

    #[cfg(test)]
    pub(super) fn retiring_resource_test_is_pending(&self) -> bool {
        self.retiring_resource_test_state == Some(RetiringResourceTestState::PendingSubmission)
    }

    #[cfg(test)]
    pub(super) fn retiring_resource_test_is_completed(&self) -> bool {
        self.retiring_resource_test_state == Some(RetiringResourceTestState::Completed)
    }

    pub(super) fn recovery_rebuild_pending(&self) -> bool {
        self.recovery_rebuild_pending
    }

    pub(super) fn can_prepare_device_recovery(&self, generation: NativeAdapterGeneration) -> bool {
        if !self.is_admitted() {
            return true;
        }
        self.runner.is_running()
            && self
                .runner
                .window
                .native_resources
                .as_ref()
                .is_none_or(|resources| resources.generation == generation)
            && self.runner.window.can_publish_native_resources()
    }

    pub(super) fn admit_device_recovery(&mut self) -> bool {
        if !self.is_admitted() {
            return true;
        }
        if !self.runner.admit_device_recovery() {
            return false;
        }
        self.recovery_rebuild_pending = self.runner.window.window.is_some();
        true
    }

    pub(super) fn admit_native_lifecycle(
        &mut self,
        adapter_generation: Option<NativeAdapterGeneration>,
    ) -> Option<NativeLifecycleStageTicket> {
        self.is_admitted()
            .then(|| self.runner.admit_native_lifecycle(adapter_generation))
            .flatten()
    }

    pub(super) fn admit_native_lifecycle_finish(
        &mut self,
        adapter_generation: Option<NativeAdapterGeneration>,
    ) -> Option<NativeLifecycleStageTicket> {
        self.is_admitted()
            .then(|| {
                self.runner
                    .admit_native_lifecycle_finish(adapter_generation)
            })
            .flatten()
    }

    pub(super) fn should_stage_native_closing(&self) -> bool {
        matches!(
            self.lifecycle,
            AuxiliaryNativeWindowLifecycle::Admitted | AuxiliaryNativeWindowLifecycle::Retiring
        ) && (self.runner.is_running() || self.runner.is_recovering())
    }

    pub(super) fn admit_native_closing(
        &mut self,
        adapter_generation: Option<NativeAdapterGeneration>,
    ) -> Option<NativeLifecycleStageTicket> {
        self.should_stage_native_closing()
            .then(|| self.runner.admit_native_closing(adapter_generation))
            .flatten()
    }

    #[cfg(test)]
    pub(super) fn admit_native_lifecycle_finish_with_evidence(
        &mut self,
        evidence: NativeLifecycleStageEvidence,
    ) -> Option<NativeLifecycleStageTicket> {
        self.is_admitted()
            .then(|| {
                self.runner
                    .admit_native_lifecycle_finish_with_evidence(evidence)
            })
            .flatten()
    }

    pub(super) fn native_lifecycle_stage_ticket_is_current(
        &self,
        ticket: &NativeLifecycleStageTicket,
    ) -> bool {
        self.runner.native_lifecycle_stage_ticket_is_current(ticket)
    }

    pub(super) fn native_lifecycle_ticket_is_current(
        &self,
        ticket: &NativeLifecycleStageTicket,
        adapter_generation: Option<NativeAdapterGeneration>,
    ) -> bool {
        self.runner
            .native_lifecycle_ticket_is_current(ticket, adapter_generation)
    }

    #[cfg(test)]
    pub(super) fn native_lifecycle_ticket_is_current_with_evidence(
        &self,
        ticket: &NativeLifecycleStageTicket,
        evidence: &NativeLifecycleStageEvidence,
    ) -> bool {
        self.runner
            .native_lifecycle_ticket_is_current_with_evidence(ticket, evidence)
    }

    pub(super) fn complete_native_lifecycle(&mut self, ticket: NativeLifecycleStageTicket) -> bool {
        self.runner.complete_native_lifecycle(ticket)
    }

    pub(super) fn veto_native_lifecycle(&mut self, ticket: NativeLifecycleStageTicket) -> bool {
        self.runner.veto_native_lifecycle(ticket)
    }

    /// Apply the local fences for an independently requested destructive close
    /// after the parent has preflighted the exact auxiliary owner.  The
    /// lifecycle ticket remains in flight until the parent has retired that
    /// owner and completes it after all local fences are installed.
    pub(super) fn prepare_destructive_close(
        &mut self,
        ticket: &NativeLifecycleStageTicket,
    ) -> bool {
        if !self.is_admitted() || !self.native_lifecycle_stage_ticket_is_current(ticket) {
            return false;
        }
        if self.runner.prepare_native_shutdown(None).is_none() {
            return false;
        }
        self.discard_frame_diagnostics();
        self.begin_retiring();
        true
    }

    pub(super) fn take_close_message(&mut self) -> Option<Message> {
        self.close_message.take()
    }

    #[cfg(test)]
    pub(super) fn has_close_message_for_test(&self) -> bool {
        self.close_message.is_some()
    }

    pub(super) fn prepare_whole_run_closing(&mut self) -> bool {
        if !matches!(
            self.lifecycle,
            AuxiliaryNativeWindowLifecycle::Admitted | AuxiliaryNativeWindowLifecycle::Retiring
        ) {
            return true;
        }
        self.runner.is_closing()
            || self.runner.is_stopped()
            || self.runner.prepare_native_shutdown(None).is_some()
    }

    pub(super) fn invalidate_terminal_convergence_stage_owner(&mut self) {
        self.runner.frame_stage_owner.invalidate();
    }

    #[cfg(test)]
    pub(super) fn frame_stage_owner_has_in_flight(&self) -> bool {
        self.runner.frame_stage_owner.has_in_flight()
    }

    #[cfg(test)]
    pub(super) fn begin_controller_closing_for_test(&mut self) -> bool {
        self.runner.core.runtime.begin_closing()
    }

    pub(super) fn quarantine_device_recovery_resources(
        &mut self,
        adapter: &mut GenericNativeAdapterOwner,
    ) -> bool {
        if !self.is_admitted() || !self.recovery_rebuild_pending {
            return true;
        }
        if !self.runner.window.quarantine_active_native_resources() {
            return false;
        }
        self.runner.refresh_atlas_residency_account(adapter);
        self.runner.refresh_signal_residency_account(adapter);
        self.runner.refresh_custom_shader_residency_account(adapter);
        self.runner.refresh_render_canvas_upload_account(adapter);
        true
    }

    pub(super) fn finish_device_recovery_if_no_rebuild(&mut self) -> bool {
        if !self.is_admitted() || self.recovery_rebuild_pending {
            return true;
        }
        self.runner.finish_device_recovery()
    }

    pub(super) fn rebuild_after_device_recovery(
        &mut self,
        adapter: &mut GenericNativeAdapterOwner,
        event_proxy: EventLoopProxy<RuntimeUserEvent>,
    ) -> Result<bool, NativeGenericRunError> {
        if !self.recovery_rebuild_pending {
            return Ok(false);
        }
        let window = self.runner.window.window.clone().ok_or_else(|| {
            NativeGenericRunError::NativeInitialization {
                stage: super::NativeInitializationStage::WgpuSurfaceCreation,
                message: String::from("recovering auxiliary window disappeared"),
            }
        })?;
        let instance =
            adapter
                .instance()
                .ok_or_else(|| NativeGenericRunError::NativeInitialization {
                    stage: super::NativeInitializationStage::WgpuSurfaceCreation,
                    message: String::from("fresh native adapter context is unavailable"),
                })?;
        let surface = instance.create_surface(window.clone()).map_err(|error| {
            NativeGenericRunError::NativeInitialization {
                stage: super::NativeInitializationStage::WgpuSurfaceCreation,
                message: error.to_string(),
            }
        })?;
        adapter
            .validate_auxiliary_surface(self.runner.options.gpu.backend, &surface)
            .map_err(|error| NativeGenericRunError::NativeInitialization {
                stage: super::NativeInitializationStage::DeviceAcquisition,
                message: error.to_string(),
            })?;
        let generation = adapter.capture_generation().ok_or_else(|| {
            NativeGenericRunError::NativeInitialization {
                stage: super::NativeInitializationStage::DeviceAcquisition,
                message: String::from("fresh native adapter has no known generation"),
            }
        })?;
        let device = adapter.selected_device_handle().ok_or_else(|| {
            NativeGenericRunError::NativeInitialization {
                stage: super::NativeInitializationStage::DeviceAcquisition,
                message: String::from("fresh native adapter has no selected device"),
            }
        })?;
        let present_modes = surface.get_capabilities(device.adapter()).present_modes;
        let present_mode =
            select_present_mode(self.runner.options.normalized_target_fps(), &present_modes);
        let size = window.inner_size();
        let render_surface = adapter
            .create_render_surface_for_selected(
                surface,
                size.width.max(1),
                size.height.max(1),
                present_mode,
            )
            .map_err(|error| NativeGenericRunError::NativeInitialization {
                stage: super::NativeInitializationStage::RenderSurfaceCreation,
                message: error.to_string(),
            })?;
        let renderer =
            vello::Renderer::new(&device.device, startup_renderer_options()).map_err(|error| {
                NativeGenericRunError::NativeInitialization {
                    stage: super::NativeInitializationStage::RendererCreation,
                    message: error.to_string(),
                }
            })?;
        let native_resources = NativeWindowResourceBundle::new(
            generation,
            render_surface,
            renderer,
            &device.device,
            &device.queue,
            event_proxy,
            NativeWindowGpuTimingConfig {
                route: self.runner.gpu_timing_route.clone(),
                enabled: self.runner.frame_gpu_timing_enabled,
            },
        )
        .ok_or_else(|| NativeGenericRunError::NativeInitialization {
            stage: super::NativeInitializationStage::DeviceAcquisition,
            message: String::from("fresh auxiliary bundle was not generation-bound"),
        })?;
        let Some(publication) = self.runner.window.reserve_native_resource_publication() else {
            return Err(NativeGenericRunError::NativeInitialization {
                stage: super::NativeInitializationStage::DeviceAcquisition,
                message: String::from("auxiliary quarantine capacity is exhausted"),
            });
        };
        publication.publish(native_resources);
        self.runner.refresh_atlas_residency_account(adapter);
        self.runner.refresh_signal_residency_account(adapter);
        self.runner.refresh_custom_shader_residency_account(adapter);
        self.runner.refresh_render_canvas_upload_account(adapter);
        self.runner.complete_native_recovery_target_transition();
        self.runner.frame.invalidate_native_resources_for_recovery();

        let Some(ticket) = self.runner.admit_native_lifecycle_finish(Some(generation)) else {
            return Err(NativeGenericRunError::NativeInitialization {
                stage: super::NativeInitializationStage::DeviceAcquisition,
                message: String::from("auxiliary finish lifecycle ticket was not admitted"),
            });
        };
        if adapter.capture_generation() != Some(generation)
            || !self
                .runner
                .native_lifecycle_ticket_is_current(&ticket, Some(generation))
        {
            let _ = self.runner.veto_native_lifecycle(ticket);
            return Err(NativeGenericRunError::NativeInitialization {
                stage: super::NativeInitializationStage::DeviceAcquisition,
                message: String::from("auxiliary finish lifecycle ticket was not current"),
            });
        }
        if !self.runner.finish_device_recovery() {
            let _ = self.runner.veto_native_lifecycle(ticket);
            return Err(NativeGenericRunError::NativeInitialization {
                stage: super::NativeInitializationStage::DeviceAcquisition,
                message: String::from("auxiliary native recovery lifecycle completion was vetoed"),
            });
        }
        if !self
            .runner
            .native_lifecycle_stage_ticket_is_current(&ticket)
        {
            let _ = self.runner.veto_native_lifecycle(ticket);
            return Err(NativeGenericRunError::NativeInitialization {
                stage: super::NativeInitializationStage::DeviceAcquisition,
                message: String::from("auxiliary finish lifecycle ticket owner changed"),
            });
        }
        if !self.runner.complete_native_lifecycle(ticket) {
            return Err(NativeGenericRunError::NativeInitialization {
                stage: super::NativeInitializationStage::DeviceAcquisition,
                message: String::from("auxiliary finish lifecycle ticket completion was vetoed"),
            });
        }
        self.runner.rebuild_scene();
        self.recovery_rebuild_pending = false;
        if self.active {
            // Recovery physically conceals the child without changing its
            // latest explicit visibility intent. Reapply that intent only
            // after the fresh bundle has been published successfully.
            self.runner
                .apply_native_window_visibility(self.runner.window.logical_window_visible);
            self.runner
                .request_redraw_for_frame_work(FrameWork::RebuildScene {
                    reason: FrameWorkReason::RuntimeSurfaceRepaint,
                    mode: SceneRebuildMode::Immediate,
                });
        } else {
            // Recovery rebuilds the cached scene state, but dormancy remains
            // authoritative until an explicit show resumes this mailbox.
            self.runner.suspend_native_visual_requests();
            self.runner
                .request_redraw_for_frame_work(FrameWork::RebuildScene {
                    reason: FrameWorkReason::RuntimeSurfaceRepaint,
                    mode: SceneRebuildMode::Immediate,
                });
        }
        Ok(true)
    }

    pub(super) fn maintain_native_resources_with_turn(
        &mut self,
        turn: &mut NativeResourceMaintenanceTurn,
        adapter: Option<&mut GenericNativeAdapterOwner>,
    ) -> bool {
        if self.is_retiring() {
            #[cfg(test)]
            if let Some(state) = self.retiring_resource_test_state {
                match state {
                    RetiringResourceTestState::PendingSubmission => {
                        self.retiring_resource_test_state =
                            Some(RetiringResourceTestState::Completed);
                        turn.record_pending_for_test();
                        return false;
                    }
                    RetiringResourceTestState::Completed => {
                        if turn.consume_drop_for_test() {
                            self.retiring_resource_test_state = None;
                            return true;
                        }
                        turn.record_pending_for_test();
                        return false;
                    }
                }
            }
            let empty = self.runner.retire_native_resources_with_turn(turn);
            if let Some(adapter) = adapter {
                self.runner.refresh_atlas_residency_account(adapter);
                self.runner.refresh_signal_residency_account(adapter);
                self.runner.refresh_custom_shader_residency_account(adapter);
                self.runner.refresh_render_canvas_upload_account(adapter);
            }
            return empty;
        }
        self.runner.maintain_native_resources_with_turn(turn);
        if let Some(adapter) = adapter {
            self.runner.refresh_atlas_residency_account(adapter);
            self.runner.refresh_signal_residency_account(adapter);
            self.runner.refresh_custom_shader_residency_account(adapter);
            self.runner.refresh_render_canvas_upload_account(adapter);
        }
        false
    }

    pub(super) fn native_resource_ownership_is_empty(&self) -> bool {
        self.runner.native_resource_ownership_is_empty()
    }

    pub(super) fn window_id(&self) -> Option<WindowId> {
        self.is_admitted()
            .then_some(self.runner.window.id)
            .flatten()
    }

    pub(super) fn dispatch_external_drag_completion(
        &mut self,
        event_loop: &ActiveEventLoop,
        identity: ExternalDragIdentity,
        result: Result<ExternalDragOutcome, String>,
        adapter: &mut GenericNativeAdapterOwner,
    ) {
        if !self.is_admitted() || !self.runner.is_running() {
            return;
        }
        let outcome = self
            .runner
            .core
            .runtime
            .dispatch_external_drag_launch_result(identity, result);
        let routed = self.runner.core.route_command_outcome(outcome);
        self.runner
            .handle_route_outcome_with_adapter(event_loop, routed, adapter, None);
    }

    pub(super) fn frame_schedule_eligibility(
        &self,
        current_generation: Option<NativeAdapterGeneration>,
    ) -> AuxiliaryScheduleEligibility {
        let native_resources = self.runner.window.native_resources.as_ref();
        AuxiliaryScheduleEligibility {
            active: self.active,
            admitted: self.is_admitted(),
            local_running: self.runner.is_running() && !self.runner.has_terminal_cause(),
            live_window: self.runner.window.id.is_some() && self.runner.window.window.is_some(),
            recovering: self.runner.is_recovering() || self.recovery_rebuild_pending,
            closing: self.runner.is_closing(),
            stopped: self.runner.is_stopped(),
            native_resources_present: native_resources.is_some(),
            resource_generation_current: native_resources
                .is_some_and(|resources| current_generation == Some(resources.generation)),
            mailbox_suspended: self.runner.window.native_visual_requests.is_suspended(),
            target_generation_known: self.runner.window.target_generation.is_known(),
            native_surface_target_unfenced: !self.runner.window.native_surface_target_fenced,
        }
    }

    fn native_discrete_input_wrapper_is_eligible(
        &self,
        adapter_generation: NativeAdapterGeneration,
    ) -> bool {
        self.frame_schedule_eligibility(Some(adapter_generation))
            .is_eligible()
    }

    pub(super) fn observe_frame_schedule(
        &mut self,
        now: Instant,
        current_generation: Option<NativeAdapterGeneration>,
    ) -> Option<FrameScheduleDemand> {
        let eligibility = self.frame_schedule_eligibility(current_generation);
        let ordinary = eligibility.is_eligible();
        let maintenance_deadline = current_generation.and_then(|generation| {
            eligibility
                .is_maintenance_eligible()
                .then(|| {
                    self.runner
                        .normal_native_resource_maintenance_deadline(now, Some(generation))
                })
                .flatten()
        });
        let recovery = !ordinary
            && eligibility.is_recovery_eligible()
            && self
                .runner
                .native_visual_request_recovery_schedule_is_eligible();
        if !ordinary && !recovery && maintenance_deadline.is_none() {
            return None;
        }
        let animation_activity = if ordinary {
            self.runner.core.animation_activity()
        } else {
            RuntimeAnimationActivity::idle()
        };
        let needs_text_caret_animation = ordinary && self.runner.core.has_focused_text_input();
        Some(
            FrameScheduleDemand::observe_runtime(
                FrameScheduleKey::Auxiliary(self.key.clone()),
                now,
                self.runner.timing.last_timed_frame_drain,
                self.runner.options.normalized_target_fps(),
                animation_activity,
                needs_text_caret_animation,
                FrameScheduleRedrawEvidence {
                    timed_repaint_deadline: ordinary
                        .then(|| self.runner.core.timed_repaint_deadline())
                        .flatten(),
                    pending_redraw_requested: self.runner.timing.redraw_requested,
                    pending_redraw_age: self.runner.pending_redraw_age(now),
                    pending_redraw_retry_deadline: self.runner.pending_redraw_retry_deadline(),
                    pending_redraw_fresh: self.runner.timing.redraw_requested
                        && !self.runner.pending_redraw_request_is_stale(now),
                },
            )
            .with_maintenance_deadline(maintenance_deadline),
        )
    }

    pub(super) fn wake_normal_native_resource_maintenance(
        &mut self,
        adapter_generation: NativeAdapterGeneration,
    ) {
        self.runner
            .wake_normal_native_resource_maintenance_with_generation(adapter_generation);
    }

    pub(super) fn admit_native_resource_maintenance(
        &mut self,
        adapter: &mut GenericNativeAdapterOwner,
        now: Instant,
        turn: &mut NativeResourceMaintenanceTurn,
    ) -> bool {
        let Some(parent_generation) = adapter.capture_generation() else {
            return false;
        };
        if !adapter.admit_generation(parent_generation)
            || !self
                .frame_schedule_eligibility(Some(parent_generation))
                .is_maintenance_eligible()
        {
            return false;
        }
        let admitted = self.runner.admit_native_resource_maintenance(
            now,
            &FrameScheduleKey::Auxiliary(self.key.clone()),
            parent_generation,
            turn,
        );
        if admitted {
            self.runner.refresh_atlas_residency_account(adapter);
            self.runner.refresh_signal_residency_account(adapter);
            self.runner.refresh_custom_shader_residency_account(adapter);
            self.runner.refresh_render_canvas_upload_account(adapter);
        }
        admitted
    }

    pub(super) fn admit_frame_schedule_work(
        &mut self,
        event_loop: &ActiveEventLoop,
        adapter: &mut GenericNativeAdapterOwner,
        observation: Option<&mut CpuFrameObservationOwner<'_>>,
        now: Instant,
        demand: &FrameScheduleDemand,
    ) -> Option<AuxiliaryWindowEventResult<Message>> {
        let parent_generation = adapter.capture_generation()?;
        if !adapter.admit_generation(parent_generation) {
            return None;
        }
        if !matches!(
            demand.key(),
            FrameScheduleKey::Auxiliary(key) if key == &self.key
        ) {
            return None;
        }
        let eligibility = self.frame_schedule_eligibility(Some(parent_generation));
        let schedule_eligible = eligibility.is_eligible()
            || (eligibility.is_recovery_eligible()
                && self
                    .runner
                    .native_visual_request_recovery_schedule_is_eligible());
        if !schedule_eligible {
            return None;
        }
        let admission =
            self.runner
                .admit_auxiliary_frame_schedule_work(now, demand, parent_generation);
        if !admission.did_work {
            return None;
        }
        self.require_scheduled_frame_admission();
        if admission.route_outcome {
            if admission.timed_frame_already_handled {
                self.runner
                    .handle_route_outcome_with_adapter_without_timed_frame(
                        event_loop,
                        admission.outcome,
                        adapter,
                        observation,
                    );
            } else {
                self.runner.handle_route_outcome_with_adapter(
                    event_loop,
                    admission.outcome,
                    adapter,
                    observation,
                );
            }
        }
        let terminal_cause = self.runner.take_terminal_cause();
        Some(self.event_result(terminal_cause, admission.visual_deadline_completed))
    }

    #[cfg(test)]
    pub(super) fn update_projection(&mut self, projection: AuxiliaryWindow<Message>) {
        let service = self.runner.core.runtime.bridge().command_service.clone();
        self.update_projection_with_commands(projection, service);
    }

    fn update_projection_with_commands(
        &mut self,
        projection: AuxiliaryWindow<Message>,
        service: Option<crate::application::CommandService<Message>>,
    ) {
        if !self.is_admitted() || self.recovery_rebuild_pending {
            return;
        }
        self.cache_on_close = projection.caches_on_close();
        self.close_message = projection.close_message;
        self.runner.core.runtime.bridge_mut().command_service = service;
        self.runner.core.runtime.bridge_mut().surface = projection.surface;
        self.runner.core.refresh_surface();
        self.runner.rebuild_scene();
        self.show();
        self.runner
            .request_redraw_for_frame_work(FrameWork::RebuildScene {
                reason: FrameWorkReason::RuntimeSurfaceRefresh,
                mode: SceneRebuildMode::ImmediateWithSurfaceRefresh,
            });
    }

    pub(super) fn initialize_runtime(
        &mut self,
        event_loop: &ActiveEventLoop,
        parent_window: Option<&Window>,
        event_proxy: EventLoopProxy<RuntimeUserEvent>,
        adapter: &mut GenericNativeAdapterOwner,
    ) -> Result<(), NativeGenericRunError> {
        if self
            .runner
            .options
            .window
            .behavior
            .owner_window_handle
            .is_none()
        {
            self.runner.options.window.behavior.owner_window_handle =
                owner_window_handle(parent_window);
        }
        if self.runner.options.window.geometry.position.is_none() {
            self.runner.options.window.geometry.position =
                centered_position(parent_window, &self.runner.options);
        }
        self.runner
            .initialize_runtime_with_adapter(event_loop, event_proxy, adapter)
    }

    pub(super) fn take_native_ime_adapter_observation(
        &mut self,
    ) -> Option<crate::runtime::NativeImeAdapterObservation> {
        self.runner.take_native_ime_adapter_observation()
    }

    pub(super) fn hide(&mut self) {
        self.active = false;
        self.runner.suspend_native_visual_requests();
        self.runner.set_native_window_visibility(false);
    }

    pub(super) fn show(&mut self) {
        if !self.is_admitted() {
            return;
        }
        let was_inactive = !self.active;
        self.active = true;
        if !self.runner.resume_native_visual_requests() {
            self.active = false;
            return;
        }
        self.runner.set_native_window_visibility(true);
        if let Some(window) = self.runner.window.window.as_ref() {
            window.focus_window();
        }
        if was_inactive {
            self.runner.request_redraw_for_frame_work(FrameWork::None);
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn queue_accessibility_display_snapshot(
        &mut self,
        snapshot: super::window_environment::AccessibilityDisplaySnapshot,
    ) {
        if !self.is_admitted() {
            return;
        }
        self.runner.queue_accessibility_display_snapshot(snapshot);
    }

    fn begin_retiring(&mut self) {
        if !self.is_admitted() {
            return;
        }
        self.discard_frame_diagnostics();
        self.lifecycle = AuxiliaryNativeWindowLifecycle::Retiring;
        self.recovery_rebuild_pending = false;
        self.hide();
        let _ = self.runner.core.runtime.begin_closing();
    }

    pub(super) fn begin_whole_run_retiring(&mut self, event_loop: &ActiveEventLoop) {
        self.begin_retiring();
        let _ = event_loop;
    }

    fn handle_close_requested(
        &mut self,
        adapter_generation: Option<NativeAdapterGeneration>,
    ) -> AuxiliaryWindowEventResult<Message> {
        if self.is_retiring() {
            self.discard_frame_diagnostics();
            return AuxiliaryWindowEventResult::ignored();
        }
        if self.cache_on_close {
            self.hide();
            self.discard_frame_diagnostics();
            let messages = self.close_message.take().into_iter().collect();
            return self.event_result_with_messages(messages);
        }
        // Native auxiliary windows are initialized only after the parent has
        // selected a known adapter generation, so the production event route
        // supplies `Some(generation)`. `None` remains exact absent evidence
        // for whole-run terminal staging and unit fixtures; it cannot be an
        // OS close event for an uninitialized auxiliary.
        let Some(ticket) = self.admit_native_closing(adapter_generation) else {
            // The close message remains owned by the projection until the
            // parent accepts the exact child ticket.  A rejected admission is
            // therefore inert and can be retried by a later event.
            return AuxiliaryWindowEventResult::ignored();
        };
        AuxiliaryWindowEventResult {
            messages: Vec::new(),
            message_origin: None,
            close_admission: Some(AuxiliaryWindowCloseAdmission {
                owner: self.owner.clone(),
                ticket,
            }),
            terminal_cause: None,
            shutdown_requested: false,
            visual_deadline_completed: false,
            native_discrete_input_route: None,
            native_immediate_transient_route: None,
        }
    }

    #[cfg(test)]
    pub(super) fn stage_destructive_close_for_test(
        &mut self,
    ) -> AuxiliaryWindowEventResult<Message> {
        self.handle_close_requested(None)
    }

    fn event_result_with_messages(
        &mut self,
        messages: Vec<Message>,
    ) -> AuxiliaryWindowEventResult<Message> {
        AuxiliaryWindowEventResult {
            messages,
            message_origin: None,
            close_admission: None,
            terminal_cause: None,
            shutdown_requested: false,
            visual_deadline_completed: false,
            native_discrete_input_route: None,
            native_immediate_transient_route: None,
        }
    }

    pub(super) fn route_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: WindowEvent,
        adapter: &mut GenericNativeAdapterOwner,
        observation: Option<&mut CpuFrameObservationOwner<'_>>,
    ) -> AuxiliaryWindowEventResult<Message> {
        let mut observation = observation;
        if self.is_retiring() {
            self.discard_frame_diagnostics();
            return AuxiliaryWindowEventResult::ignored();
        }
        let mut terminal_cause = None;
        let mut native_discrete_input_route = None;
        let mut native_immediate_transient_route = None;
        match event {
            WindowEvent::CloseRequested => {
                return self.handle_close_requested(adapter.capture_generation());
            }
            WindowEvent::Resized(size) => self.runner.resize_surface(size),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.runner.update_native_dpi_scale(scale_factor);
            }
            WindowEvent::Moved(_) => self.runner.observe_monitor_move(),
            WindowEvent::ThemeChanged(theme) => self.runner.observe_theme_change(Some(theme)),
            WindowEvent::Occluded(occluded) => self.runner.handle_surface_occlusion(occluded),
            WindowEvent::Focused(false) => {
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = adapter.capture_generation() else {
                    return self.event_result(None, false);
                };
                let wrapper_eligible =
                    self.native_discrete_input_wrapper_is_eligible(adapter_generation);
                let Some(ticket) = self.runner.begin_native_immediate_transient_event(
                    event_loop,
                    NativeImmediateTransientKind::Focused(false),
                    timestamp,
                    adapter_generation,
                    wrapper_eligible,
                ) else {
                    return self.event_result(None, false);
                };
                let Some(ticket) = self.runner.revalidate_native_immediate_transient(
                    ticket,
                    adapter_generation,
                    self.native_discrete_input_wrapper_is_eligible(adapter_generation),
                ) else {
                    return self.event_result(None, false);
                };
                let routed = self.runner.handle_focus_lost_before_external_drag();
                let launch_external_drag = self.runner.core.runtime.external_drag_armed();
                native_immediate_transient_route = Some(AuxiliaryNativeImmediateTransientRoute {
                    ticket,
                    kind: AuxiliaryNativeImmediateTransientRouteKind::Focused {
                        outcome: routed,
                        launch_external_drag,
                    },
                });
            }
            WindowEvent::Focused(true) => {
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = adapter.capture_generation() else {
                    return self.event_result(None, false);
                };
                let wrapper_eligible =
                    self.native_discrete_input_wrapper_is_eligible(adapter_generation);
                let Some(ticket) = self.runner.begin_native_immediate_transient_event(
                    event_loop,
                    NativeImmediateTransientKind::Focused(true),
                    timestamp,
                    adapter_generation,
                    wrapper_eligible,
                ) else {
                    return self.event_result(None, false);
                };
                let Some(ticket) = self.runner.revalidate_native_immediate_transient(
                    ticket,
                    adapter_generation,
                    self.native_discrete_input_wrapper_is_eligible(adapter_generation),
                ) else {
                    return self.event_result(None, false);
                };
                let routed = self.runner.handle_focus_regained_after_native_modal_loop();
                native_immediate_transient_route = Some(AuxiliaryNativeImmediateTransientRoute {
                    ticket,
                    kind: AuxiliaryNativeImmediateTransientRouteKind::Focused {
                        outcome: routed,
                        launch_external_drag: false,
                    },
                });
            }
            WindowEvent::CursorEntered { device_id } => {
                self.runner
                    .retain_native_mouse_device(device_id, Some(true));
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = adapter.capture_generation() else {
                    return self.event_result(None, false);
                };
                let wrapper_eligible =
                    self.native_discrete_input_wrapper_is_eligible(adapter_generation);
                let Some(ticket) = self.runner.begin_native_immediate_transient_event(
                    event_loop,
                    NativeImmediateTransientKind::CursorEntered,
                    timestamp,
                    adapter_generation,
                    wrapper_eligible,
                ) else {
                    return self.event_result(None, false);
                };
                let Some(ticket) = self.runner.revalidate_native_immediate_transient(
                    ticket,
                    adapter_generation,
                    self.native_discrete_input_wrapper_is_eligible(adapter_generation),
                ) else {
                    return self.event_result(None, false);
                };
                self.runner.handle_cursor_entered();
                native_immediate_transient_route = Some(AuxiliaryNativeImmediateTransientRoute {
                    ticket,
                    kind: AuxiliaryNativeImmediateTransientRouteKind::CursorEntered,
                });
            }
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                self.runner.retain_native_mouse_device(device_id, None);
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = adapter.capture_generation() else {
                    return self.event_result(None, false);
                };
                let wrapper_eligible =
                    self.native_discrete_input_wrapper_is_eligible(adapter_generation);
                let Some(ticket) = self.runner.begin_native_immediate_transient_event(
                    event_loop,
                    NativeImmediateTransientKind::CursorMoved,
                    timestamp,
                    adapter_generation,
                    wrapper_eligible,
                ) else {
                    return self.event_result(None, false);
                };
                let Some(ticket) = self.runner.revalidate_native_immediate_transient(
                    ticket,
                    adapter_generation,
                    self.native_discrete_input_wrapper_is_eligible(adapter_generation),
                ) else {
                    return self.event_result(None, false);
                };
                let route = self
                    .runner
                    .route_cursor_moved_with_timestamp(position, timestamp);
                native_immediate_transient_route = Some(AuxiliaryNativeImmediateTransientRoute {
                    ticket,
                    kind: AuxiliaryNativeImmediateTransientRouteKind::CursorMoved(route),
                });
            }
            WindowEvent::CursorLeft { device_id } => {
                self.runner
                    .retain_native_mouse_device(device_id, Some(false));
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = adapter.capture_generation() else {
                    return self.event_result(None, false);
                };
                let wrapper_eligible =
                    self.native_discrete_input_wrapper_is_eligible(adapter_generation);
                let Some(ticket) = self.runner.begin_native_immediate_transient_event(
                    event_loop,
                    NativeImmediateTransientKind::CursorLeft,
                    timestamp,
                    adapter_generation,
                    wrapper_eligible,
                ) else {
                    return self.event_result(None, false);
                };
                let Some(ticket) = self.runner.revalidate_native_immediate_transient(
                    ticket,
                    adapter_generation,
                    self.native_discrete_input_wrapper_is_eligible(adapter_generation),
                ) else {
                    return self.event_result(None, false);
                };
                let route = self.runner.route_cursor_left();
                native_immediate_transient_route = Some(AuxiliaryNativeImmediateTransientRoute {
                    ticket,
                    kind: AuxiliaryNativeImmediateTransientRouteKind::CursorLeft(route),
                });
            }
            WindowEvent::MouseInput {
                device_id,
                button,
                state,
            } => {
                self.runner.retain_native_mouse_device(device_id, None);
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = adapter.capture_generation() else {
                    return self.event_result(None, false);
                };
                let wrapper_eligible =
                    self.native_discrete_input_wrapper_is_eligible(adapter_generation);
                let Some(ticket) = self.runner.begin_native_discrete_input_event(
                    event_loop,
                    NativeDiscreteInputKind::MouseInput,
                    timestamp,
                    adapter_generation,
                    wrapper_eligible,
                ) else {
                    return self.event_result(None, false);
                };
                let route = self.runner.route_native_mouse_input_with_timestamp(
                    button,
                    state,
                    Some(timestamp),
                );
                native_discrete_input_route = Some(AuxiliaryNativeDiscreteInputRoute {
                    ticket,
                    outcome: Some(route.outcome),
                });
            }
            WindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
            } => {
                self.runner.retain_native_mouse_device(device_id, None);
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = adapter.capture_generation() else {
                    return self.event_result(None, false);
                };
                let wrapper_eligible =
                    self.native_discrete_input_wrapper_is_eligible(adapter_generation);
                let Some(ticket) = self.runner.begin_native_immediate_transient_event(
                    event_loop,
                    NativeImmediateTransientKind::MouseWheel(phase),
                    timestamp,
                    adapter_generation,
                    wrapper_eligible,
                ) else {
                    return self.event_result(None, false);
                };
                let Some(ticket) = self.runner.revalidate_native_immediate_transient(
                    ticket,
                    adapter_generation,
                    self.native_discrete_input_wrapper_is_eligible(adapter_generation),
                ) else {
                    return self.event_result(None, false);
                };
                let route = self
                    .runner
                    .route_native_mouse_wheel_with_phase_and_timestamp(delta, phase, timestamp);
                native_immediate_transient_route = Some(AuxiliaryNativeImmediateTransientRoute {
                    ticket,
                    kind: AuxiliaryNativeImmediateTransientRouteKind::MouseWheel(route),
                });
            }
            WindowEvent::Touch(touch) => {
                self.runner
                    .normalize_native_touch_transient(event_loop, touch);
                return self.event_result(None, false);
            }
            WindowEvent::PinchGesture {
                device_id,
                delta,
                phase,
            } => {
                self.runner.normalize_native_gesture_transient(
                    event_loop,
                    NativeImmediateTransientKind::PinchGesture(phase),
                    device_id,
                    GestureInput::Pinch { delta, phase },
                );
                return self.event_result(None, false);
            }
            WindowEvent::RotationGesture {
                device_id,
                delta,
                phase,
            } => {
                self.runner.normalize_native_gesture_transient(
                    event_loop,
                    NativeImmediateTransientKind::RotationGesture(phase),
                    device_id,
                    GestureInput::Rotate {
                        delta_degrees: delta,
                        phase,
                    },
                );
                return self.event_result(None, false);
            }
            WindowEvent::PanGesture {
                device_id,
                delta,
                phase,
            } => {
                self.runner.normalize_native_gesture_transient(
                    event_loop,
                    NativeImmediateTransientKind::DesktopPanUnsupported(phase),
                    device_id,
                    GestureInput::Pan { delta, phase },
                );
                return self.event_result(None, false);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let wrapper_eligible = adapter.capture_generation().is_some_and(|generation| {
                    self.native_discrete_input_wrapper_is_eligible(generation)
                });
                native_discrete_input_route = self
                    .runner
                    .route_keyboard_event_with_adapter(
                        event_loop,
                        event,
                        adapter,
                        observation.as_deref_mut(),
                        wrapper_eligible,
                    )
                    .map(|(ticket, outcome)| AuxiliaryNativeDiscreteInputRoute { ticket, outcome });
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = adapter.capture_generation() else {
                    return self.event_result(None, false);
                };
                let wrapper_eligible =
                    self.native_discrete_input_wrapper_is_eligible(adapter_generation);
                let Some(ticket) = self.runner.begin_native_discrete_input_event(
                    event_loop,
                    NativeDiscreteInputKind::ModifiersChanged,
                    timestamp,
                    adapter_generation,
                    wrapper_eligible,
                ) else {
                    return self.event_result(None, false);
                };
                let routed = self.runner.route_native_modifiers_changed_with_timestamp(
                    modifiers.state(),
                    Some(timestamp),
                );
                native_discrete_input_route = Some(AuxiliaryNativeDiscreteInputRoute {
                    ticket,
                    outcome: Some(routed),
                });
            }
            WindowEvent::Ime(ime) => {
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = adapter.capture_generation() else {
                    return self.event_result(None, false);
                };
                let wrapper_eligible =
                    self.native_discrete_input_wrapper_is_eligible(adapter_generation);
                let Some(ticket) = self.runner.begin_native_discrete_input_event(
                    event_loop,
                    NativeDiscreteInputKind::Ime,
                    timestamp,
                    adapter_generation,
                    wrapper_eligible,
                ) else {
                    return self.event_result(None, false);
                };
                let routed = self
                    .runner
                    .route_native_ime_event_with_timestamp(ime, Some(timestamp));
                native_discrete_input_route = Some(AuxiliaryNativeDiscreteInputRoute {
                    ticket,
                    outcome: Some(routed),
                });
            }
            WindowEvent::RedrawRequested => {
                if !self.active || !self.is_admitted() {
                    self.runner.suspend_native_visual_requests();
                    return self.event_result(None, false);
                }
                let packet = match self.runner.begin_native_visual_request(adapter) {
                    NativeVisualRequestBegin::Requested(packet) => Some((packet, true)),
                    NativeVisualRequestBegin::UnsolicitedFallback(packet) => Some((packet, false)),
                    NativeVisualRequestBegin::Stale => {
                        self.runner.clear_native_visual_request_wake();
                        None
                    }
                    NativeVisualRequestBegin::RequestedVetoed
                    | NativeVisualRequestBegin::WrongWindow
                    | NativeVisualRequestBegin::Ineligible
                    | NativeVisualRequestBegin::Exhausted => None,
                };
                if let Some((packet, requested_packet)) = packet {
                    let packet_identity = packet.identity();
                    let admission = observation.as_deref_mut().map(|owner| {
                        self.runner
                            .begin_cpu_frame_observation_with_owner(owner, Instant::now())
                    });
                    let redraw_result =
                        self.runner
                            .redraw(event_loop, adapter, requested_packet, packet_identity);
                    let (disposition, redraw_failed) = match redraw_result {
                        Ok(disposition) => (disposition, false),
                        Err(failure) => {
                            self.runner.mark_cpu_frame_observation_recovery();
                            let kind = NativeRendererRecoveryWindowKind::Auxiliary {
                                requested_backend: self.runner.options.gpu.backend,
                            };
                            terminal_cause = self
                                .runner
                                .recover_frame_render_failure(event_loop, adapter, failure, kind)
                                .err();
                            (NativeVisualRequestDisposition::DropPacket, true)
                        }
                    };
                    let _ = self
                        .runner
                        .finish_native_visual_request(packet, disposition);
                    if let (Some(owner), Some(admission)) = (observation, admission) {
                        self.runner.finish_cpu_frame_observation_with_owner(
                            owner,
                            admission,
                            redraw_failed,
                        );
                    }
                }
            }
            _ => {}
        }
        let terminal_cause = terminal_cause.or_else(|| self.runner.take_terminal_cause());
        self.event_result_with_native_routes(
            terminal_cause,
            false,
            native_discrete_input_route,
            native_immediate_transient_route,
        )
    }

    fn take_messages(&mut self) -> Vec<Message> {
        self.runner.core.runtime.bridge_mut().take_messages()
    }
}

#[cfg(test)]
fn auxiliary_redraw_terminal_cause(
    redraw_result: Result<(), NativeFrameRenderFailure>,
) -> Option<NativeGenericRunError> {
    redraw_result
        .err()
        .map(NativeFrameRenderFailure::into_error)
}

pub(super) struct AuxiliaryWindowEventResult<Message> {
    pub(super) messages: Vec<Message>,
    pub(super) message_origin: Option<AuxiliaryWindowOwner>,
    pub(super) close_admission: Option<AuxiliaryWindowCloseAdmission>,
    pub(super) terminal_cause: Option<NativeGenericRunError>,
    pub(super) shutdown_requested: bool,
    pub(super) visual_deadline_completed: bool,
    pub(super) native_discrete_input_route: Option<AuxiliaryNativeDiscreteInputRoute>,
    pub(super) native_immediate_transient_route: Option<AuxiliaryNativeImmediateTransientRoute>,
}

pub(super) struct AuxiliaryWindowCloseAdmission {
    pub(super) owner: AuxiliaryWindowOwner,
    pub(super) ticket: NativeLifecycleStageTicket,
}

impl<Message> AuxiliaryWindowEventResult<Message> {
    fn ignored() -> Self {
        Self {
            messages: Vec::new(),
            message_origin: None,
            close_admission: None,
            terminal_cause: None,
            shutdown_requested: false,
            visual_deadline_completed: false,
            native_discrete_input_route: None,
            native_immediate_transient_route: None,
        }
    }
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn cancel_auxiliary_native_discrete_input_route(
        &mut self,
        index: usize,
        pending: Option<AuxiliaryNativeDiscreteInputRoute>,
    ) {
        if let Some(pending) = pending
            && let Some(window) = self.auxiliary_windows.get_mut(index)
        {
            let _ = window.cancel_native_discrete_input_route(pending);
        }
    }

    pub(super) fn cancel_auxiliary_native_immediate_transient_route(
        &mut self,
        index: usize,
        pending: Option<AuxiliaryNativeImmediateTransientRoute>,
    ) {
        if let Some(pending) = pending
            && let Some(window) = self.auxiliary_windows.get_mut(index)
        {
            let _ = window.cancel_native_immediate_transient_route(pending);
        }
    }

    fn reduce_auxiliary_messages(
        &mut self,
        message_origin: Option<AuxiliaryWindowOwner>,
        messages: Vec<Message>,
    ) -> GenericRouteOutcome {
        let mut outcome = GenericRouteOutcome::default();
        for message in messages {
            let command_outcome = match message_origin.as_ref() {
                Some(owner) => self
                    .core
                    .runtime
                    .dispatch_message_from_auxiliary(message, owner.clone()),
                None => self.core.runtime.dispatch_message(message),
            };
            outcome.merge(self.core.route_command_outcome(command_outcome));
        }
        outcome
    }

    fn apply_auxiliary_native_discrete_input_resolution(
        &mut self,
        outcome: GenericRouteOutcome,
        disposition: NativeInputStageDisposition,
        child_outcome: Option<GenericRouteOutcome>,
    ) -> GenericRouteOutcome {
        let exits =
            outcome.exit_requested || child_outcome.is_some_and(|outcome| outcome.exit_requested);
        if disposition == NativeInputStageDisposition::DeferLowerPriority && !exits {
            // This is the parent-owned boundary for an auxiliary input. Arm
            // it even when neither side returned visual work; the later
            // presentation boundary is the only consumer.
            self.defer_auxiliary_window_sync();
        }
        outcome.with_native_input_stage_disposition(disposition)
    }

    pub(super) fn dispatch_auxiliary_messages(
        &mut self,
        event_loop: &ActiveEventLoop,
        message_origin: Option<AuxiliaryWindowOwner>,
        messages: Vec<Message>,
        native_discrete_input_route: Option<(usize, AuxiliaryNativeDiscreteInputRoute)>,
        native_immediate_transient_route: Option<(usize, AuxiliaryNativeImmediateTransientRoute)>,
    ) {
        self.dispatch_auxiliary_messages_with_timed_frame(
            event_loop,
            message_origin,
            messages,
            native_discrete_input_route,
            native_immediate_transient_route,
            true,
        );
    }

    pub(super) fn dispatch_auxiliary_messages_without_timed_frame(
        &mut self,
        event_loop: &ActiveEventLoop,
        message_origin: Option<AuxiliaryWindowOwner>,
        messages: Vec<Message>,
        native_discrete_input_route: Option<(usize, AuxiliaryNativeDiscreteInputRoute)>,
        native_immediate_transient_route: Option<(usize, AuxiliaryNativeImmediateTransientRoute)>,
    ) {
        self.dispatch_auxiliary_messages_with_timed_frame(
            event_loop,
            message_origin,
            messages,
            native_discrete_input_route,
            native_immediate_transient_route,
            false,
        );
    }

    fn dispatch_auxiliary_messages_with_timed_frame(
        &mut self,
        event_loop: &ActiveEventLoop,
        message_origin: Option<AuxiliaryWindowOwner>,
        messages: Vec<Message>,
        native_discrete_input_route: Option<(usize, AuxiliaryNativeDiscreteInputRoute)>,
        native_immediate_transient_route: Option<(usize, AuxiliaryNativeImmediateTransientRoute)>,
        merge_due_timed_frame: bool,
    ) {
        if !self.should_admit_auxiliary_sync() {
            if let Some((index, pending)) = native_discrete_input_route {
                self.cancel_auxiliary_native_discrete_input_route(index, Some(pending));
            }
            if let Some((index, pending)) = native_immediate_transient_route {
                self.cancel_auxiliary_native_immediate_transient_route(index, Some(pending));
            }
            return;
        }
        let mut outcome = self.reduce_auxiliary_messages(message_origin, messages);

        if let Some((index, pending)) = native_discrete_input_route {
            let Some(resolution) =
                self.auxiliary_windows[index].resolve_native_discrete_input_route(pending)
            else {
                // The semantic route and parent message reduction already ran,
                // but an exact completion mismatch cannot authorize any
                // lower-stage child or parent work.
                return;
            };
            let AuxiliaryNativeDiscreteInputResolution {
                disposition,
                child_outcome,
            } = resolution;
            outcome = self.apply_auxiliary_native_discrete_input_resolution(
                outcome,
                disposition,
                child_outcome,
            );
            let Some(adapter) = self.adapter.as_mut() else {
                // The ticket is settled above. Without an adapter, suppress
                // both lower-stage outcomes rather than applying one side or
                // falling back to a replay.
                return;
            };
            if let Some(child_outcome) = child_outcome {
                self.auxiliary_windows[index].apply_native_discrete_input_route_with_adapter(
                    event_loop,
                    child_outcome,
                    adapter,
                );
            }
            if disposition == NativeInputStageDisposition::DeferLowerPriority {
                if merge_due_timed_frame {
                    self.handle_route_outcome(event_loop, outcome);
                } else {
                    self.handle_route_outcome_without_timed_frame(event_loop, outcome);
                }
                return;
            }
        }
        if let Some((index, pending)) = native_immediate_transient_route {
            let Some(resolution) =
                self.auxiliary_windows[index].resolve_native_immediate_transient_route(pending)
            else {
                // The semantic route and parent message reduction already
                // ran, but an exact completion mismatch cannot authorize any
                // lower-stage child or parent work.
                return;
            };
            let disposition = resolution.disposition;
            let child_outcome = match &resolution.child_route {
                AuxiliaryNativeImmediateTransientResolvedRoute::None => None,
                AuxiliaryNativeImmediateTransientResolvedRoute::Outcome(outcome) => Some(*outcome),
                AuxiliaryNativeImmediateTransientResolvedRoute::CursorMoved(route) => {
                    Some(route.outcome)
                }
                AuxiliaryNativeImmediateTransientResolvedRoute::MouseWheel(route) => {
                    Some(route.outcome)
                }
            };
            outcome = self.apply_auxiliary_native_discrete_input_resolution(
                outcome,
                disposition,
                child_outcome,
            );
            let Some(adapter) = self.adapter.as_mut() else {
                // The ticket is settled above. Without an adapter, suppress
                // both lower-stage outcomes rather than applying one side or
                // falling back to a replay.
                return;
            };
            self.auxiliary_windows[index].apply_native_immediate_transient_route_with_adapter(
                event_loop, resolution, adapter,
            );
            if disposition == NativeInputStageDisposition::DeferLowerPriority {
                if merge_due_timed_frame {
                    self.handle_route_outcome(event_loop, outcome);
                } else {
                    self.handle_route_outcome_without_timed_frame(event_loop, outcome);
                }
                return;
            }
        }
        if merge_due_timed_frame {
            self.handle_route_outcome(event_loop, outcome);
        } else {
            self.handle_route_outcome_without_timed_frame(event_loop, outcome);
        }
        if !self.should_admit_auxiliary_sync() {
            return;
        }
        if let Some(event_proxy) = self.runtime_wakeup.event_loop_proxy() {
            let _ = self.sync_auxiliary_windows(event_loop, event_proxy);
        }
    }

    pub(super) fn sync_auxiliary_windows(
        &mut self,
        event_loop: &ActiveEventLoop,
        event_proxy: EventLoopProxy<RuntimeUserEvent>,
    ) -> Result<(), NativeGenericRunError> {
        if !self.should_admit_auxiliary_sync() || self.timing.deferred_auxiliary_window_sync {
            return Ok(());
        }
        let mut maintenance = NativeResourceMaintenanceTurn::new();
        let Some(mut adapter) = self.adapter.take() else {
            return Err(NativeGenericRunError::NativeInitialization {
                stage: super::NativeInitializationStage::DeviceAcquisition,
                message: String::from("native adapter owner was not initialized"),
            });
        };
        let result = self.sync_auxiliary_windows_with_adapter_in_turn(
            event_loop,
            event_proxy,
            &mut adapter,
            &mut maintenance,
        );
        self.adapter = Some(adapter);
        result
    }

    pub(super) fn sync_auxiliary_windows_with_adapter(
        &mut self,
        event_loop: &ActiveEventLoop,
        event_proxy: EventLoopProxy<RuntimeUserEvent>,
        adapter: &mut GenericNativeAdapterOwner,
    ) -> Result<(), NativeGenericRunError> {
        if !self.should_admit_auxiliary_sync() || self.timing.deferred_auxiliary_window_sync {
            return Ok(());
        }
        let mut maintenance = NativeResourceMaintenanceTurn::new();
        self.sync_auxiliary_windows_with_adapter_in_turn(
            event_loop,
            event_proxy,
            adapter,
            &mut maintenance,
        )
    }

    pub(super) fn sync_auxiliary_windows_with_adapter_in_turn(
        &mut self,
        event_loop: &ActiveEventLoop,
        event_proxy: EventLoopProxy<RuntimeUserEvent>,
        adapter: &mut GenericNativeAdapterOwner,
        _maintenance: &mut NativeResourceMaintenanceTurn,
    ) -> Result<(), NativeGenericRunError> {
        let recovery_followup_pending = self.recovery_auxiliary_followup_pending;
        if !self.should_admit_auxiliary_sync() {
            return Ok(());
        }
        let retiring_keys_before_maintenance = self
            .auxiliary_windows
            .iter()
            .filter(|window| window.is_retiring())
            .map(|window| window.key().to_owned())
            .collect::<Vec<_>>();
        self.maintain_retiring_auxiliary_resources_with_adapter(_maintenance, Some(adapter));
        self.rearm_retiring_auxiliary_maintenance(Instant::now());
        let retired_keys_removed_this_sync = auxiliary_keys_removed_during_sync(
            &retiring_keys_before_maintenance,
            &self.auxiliary_windows,
        );
        let mut recovery_opportunity = AuxiliaryRecoveryOpportunity::default();
        if recovery_opportunity.admit_rebuild()
            && let Some(index) = self
                .auxiliary_windows
                .iter()
                .position(AuxiliaryNativeWindow::recovery_rebuild_pending)
        {
            let rebuild_result = self.auxiliary_windows[index]
                .rebuild_after_device_recovery(adapter, event_proxy.clone());
            if let Err(error) = rebuild_result {
                let cause = take_deferred_auxiliary_recovery_failure_cause(
                    &mut self.recovery_cause,
                    &mut self.recovery_auxiliary_followup_pending,
                    error,
                );
                self.admit_native_shutdown(event_loop, Some(cause));
                return Ok(());
            }
            self.timing.deferred_auxiliary_window_sync = true;
            self.request_redraw_for_frame_work(FrameWork::None);
            return Ok(());
        }
        let projections = self.core.runtime.host_project_auxiliary_windows();
        let command_service = self.core.runtime.command_service();
        for window in &mut self.auxiliary_windows {
            if window.is_admitted()
                && !auxiliary_projection_contains_key(&projections, window.key())
            {
                let owner = window.effect_owner();
                window.hide();
                self.core
                    .runtime
                    .discard_pending_auxiliary_focus_requests_for(&owner);
            }
        }
        for projection in projections {
            if let Some(window) = self
                .auxiliary_windows
                .iter_mut()
                .find(|window| window.is_admitted() && window.key() == projection.key)
            {
                let was_active = window.active;
                let owner = window.effect_owner();
                window.update_projection_with_commands(projection, command_service.clone());
                if !was_active {
                    self.core
                        .runtime
                        .discard_pending_auxiliary_focus_requests_for(&owner);
                }
            } else if auxiliary_key_is_suppressed_for_sync(
                &self.auxiliary_windows,
                &retired_keys_removed_this_sync,
                &projection.key,
            ) {
                // Keep the projection pending in application state, but do
                // not reactivate, recreate, or replay it while the older
                // generation-bound child is still retiring, or during the
                // same sync turn that removed that child. Removal and
                // recreation are separate sync boundaries.
                continue;
            } else {
                let native_window_diagnostic_identity =
                    self.allocate_auxiliary_window_diagnostic_identity();
                let owner = self
                    .core
                    .runtime
                    .acquire_auxiliary_effect_owner(&projection.key);
                let initialized = {
                    let parent_window = self.window.window.as_deref();
                    let mut window = AuxiliaryNativeWindow::new_with_owner(
                        projection,
                        &self.options,
                        native_window_diagnostic_identity,
                        self.frame_diagnostics_enabled,
                        self.core.has_frame_profile_observer(),
                        self.core.has_frame_gpu_timing_observer(),
                        owner.clone(),
                    );
                    window.runner.core.runtime.bridge_mut().command_service =
                        command_service.clone();
                    window.runner.native_ime_adapter_observer_enabled =
                        self.core.has_native_ime_adapter_observer();
                    window
                        .initialize_runtime(event_loop, parent_window, event_proxy.clone(), adapter)
                        .map(|()| window)
                };
                if let Err(error) =
                    self.append_initialized_auxiliary_window_with_ime_observation(initialized)
                {
                    self.core.runtime.retire_auxiliary_effect_owner(&owner);
                    if let Some(cause) = self.recovery_cause.take() {
                        self.recovery_auxiliary_followup_pending = false;
                        self.admit_native_shutdown(event_loop, Some(cause));
                    } else {
                        self.record_initialization_error_and_exit(event_loop, error.clone());
                    }
                    return Err(error);
                }
            }
        }
        if recovery_followup_pending {
            self.recovery_auxiliary_followup_pending = false;
            self.recovery_cause.take();
        }
        self.apply_pending_auxiliary_focus_requests(event_loop, adapter);
        Ok(())
    }

    fn append_initialized_auxiliary_window_with_ime_observation(
        &mut self,
        initialized: Result<AuxiliaryNativeWindow<Message>, NativeGenericRunError>,
    ) -> Result<(), NativeGenericRunError> {
        append_initialized_auxiliary_window(&mut self.auxiliary_windows, initialized)?;

        // A child can only reach this point after its native initialization
        // succeeded and the parent retained it as an admitted window. The
        // observer runs at this parent boundary, once, outside frame handoff.
        if let Some(observation) = self
            .auxiliary_windows
            .last_mut()
            .and_then(AuxiliaryNativeWindow::take_native_ime_adapter_observation)
        {
            self.core
                .runtime
                .host_observe_native_ime_adapter(observation);
        }
        Ok(())
    }

    fn auxiliary_focus_request_target_index(
        &self,
        request: &AuxiliaryFocusRequest,
    ) -> Option<usize> {
        if !self
            .core
            .runtime
            .auxiliary_effect_owner_is_active(request.owner())
        {
            return None;
        }

        let mut target = None;
        for (index, window) in self.auxiliary_windows.iter().enumerate() {
            if !window.can_apply_auxiliary_focus_request(request) {
                continue;
            }
            if target.replace(index).is_some() {
                return None;
            }
        }
        target
    }

    fn apply_pending_auxiliary_focus_requests(
        &mut self,
        event_loop: &ActiveEventLoop,
        adapter: &mut GenericNativeAdapterOwner,
    ) {
        self.for_each_pending_auxiliary_focus_request(|window, request| {
            window.apply_auxiliary_focus_request(event_loop, request, adapter);
        });
    }

    fn for_each_pending_auxiliary_focus_request(
        &mut self,
        mut apply: impl FnMut(&mut AuxiliaryNativeWindow<Message>, AuxiliaryFocusRequest),
    ) {
        let requests = self.core.runtime.take_pending_auxiliary_focus_requests();
        for request in requests {
            let Some(index) = self.auxiliary_focus_request_target_index(&request) else {
                continue;
            };
            let Some(window) = self.auxiliary_windows.get_mut(index) else {
                continue;
            };
            apply(window, request);
        }
    }

    pub(super) fn defer_auxiliary_window_sync(&mut self) {
        self.timing.deferred_auxiliary_window_sync = true;
    }

    pub(super) fn sync_deferred_auxiliary_windows_if_needed(
        &mut self,
        event_loop: &ActiveEventLoop,
        adapter: &mut GenericNativeAdapterOwner,
    ) {
        if self.timing.deferred_auxiliary_window_sync
            && self.should_admit_auxiliary_sync()
            && let Some(event_proxy) = self.runtime_wakeup.event_loop_proxy()
        {
            self.timing.deferred_auxiliary_window_sync = false;
            let _ = self.sync_auxiliary_windows_with_adapter(event_loop, event_proxy, adapter);
        }
    }
}

fn auxiliary_projection_contains_key<Message>(
    projections: &[AuxiliaryWindow<Message>],
    key: &str,
) -> bool {
    projections
        .iter()
        .any(|projection| projection.key.as_str() == key)
}

fn auxiliary_key_is_retiring<Message>(
    windows: &[AuxiliaryNativeWindow<Message>],
    key: &str,
) -> bool {
    windows
        .iter()
        .any(|window| window.is_retiring() && window.key() == key)
}

fn auxiliary_keys_removed_during_sync<Message>(
    retiring_keys_before_maintenance: &[String],
    windows_after_maintenance: &[AuxiliaryNativeWindow<Message>],
) -> Vec<String> {
    retiring_keys_before_maintenance
        .iter()
        .filter(|key| {
            !windows_after_maintenance
                .iter()
                .any(|window| window.key() == *key)
        })
        .cloned()
        .collect()
}

fn auxiliary_key_is_suppressed_for_sync<Message>(
    windows: &[AuxiliaryNativeWindow<Message>],
    removed_keys: &[String],
    key: &str,
) -> bool {
    auxiliary_key_is_retiring(windows, key) || removed_keys.iter().any(|removed| removed == key)
}

fn append_initialized_auxiliary_window<T>(
    windows: &mut Vec<T>,
    initialized: Result<T, NativeGenericRunError>,
) -> Result<(), NativeGenericRunError> {
    let window = initialized?;
    windows.push(window);
    Ok(())
}

fn take_deferred_auxiliary_recovery_failure_cause(
    recovery_cause: &mut Option<NativeGenericRunError>,
    recovery_auxiliary_followup_pending: &mut bool,
    auxiliary_error: NativeGenericRunError,
) -> NativeGenericRunError {
    *recovery_auxiliary_followup_pending = false;
    recovery_cause.take().unwrap_or(auxiliary_error)
}

#[derive(Default)]
struct AuxiliaryRecoveryOpportunity {
    rebuilds: u8,
}

impl AuxiliaryRecoveryOpportunity {
    fn admit_rebuild(&mut self) -> bool {
        if self.rebuilds != 0 {
            return false;
        }
        self.rebuilds = 1;
        true
    }

    #[cfg(test)]
    const fn rebuilds(&self) -> u8 {
        self.rebuilds
    }
}

#[cfg(test)]
#[path = "auxiliary/tests.rs"]
mod tests;
