#[cfg(test)]
use super::native_lifecycle_stage::NativeLifecycleStageEvidence;
use super::native_lifecycle_stage::NativeLifecycleStageTicket;
use super::native_visual_packet::{NativeVisualRequestBegin, NativeVisualRequestDisposition};
use super::renderer_recovery::NativeRendererRecoveryWindowKind;
use super::runner_state::{NativeWindowDiagnosticIdentityAllocator, NativeWindowResourceBundle};
#[cfg(test)]
use super::scene_texture::NativeFrameRenderFailure;
use super::{
    AuxiliaryScheduleEligibility, CpuFrameObservationCapture, CpuFrameObservationOwner,
    FrameScheduleDemand, FrameScheduleKey, FrameScheduleRedrawEvidence, FrameWork, FrameWorkReason,
    GenericNativeAdapterOwner, GenericNativeVelloRunner, GenericRouteOutcome,
    NativeAdapterGeneration, NativeGenericRunError, NativeResourceMaintenanceTurn,
    RuntimeUserEvent, SceneRebuildMode, initial_viewport, owner_window_handle,
};
use crate::gui_runtime::native_vello::{select_present_mode, startup_renderer_options};
#[cfg(test)]
use crate::runtime::{
    AuxiliaryWindow, NativeFrameDiagnostics, NativeRunOptions, NativeWindowDiagnosticIdentity,
    RuntimeBridge,
};
#[cfg(not(test))]
use crate::runtime::{
    AuxiliaryWindow, NativeRunOptions, NativeWindowDiagnosticIdentity, RuntimeBridge,
};
use crate::runtime::{AuxiliaryWindowOwner, RuntimeAnimationActivity};
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
mod placement;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AuxiliaryNativeWindowLifecycle {
    Admitted,
    Retiring,
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
            owner,
        )
    }

    pub(super) fn new_with_owner(
        projection: AuxiliaryWindow<Message>,
        parent_options: &NativeRunOptions,
        native_window_diagnostic_identity: Option<NativeWindowDiagnosticIdentity>,
        frame_diagnostics_enabled: bool,
        frame_profile_host_enabled: bool,
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
        let bridge = AuxiliarySurfaceBridge::new(
            projection.surface,
            frame_diagnostics_enabled,
            frame_profile_enabled,
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
        }
    }

    pub(super) fn key(&self) -> &str {
        &self.key
    }

    pub(super) fn effect_owner(&self) -> AuxiliaryWindowOwner {
        self.owner.clone()
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

    pub(super) fn record_native_interactive_arrival(&mut self, arrived_at: Instant) {
        self.runner.record_native_interactive_arrival(arrived_at);
    }

    fn event_result(
        &mut self,
        terminal_cause: Option<NativeGenericRunError>,
        visual_deadline_completed: bool,
    ) -> AuxiliaryWindowEventResult<Message> {
        AuxiliaryWindowEventResult {
            messages: self.take_messages(),
            message_origin: Some(self.owner.clone()),
            terminal_cause,
            shutdown_requested: self.runner.native_shutdown_requested(),
            visual_deadline_completed,
        }
    }

    pub(super) fn is_admitted(&self) -> bool {
        matches!(self.lifecycle, AuxiliaryNativeWindowLifecycle::Admitted)
    }

    pub(super) fn is_retiring(&self) -> bool {
        matches!(self.lifecycle, AuxiliaryNativeWindowLifecycle::Retiring)
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
    pub(super) fn begin_controller_closing_for_test(&mut self) -> bool {
        self.runner.core.runtime.begin_closing()
    }

    pub(super) fn quarantine_device_recovery_resources(&mut self) -> bool {
        if !self.is_admitted() || !self.recovery_rebuild_pending {
            return true;
        }
        self.runner.window.quarantine_active_native_resources()
    }

    pub(super) fn finish_device_recovery_if_no_rebuild(&mut self) -> bool {
        if !self.is_admitted() || self.recovery_rebuild_pending {
            return true;
        }
        self.runner.finish_device_recovery()
    }

    pub(super) fn rebuild_after_device_recovery(
        &mut self,
        adapter: &GenericNativeAdapterOwner,
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
    ) -> bool {
        if self.is_retiring() {
            return self.runner.retire_native_resources_with_turn(turn);
        }
        self.runner.maintain_native_resources_with_turn(turn);
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
        adapter: &GenericNativeAdapterOwner,
        now: Instant,
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
        self.runner.admit_native_resource_maintenance(
            now,
            &FrameScheduleKey::Auxiliary(self.key.clone()),
            parent_generation,
        )
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

    pub(super) fn update_projection(&mut self, projection: AuxiliaryWindow<Message>) {
        if !self.is_admitted() || self.recovery_rebuild_pending {
            return;
        }
        self.cache_on_close = projection.caches_on_close();
        self.close_message = projection.close_message;
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

    fn handle_close_requested(&mut self) -> AuxiliaryWindowEventResult<Message> {
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
        self.begin_retiring();
        self.discard_frame_diagnostics();
        let messages = self.close_message.take().into_iter().collect();
        self.event_result_with_messages(messages)
    }

    fn event_result_with_messages(
        &mut self,
        messages: Vec<Message>,
    ) -> AuxiliaryWindowEventResult<Message> {
        AuxiliaryWindowEventResult {
            messages,
            message_origin: None,
            terminal_cause: None,
            shutdown_requested: false,
            visual_deadline_completed: false,
        }
    }

    pub(super) fn route_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: WindowEvent,
        adapter: &mut GenericNativeAdapterOwner,
        mut observation: Option<&mut CpuFrameObservationOwner<'_>>,
    ) -> AuxiliaryWindowEventResult<Message> {
        if self.is_retiring() {
            self.discard_frame_diagnostics();
            return AuxiliaryWindowEventResult::ignored();
        }
        let mut terminal_cause = None;
        match event {
            WindowEvent::CloseRequested => return self.handle_close_requested(),
            WindowEvent::Resized(size) => self.runner.resize_surface(size),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.runner.update_native_dpi_scale(scale_factor);
            }
            WindowEvent::Moved(_) => self.runner.observe_monitor_move(),
            WindowEvent::ThemeChanged(theme) => self.runner.observe_theme_change(Some(theme)),
            WindowEvent::Focused(false) => {
                let routed = self.runner.handle_focus_lost_before_external_drag();
                self.runner.handle_route_outcome_with_adapter(
                    event_loop,
                    routed,
                    adapter,
                    observation.as_deref_mut(),
                );
                if self.runner.core.runtime.external_drag_armed() {
                    let outcome = self.runner.launch_external_drag_if_armed();
                    self.runner.handle_route_outcome_with_adapter(
                        event_loop,
                        outcome,
                        adapter,
                        observation.as_deref_mut(),
                    );
                }
            }
            WindowEvent::Focused(true) => {
                let routed = self.runner.handle_focus_regained_after_native_modal_loop();
                self.runner.handle_route_outcome_with_adapter(
                    event_loop,
                    routed,
                    adapter,
                    observation,
                );
            }
            WindowEvent::CursorEntered { .. } => self.runner.handle_cursor_entered(),
            WindowEvent::CursorMoved { position, .. } => self.runner.handle_cursor_moved(position),
            WindowEvent::CursorLeft { .. } => self.runner.handle_cursor_left(event_loop),
            WindowEvent::MouseInput { button, state, .. } => {
                let route = self.runner.route_native_mouse_input(button, state);
                self.runner.handle_route_outcome_with_adapter(
                    event_loop,
                    route.outcome,
                    adapter,
                    observation.as_deref_mut(),
                );
            }
            WindowEvent::MouseWheel { delta, phase, .. } => {
                let route = self
                    .runner
                    .route_native_mouse_wheel_with_phase(delta, phase);
                self.runner.handle_route_outcome_with_adapter(
                    event_loop,
                    route.outcome,
                    adapter,
                    observation.as_deref_mut(),
                );
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.runner.handle_keyboard_event_with_adapter(
                    event_loop,
                    event,
                    adapter,
                    observation.as_deref_mut(),
                )
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let routed = self
                    .runner
                    .route_native_modifiers_changed(modifiers.state());
                self.runner.handle_route_outcome_with_adapter(
                    event_loop,
                    routed,
                    adapter,
                    observation,
                );
            }
            WindowEvent::Ime(ime) => {
                let routed = self.runner.route_native_ime_event(ime);
                self.runner.handle_route_outcome_with_adapter(
                    event_loop,
                    routed,
                    adapter,
                    observation,
                );
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
        self.event_result(terminal_cause, false)
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
    pub(super) terminal_cause: Option<NativeGenericRunError>,
    pub(super) shutdown_requested: bool,
    pub(super) visual_deadline_completed: bool,
}

impl<Message> AuxiliaryWindowEventResult<Message> {
    fn ignored() -> Self {
        Self {
            messages: Vec::new(),
            message_origin: None,
            terminal_cause: None,
            shutdown_requested: false,
            visual_deadline_completed: false,
        }
    }
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn dispatch_auxiliary_messages(
        &mut self,
        event_loop: &ActiveEventLoop,
        message_origin: Option<AuxiliaryWindowOwner>,
        messages: Vec<Message>,
    ) {
        self.dispatch_auxiliary_messages_with_timed_frame(
            event_loop,
            message_origin,
            messages,
            true,
        );
    }

    pub(super) fn dispatch_auxiliary_messages_without_timed_frame(
        &mut self,
        event_loop: &ActiveEventLoop,
        message_origin: Option<AuxiliaryWindowOwner>,
        messages: Vec<Message>,
    ) {
        self.dispatch_auxiliary_messages_with_timed_frame(
            event_loop,
            message_origin,
            messages,
            false,
        );
    }

    fn dispatch_auxiliary_messages_with_timed_frame(
        &mut self,
        event_loop: &ActiveEventLoop,
        message_origin: Option<AuxiliaryWindowOwner>,
        messages: Vec<Message>,
        merge_due_timed_frame: bool,
    ) {
        if !self.should_admit_auxiliary_sync() {
            return;
        }
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
        if !self.should_admit_auxiliary_sync() {
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
        if !self.should_admit_auxiliary_sync() {
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
        self.timing.deferred_auxiliary_window_sync = false;
        if !self.should_admit_auxiliary_sync() {
            return Ok(());
        }
        self.maintain_retiring_auxiliary_resources_with_turn(_maintenance);
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
        for window in &mut self.auxiliary_windows {
            if window.is_admitted()
                && !auxiliary_projection_contains_key(&projections, window.key())
            {
                window.hide();
            }
        }
        for projection in projections {
            if let Some(window) = self
                .auxiliary_windows
                .iter_mut()
                .find(|window| window.is_admitted() && window.key() == projection.key)
            {
                window.update_projection(projection);
            } else if auxiliary_key_is_retiring(&self.auxiliary_windows, &projection.key) {
                // Keep the projection pending in application state, but do
                // not reactivate, recreate, or replay it while the older
                // generation-bound child is still retiring.
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
                        owner.clone(),
                    );
                    window
                        .initialize_runtime(event_loop, parent_window, event_proxy.clone(), adapter)
                        .map(|()| window)
                };
                if let Err(error) =
                    append_initialized_auxiliary_window(&mut self.auxiliary_windows, initialized)
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
        Ok(())
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
mod tests {
    use super::super::runner_state::NativeTargetGeneration;
    use super::super::{NativeLifecycle, native_lifecycle_stage};
    use super::{
        AuxiliaryNativeWindow, AuxiliaryRecoveryOpportunity, AuxiliarySurfaceBridge,
        AuxiliaryWindowEventResult, FrameScheduleKey, FrameWork, FrameWorkReason,
        GenericNativeVelloRunner, NativeAdapterGeneration, NativeFrameRenderFailure,
        NativeResourceMaintenanceTurn, SceneRebuildMode, append_initialized_auxiliary_window,
        auxiliary_key_is_retiring, auxiliary_projection_contains_key,
        auxiliary_redraw_terminal_cause, take_deferred_auxiliary_recovery_failure_cause,
    };
    use crate::gui::types::Vector2;
    use crate::{
        application::empty,
        gui_runtime::NativeRunOptions,
        prelude::IntoView,
        runtime::{
            AuxiliaryWindow, NativeFrameDiagnostics, NativeWindowDiagnosticIdentity,
            RuntimeFrameDiagnosticsHost,
        },
    };
    use native_lifecycle_stage::{NativeLifecycleStageEvidence, NativeLifecycleTransitionKind};
    use std::sync::Arc;
    use winit::window::WindowId;

    fn auxiliary_window_with_diagnostics(
        cache_on_close: bool,
        frame_diagnostics_enabled: bool,
    ) -> AuxiliaryNativeWindow<i32> {
        let surface = crate::runtime::test_arc_surface(empty::<i32>().into_surface());
        let projection =
            AuxiliaryWindow::new("settings", NativeRunOptions::default(), surface).on_close(7);
        let projection = if cache_on_close {
            projection.cache_on_close()
        } else {
            projection
        };
        AuxiliaryNativeWindow::new(
            projection,
            &NativeRunOptions::default(),
            Some(NativeWindowDiagnosticIdentity::from_runtime_value(2)),
            frame_diagnostics_enabled,
            false,
        )
    }

    fn auxiliary_window(cache_on_close: bool) -> AuxiliaryNativeWindow<i32> {
        auxiliary_window_with_diagnostics(cache_on_close, false)
    }

    #[test]
    fn constructed_auxiliary_runner_owns_its_exact_schedule_key() {
        let window = auxiliary_window(false);

        assert_eq!(
            window.runner.frame_stage_owner.key(),
            &FrameScheduleKey::Auxiliary(String::from("settings"))
        );
    }

    #[test]
    fn auxiliary_projection_key_lookup_uses_projected_windows_without_key_clones() {
        let surface = crate::runtime::test_arc_surface(empty::<()>().into_surface());
        let projections = vec![
            AuxiliaryWindow::new(
                "settings",
                crate::gui_runtime::NativeRunOptions::default(),
                Arc::clone(&surface),
            ),
            AuxiliaryWindow::new(
                "inspector",
                crate::gui_runtime::NativeRunOptions::default(),
                surface,
            ),
        ];

        assert!(auxiliary_projection_contains_key(&projections, "settings"));
        assert!(auxiliary_projection_contains_key(&projections, "inspector"));
        assert!(!auxiliary_projection_contains_key(&projections, "mixer"));
    }

    #[test]
    fn failed_auxiliary_initialization_propagates_without_appending_child() {
        let failure = crate::gui_runtime::NativeGenericRunError::NativeInitialization {
            stage: crate::gui_runtime::NativeInitializationStage::RendererCreation,
            message: String::from("renderer rejected device"),
        };
        let mut windows = vec![String::from("existing")];

        assert_eq!(
            append_initialized_auxiliary_window(&mut windows, Err(failure.clone())),
            Err(failure)
        );
        assert_eq!(windows, [String::from("existing")]);

        assert_eq!(
            append_initialized_auxiliary_window(&mut windows, Ok(String::from("ready"))),
            Ok(())
        );
        assert_eq!(windows, [String::from("existing"), String::from("ready")]);
    }

    #[test]
    fn auxiliary_redraw_failure_crosses_the_child_event_boundary() {
        let failure = NativeFrameRenderFailure::from_message("backend rejected scene");

        assert_eq!(
            auxiliary_redraw_terminal_cause(Err(failure)),
            Some(crate::gui_runtime::NativeGenericRunError::FrameRender(
                String::from("backend rejected scene"),
            ))
        );
        assert_eq!(auxiliary_redraw_terminal_cause(Ok(())), None);
    }

    #[test]
    fn destructive_close_enters_retiring_and_consumes_its_message_once() {
        let mut window = auxiliary_window(false);

        let first = window.handle_close_requested();
        assert_eq!(first.messages, [7]);
        assert!(first.message_origin.is_none());
        assert!(window.is_retiring());
        assert!(!window.active);
        assert!(window.window_id().is_none());
        assert!(!window.runner.core.runtime.begin_closing());

        let unrelated = auxiliary_window(true);
        assert!(auxiliary_key_is_retiring(
            std::slice::from_ref(&window),
            "settings"
        ));
        assert!(!auxiliary_key_is_retiring(
            std::slice::from_ref(&unrelated),
            "mixer"
        ));

        let duplicate = window.handle_close_requested();
        assert_eq!(duplicate.messages, Vec::<i32>::new());
        assert!(duplicate.terminal_cause.is_none());

        let late = AuxiliaryWindowEventResult::<i32>::ignored();
        assert!(late.messages.is_empty());
        assert!(late.terminal_cause.is_none());
        assert!(!late.shutdown_requested);
    }

    #[test]
    fn child_outbox_messages_carry_the_auxiliary_generation_owner() {
        let mut window = auxiliary_window(false);
        let _ = window.runner.core.runtime.dispatch_message(17);
        let result = window.event_result(None, false);
        assert_eq!(result.messages, [17]);
        let owner = window.effect_owner();
        assert!(
            result
                .message_origin
                .is_some_and(|origin| origin.is_same_generation(&owner))
        );
    }

    #[test]
    fn whole_run_retirement_reuses_retiring_transition_without_dispatching_close_message() {
        let mut window = auxiliary_window_with_diagnostics(false, true);
        window.stage_frame_diagnostics_for_test(NativeFrameDiagnostics {
            window_identity: Some(NativeWindowDiagnosticIdentity::from_runtime_value(2)),
            frame_sequence: Some(13),
            ..NativeFrameDiagnostics::default()
        });

        window.begin_retiring();

        assert!(window.is_retiring());
        window.mark_parent_observation_finalized();
        assert_eq!(window.take_ready_frame_diagnostics(), None);
        let late_close = window.handle_close_requested();
        assert!(late_close.messages.is_empty());
        assert!(!late_close.shutdown_requested);
    }

    #[test]
    fn cached_close_hides_reuses_and_does_not_begin_closing() {
        let mut window = auxiliary_window(true);
        let owner = window.effect_owner();

        let close = window.handle_close_requested();
        assert_eq!(close.messages, [7]);
        assert!(!window.is_retiring());
        assert!(!window.active);

        let surface = crate::runtime::test_arc_surface(empty::<i32>().into_surface());
        window.update_projection(
            AuxiliaryWindow::new("settings", NativeRunOptions::default(), surface).cache_on_close(),
        );
        assert!(window.active);
        assert!(!window.is_retiring());
        assert!(window.effect_owner().is_same_generation(&owner));
        assert!(window.runner.core.runtime.begin_closing());

        let duplicate = window.handle_close_requested();
        assert!(duplicate.messages.is_empty());
    }

    #[test]
    fn cached_auxiliary_hide_recovery_and_show_rearm_mailbox_dormancy() {
        let mut window = auxiliary_window(true);

        window.hide();
        assert!(!window.active);
        assert!(window.runner.window.native_visual_requests.is_suspended());
        assert!(!window.runner.window.native_visual_requests.has_work());

        // A device/resource invalidation must not turn a cached inactive child
        // back into an unsolicited redraw source.
        assert!(window.runner.invalidate_native_visual_requests());
        window
            .runner
            .request_redraw_for_frame_work(FrameWork::RebuildScene {
                reason: FrameWorkReason::RuntimeSurfaceRepaint,
                mode: SceneRebuildMode::Immediate,
            });
        assert!(window.runner.window.native_visual_requests.is_suspended());
        assert!(!window.runner.window.native_visual_requests.has_work());

        // Repeated inactive callbacks reassert dormancy; explicit show is the
        // only rearm and leaves no stale packet to replay.
        window.hide();
        window.show();
        assert!(window.active);
        assert!(!window.runner.window.native_visual_requests.is_suspended());
        assert!(!window.runner.window.native_visual_requests.has_work());
    }

    #[test]
    fn auxiliary_active_and_inactive_recovery_preserve_dormancy_boundaries() {
        let mut active = auxiliary_window(true);
        active.runner.set_native_window_visibility(true);
        assert!(active.runner.window.logical_window_visible);
        assert!(active.admit_device_recovery());
        assert!(active.runner.window.logical_window_visible);
        assert!(active.finish_device_recovery_if_no_rebuild());
        active
            .runner
            .apply_native_window_visibility(active.runner.window.logical_window_visible);
        assert!(active.runner.window.logical_window_visible);

        let mut inactive = auxiliary_window(true);
        inactive.hide();
        assert!(inactive.runner.window.native_visual_requests.is_suspended());
        assert!(inactive.admit_device_recovery());
        assert!(inactive.finish_device_recovery_if_no_rebuild());
        assert!(!inactive.active);
        assert!(inactive.runner.window.native_visual_requests.is_suspended());
        assert!(!inactive.runner.window.native_visual_requests.has_work());
        inactive.show();
        assert!(inactive.active);
        assert!(!inactive.runner.window.native_visual_requests.is_suspended());
        assert!(!inactive.runner.window.native_visual_requests.has_work());
    }

    #[test]
    fn lazy_finish_veto_retains_recovering_and_rebuild_pending() {
        let mut window = auxiliary_window(true);
        let generation = NativeAdapterGeneration::from_test_serial(1);

        assert!(window.admit_device_recovery());
        window.recovery_rebuild_pending = true;
        let mut source_phase = NativeLifecycle::default();
        assert!(source_phase.admit_recovery());
        let evidence = NativeLifecycleStageEvidence {
            key: FrameScheduleKey::Auxiliary(String::from("settings")),
            transition: NativeLifecycleTransitionKind::FinishDeviceRecovery,
            source_phase,
            window_id: Some(WindowId::dummy()),
            adapter_generation: Some(generation),
            active_resource_generation: Some(generation),
            target_generation: NativeTargetGeneration::from_test_serial(2),
            target_fenced: false,
        };
        let ticket = window
            .admit_native_lifecycle_finish_with_evidence(evidence.clone())
            .expect("finish lifecycle ticket");

        assert!(window.native_lifecycle_ticket_is_current_with_evidence(&ticket, &evidence));
        assert!(window.veto_native_lifecycle(ticket));
        assert!(window.runner.is_recovering());
        assert!(window.recovery_rebuild_pending());
    }

    #[test]
    fn retiring_auxiliary_is_excluded_from_finish_lifecycle_admission() {
        let mut window = auxiliary_window(false);
        window.begin_retiring();

        assert!(
            window
                .admit_native_lifecycle_finish(Some(NativeAdapterGeneration::from_test_serial(1)))
                .is_none()
        );
    }

    #[test]
    fn terminal_closing_includes_retiring_child_and_skips_closing_child() {
        let mut retiring = auxiliary_window(false);
        retiring.begin_retiring();
        assert!(retiring.is_retiring());
        assert!(retiring.should_stage_native_closing());
        let ticket = retiring
            .admit_native_closing(None)
            .expect("retiring child closing ticket");
        assert!(retiring.veto_native_lifecycle(ticket));

        let mut closing = auxiliary_window(false);
        assert!(closing.runner.prepare_native_shutdown(None).is_some());
        assert!(!closing.should_stage_native_closing());
        assert!(closing.admit_native_closing(None).is_none());
    }

    #[test]
    fn terminal_convergence_invalidates_auxiliary_lifecycle_owner() {
        let mut auxiliary = auxiliary_window(false);
        let ticket = auxiliary
            .admit_native_closing(None)
            .expect("auxiliary terminal lifecycle ticket");
        let identity = ticket.stage_ticket().identity().clone();
        let owner_generation = auxiliary.runner.frame_stage_owner.owner_generation();
        assert!(auxiliary.runner.frame_stage_owner.has_in_flight());

        auxiliary.invalidate_terminal_convergence_stage_owner();

        assert!(!auxiliary.runner.frame_stage_owner.has_in_flight());
        assert!(auxiliary.runner.frame_stage_owner.owner_generation() > owner_generation);
        assert!(auxiliary.runner.frame_stage_owner.stale(&identity));
        assert!(
            !auxiliary
                .runner
                .native_lifecycle_stage_ticket_is_current(&ticket)
        );
        assert!(!auxiliary.veto_native_lifecycle(ticket));
    }

    #[test]
    fn auxiliary_parent_handoff_requires_finalization_and_admission() {
        let diagnostics = NativeFrameDiagnostics {
            window_identity: Some(NativeWindowDiagnosticIdentity::from_runtime_value(2)),
            frame_sequence: Some(13),
            ..NativeFrameDiagnostics::default()
        };
        let mut window = auxiliary_window_with_diagnostics(true, true);

        window
            .runner
            .core
            .runtime
            .bridge_mut()
            .observe_frame_diagnostics(diagnostics);
        assert_eq!(window.take_ready_frame_diagnostics(), None);

        window.mark_parent_observation_finalized();
        assert_eq!(window.take_ready_frame_diagnostics(), Some(diagnostics));

        window
            .runner
            .core
            .runtime
            .bridge_mut()
            .observe_frame_diagnostics(diagnostics);
        window.require_scheduled_frame_admission();
        window.mark_parent_observation_finalized();
        assert_eq!(window.take_ready_frame_diagnostics(), None);

        window.mark_scheduled_frame_admission_recorded();
        assert_eq!(window.take_ready_frame_diagnostics(), Some(diagnostics));

        window
            .runner
            .core
            .runtime
            .bridge_mut()
            .observe_frame_diagnostics(diagnostics);
        let close = window.handle_close_requested();
        assert_eq!(close.messages, [7]);
        assert_eq!(window.take_ready_frame_diagnostics(), None);
    }

    #[test]
    fn maintenance_removes_retiring_child_only_after_gpu_state_is_empty() {
        let surface = crate::runtime::test_arc_surface(empty::<i32>().into_surface());
        let mut parent = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            AuxiliarySurfaceBridge::new(surface, false, false),
            Vector2::new(1280.0, 720.0),
        );
        parent.auxiliary_windows.push(auxiliary_window(false));
        let child = parent
            .auxiliary_windows
            .last_mut()
            .expect("test parent should retain the auxiliary child");
        let close = child.handle_close_requested();
        assert_eq!(close.messages, [7]);

        let mut turn = NativeResourceMaintenanceTurn::new();
        assert!(parent.maintain_native_resources_with_turn(&mut turn));

        assert!(parent.auxiliary_windows.is_empty());
        assert!(parent.timing.deferred_auxiliary_window_sync);
        assert!(!turn.has_pending());
    }

    #[test]
    fn recovery_opportunity_admits_at_most_one_auxiliary_rebuild() {
        let mut opportunity = AuxiliaryRecoveryOpportunity::default();

        assert!(opportunity.admit_rebuild());
        assert!(!opportunity.admit_rebuild());
        assert_eq!(opportunity.rebuilds(), 1);
    }

    #[test]
    fn deferred_recovery_rebuilds_two_children_across_opportunities_and_clears_followup() {
        let mut pending_children = 2;
        let mut recovery_followup_pending = true;
        let mut total_rebuilds = 0;

        while pending_children != 0 {
            let mut opportunity = AuxiliaryRecoveryOpportunity::default();

            assert!(opportunity.admit_rebuild());
            assert!(!opportunity.admit_rebuild());
            pending_children -= usize::from(opportunity.rebuilds());
            total_rebuilds += usize::from(opportunity.rebuilds());

            if pending_children == 0 {
                recovery_followup_pending = false;
            } else {
                assert!(recovery_followup_pending);
            }
        }

        assert_eq!(total_rebuilds, 2);
        assert_eq!(pending_children, 0);
        assert!(!recovery_followup_pending);
    }

    #[test]
    fn deferred_auxiliary_rebuild_failure_preserves_render_device_loss_and_fences_followup() {
        let recovery_cause = crate::gui_runtime::NativeGenericRunError::RenderDeviceLost(
            String::from("driver reset"),
        );
        let auxiliary_error = crate::gui_runtime::NativeGenericRunError::NativeInitialization {
            stage: crate::gui_runtime::NativeInitializationStage::RendererCreation,
            message: String::from("auxiliary renderer rejected device"),
        };
        let mut retained_cause = Some(recovery_cause.clone());
        let mut followup_pending = true;

        assert_eq!(
            take_deferred_auxiliary_recovery_failure_cause(
                &mut retained_cause,
                &mut followup_pending,
                auxiliary_error,
            ),
            recovery_cause
        );
        assert!(retained_cause.is_none());
        assert!(!followup_pending);
    }

    #[test]
    fn deferred_auxiliary_rebuild_failure_falls_back_to_auxiliary_error_without_recovery_cause() {
        let auxiliary_error = crate::gui_runtime::NativeGenericRunError::NativeInitialization {
            stage: crate::gui_runtime::NativeInitializationStage::RenderSurfaceCreation,
            message: String::from("auxiliary surface rejected device"),
        };
        let mut retained_cause = None;
        let mut followup_pending = true;

        assert_eq!(
            take_deferred_auxiliary_recovery_failure_cause(
                &mut retained_cause,
                &mut followup_pending,
                auxiliary_error.clone(),
            ),
            auxiliary_error
        );
        assert!(retained_cause.is_none());
        assert!(!followup_pending);
    }
}
