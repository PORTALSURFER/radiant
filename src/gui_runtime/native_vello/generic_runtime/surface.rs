//! Window, surface, and renderer setup for the generic native Vello runner.

use super::native_visual_packet::NativeVisualRequestDisposition;
use super::runner_state::{
    NativeResourceMaintenanceTurn, NativeSurfaceAcquireFailure, NativeWindowResourceBundle,
    SurfaceAcquirePolicy, surface_acquire_policy,
};
use super::{
    FrameWork, FrameWorkReason, GenericNativeAdapterOwner, GenericNativeVelloRunner,
    NativeGenericRunError, NativeInitializationStage, NativeRenderDeviceErrorKind,
    RuntimeUserEvent, SceneRebuildMode, configure_created_top_level_window,
    generic_window_attributes, reveal_window_after_surface_setup,
};
use super::{
    accessibility,
    window_environment::{
        current_monitor_fingerprint, environment_for_native_state, window_color_scheme,
    },
};
use crate::{
    gui::types::Vector2,
    gui_runtime::NativeRunOptions,
    gui_runtime::native_vello::{select_present_mode, startup_renderer_options},
    runtime::RuntimeBridge,
    theme::DpiScale,
};
use std::{sync::Arc, time::Instant};
use tracing::{error, info, warn};
use vello::{Renderer, wgpu};
use winit::{
    dpi::PhysicalSize,
    event_loop::{ActiveEventLoop, EventLoopProxy},
};

mod backend;
mod viewport;

use viewport::{logical_viewport_for_size, surface_size_changed};

#[derive(Debug)]
pub(super) enum NativeSurfaceAcquireError {
    MissingResources,
    Surface(NativeSurfaceAcquireFailure),
}

fn map_current_surface_texture_error(
    texture: &wgpu::CurrentSurfaceTexture,
    uncaptured_error: Option<NativeRenderDeviceErrorKind>,
) -> Option<NativeSurfaceAcquireFailure> {
    match texture {
        wgpu::CurrentSurfaceTexture::Success(_) | wgpu::CurrentSurfaceTexture::Suboptimal(_) => {
            None
        }
        wgpu::CurrentSurfaceTexture::Timeout => Some(NativeSurfaceAcquireFailure::Timeout),
        wgpu::CurrentSurfaceTexture::Occluded => Some(NativeSurfaceAcquireFailure::Occluded),
        wgpu::CurrentSurfaceTexture::Outdated => Some(NativeSurfaceAcquireFailure::Outdated),
        wgpu::CurrentSurfaceTexture::Lost => Some(NativeSurfaceAcquireFailure::Lost),
        wgpu::CurrentSurfaceTexture::Validation => Some(
            if matches!(
                uncaptured_error,
                Some(NativeRenderDeviceErrorKind::OutOfMemory)
            ) {
                NativeSurfaceAcquireFailure::OutOfMemory
            } else {
                NativeSurfaceAcquireFailure::Other
            },
        ),
    }
}

pub(super) fn instance_for_options(options: &NativeRunOptions) -> wgpu::Instance {
    backend::instance_for_options(options)
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn initialize_runtime(
        &mut self,
        event_loop: &ActiveEventLoop,
        event_proxy: EventLoopProxy<RuntimeUserEvent>,
        maintenance: &mut NativeResourceMaintenanceTurn,
    ) -> Result<(), NativeGenericRunError> {
        if !self.is_running() {
            return Ok(());
        }
        let mut adapter = GenericNativeAdapterOwner::new(&self.options);
        self.initialize_window_runtime(event_loop, event_proxy.clone(), &mut adapter, true)?;
        self.adapter = Some(adapter);
        let Some(mut adapter) = self.adapter.take() else {
            return Err(NativeGenericRunError::NativeInitialization {
                stage: NativeInitializationStage::DeviceAcquisition,
                message: String::from("native adapter owner was not initialized"),
            });
        };
        let result = self.sync_auxiliary_windows_with_adapter_in_turn(
            event_loop,
            event_proxy,
            &mut adapter,
            maintenance,
        );
        self.adapter = Some(adapter);
        result
    }

    pub(super) fn initialize_runtime_with_adapter(
        &mut self,
        event_loop: &ActiveEventLoop,
        event_proxy: EventLoopProxy<RuntimeUserEvent>,
        adapter: &mut GenericNativeAdapterOwner,
    ) -> Result<(), NativeGenericRunError> {
        if !self.is_running() {
            return Ok(());
        }
        self.initialize_window_runtime(event_loop, event_proxy, adapter, false)
    }

    fn initialize_window_runtime(
        &mut self,
        event_loop: &ActiveEventLoop,
        event_proxy: EventLoopProxy<RuntimeUserEvent>,
        adapter: &mut GenericNativeAdapterOwner,
        primary: bool,
    ) -> Result<(), NativeGenericRunError> {
        if !self.window.can_publish_native_resources() {
            return Err(native_initialization_error(
                NativeInitializationStage::DeviceAcquisition,
                "native resource quarantine capacity is exhausted",
            ));
        }
        info!("radiant generic native vello: initializing runtime window and surface");
        self.timing.startup_timing.mark_init_started();
        let window = event_loop
            .create_window(generic_window_attributes(&self.options))
            .map(Arc::new)
            .map_err(|err| {
                native_initialization_error(NativeInitializationStage::WindowCreation, err)
            })?;
        configure_created_top_level_window(&window, &self.options);
        self.timing.startup_timing.mark_window_created();
        let candidate_native_dpi_scale = DpiScale::new(window.scale_factor());
        let candidate_dpi_scale = self
            .window
            .dpi_scale_override
            .unwrap_or(candidate_native_dpi_scale);
        let candidate_monitor_fingerprint = current_monitor_fingerprint(&window);
        let candidate_accessibility_display = accessibility::current_snapshot();
        let candidate_environment = environment_for_native_state(
            candidate_dpi_scale,
            window_color_scheme(window.theme()),
            candidate_accessibility_display,
        );
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        let candidate_viewport = logical_viewport_for_size(size, candidate_dpi_scale);
        let Some(native_resource_publication) = self.window.reserve_native_resource_publication()
        else {
            return Err(native_initialization_error(
                NativeInitializationStage::DeviceAcquisition,
                "native resource quarantine capacity is exhausted",
            ));
        };
        let surface = adapter
            .instance()
            .ok_or_else(|| {
                native_initialization_error(
                    NativeInitializationStage::WgpuSurfaceCreation,
                    "native adapter render context is unavailable",
                )
            })?
            .create_surface(window.clone())
            .map_err(|err| {
                native_initialization_error(NativeInitializationStage::WgpuSurfaceCreation, err)
            })?;
        self.timing.startup_timing.mark_wgpu_surface_created();
        if primary {
            adapter
                .select_primary_device(&surface, event_proxy.clone())
                .map_err(|err| {
                    native_initialization_error(NativeInitializationStage::DeviceAcquisition, err)
                })?;
        } else {
            adapter
                .validate_auxiliary_surface(self.options.gpu.backend, &surface)
                .map_err(|err| {
                    native_initialization_error(NativeInitializationStage::DeviceAcquisition, err)
                })?;
        }
        self.timing.startup_timing.mark_wgpu_device_ready();
        let generation = adapter.capture_generation().ok_or_else(|| {
            native_initialization_error(
                NativeInitializationStage::DeviceAcquisition,
                "native adapter has no current known generation",
            )
        })?;
        let supported_present_modes = surface
            .get_capabilities(
                adapter
                    .selected_device_handle()
                    .ok_or_else(|| {
                        native_initialization_error(
                            NativeInitializationStage::DeviceAcquisition,
                            "native adapter did not retain a selected device",
                        )
                    })?
                    .adapter(),
            )
            .present_modes;
        let present_mode = select_present_mode(
            self.options.normalized_target_fps(),
            &supported_present_modes,
        );
        let render_surface = adapter
            .create_render_surface(surface, width, height, present_mode)
            .map_err(|err| {
                native_initialization_error(NativeInitializationStage::RenderSurfaceCreation, err)
            })?;
        self.timing.startup_timing.mark_surface_ready();
        let dev_handle = adapter.selected_device_handle().ok_or_else(|| {
            native_initialization_error(
                NativeInitializationStage::DeviceAcquisition,
                "native adapter did not retain a selected device",
            )
        })?;
        self.timing.startup_timing.mark_renderer_started();
        let renderer =
            Renderer::new(&dev_handle.device, startup_renderer_options()).map_err(|err| {
                native_initialization_error(NativeInitializationStage::RendererCreation, err)
            })?;
        self.timing.startup_timing.mark_renderer_ready();
        if !adapter.admit_generation(generation) {
            return Err(native_initialization_error(
                NativeInitializationStage::DeviceAcquisition,
                "native adapter generation changed during window resource initialization",
            ));
        }
        let native_resources = NativeWindowResourceBundle::new(
            generation,
            render_surface,
            renderer,
            &dev_handle.device,
            &dev_handle.queue,
            event_proxy,
        )
        .ok_or_else(|| {
            native_initialization_error(
                NativeInitializationStage::DeviceAcquisition,
                "native window resources require a known adapter generation",
            )
        })?;
        native_resource_publication.publish(native_resources);
        self.window.id = Some(window.id());
        self.window.window = Some(Arc::clone(&window));
        if !self.window.native_visual_requests.bind_window(window.id()) {
            return Err(native_initialization_error(
                NativeInitializationStage::DeviceAcquisition,
                "native visual request mailbox could not bind the window identity",
            ));
        }
        self.sync_native_ime_allowed();
        self.window.native_dpi_scale = candidate_native_dpi_scale;
        self.window.dpi_scale = candidate_dpi_scale;
        self.window.monitor_fingerprint = candidate_monitor_fingerprint;
        self.window.accessibility_display = candidate_accessibility_display;
        self.window.environment = candidate_environment;
        if self
            .core
            .runtime
            .set_window_environment(candidate_environment)
        {
            // Startup is the one non-deferred environment transition: the
            // first scene must be projected with the native values already
            // known from the created window.
            self.core.refresh_surface();
        }
        self.core.set_viewport(candidate_viewport);
        self.window.target_generation.advance();
        self.frame.clear_native_paint_segment_artifacts();
        self.rebuild_scene();
        self.timing.startup_timing.mark_first_scene_ready();
        if reveal_window_after_surface_setup(&self.options) {
            self.reveal_prepared_window_at_activation_boundary();
        }
        self.timing.last_redraw = Instant::now();
        self.request_redraw_for_frame_work(FrameWork::RebuildScene {
            reason: FrameWorkReason::RuntimeSurfaceRepaint,
            mode: SceneRebuildMode::Immediate,
        });
        Ok(())
    }

    pub(super) fn resize_surface(&mut self, size: PhysicalSize<u32>) {
        if !self.is_running() {
            return;
        }
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.defer_surface_resize_with_reason(size, FrameWorkReason::NativeResize);
        self.request_redraw_for_frame_work(FrameWork::None);
    }

    pub(super) fn defer_surface_resize_with_reason(
        &mut self,
        size: PhysicalSize<u32>,
        reason: FrameWorkReason,
    ) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        if self.window.native_surface_target_fenced || !self.window.target_generation.is_known() {
            self.arm_requested_recovery_redraw();
        }
        self.timing.pending_surface_resize = Some(size);
        self.timing.pending_surface_resize_reason = Some(reason);
    }

    pub(super) fn apply_pending_surface_resize_if_needed(
        &mut self,
        adapter: &GenericNativeAdapterOwner,
    ) {
        if !self.admit_native_resources(adapter) {
            return;
        }
        let Some(size) = self.timing.pending_surface_resize.take() else {
            return;
        };
        let reason = self
            .timing
            .pending_surface_resize_reason
            .take()
            .unwrap_or(FrameWorkReason::NativeResize);
        let applied = self.resize_surface_now(size, false, reason, adapter);
        self.timing.surface_resize_applied_this_frame = applied;
        if applied {
            self.record_frame_work(FrameWork::ResizeSurface { reason });
        }
    }

    pub(super) fn resize_surface_now(
        &mut self,
        size: PhysicalSize<u32>,
        request_redraw: bool,
        reason: FrameWorkReason,
        adapter: &GenericNativeAdapterOwner,
    ) -> bool {
        if size.width == 0 || size.height == 0 {
            return false;
        }
        if !self.admit_native_resources(adapter) {
            return false;
        }
        self.timing.pending_surface_resize = None;
        self.timing.pending_surface_resize_reason = None;
        if let Some(resources) = self.window.native_resources.as_mut() {
            if !surface_size_changed(
                resources.render_surface.config.width,
                resources.render_surface.config.height,
                size,
            ) {
                return false;
            }
            if !adapter.resize_surface(&mut resources.render_surface, size.width, size.height) {
                return false;
            }
            self.complete_target_transition();
            self.defer_viewport_resize_with_reason(
                logical_viewport_for_size(size, self.window.dpi_scale),
                reason,
            );
            if request_redraw {
                self.request_redraw_for_frame_work(FrameWork::ResizeSurface { reason });
            }
            return true;
        }
        false
    }

    fn resize_surface_now_for_recovery(
        &mut self,
        size: PhysicalSize<u32>,
        adapter: &GenericNativeAdapterOwner,
    ) -> bool {
        if size.width == 0 || size.height == 0 {
            return false;
        }
        if !self.admit_native_resources(adapter) {
            return false;
        }
        self.timing.pending_surface_resize = None;
        self.timing.pending_surface_resize_reason = None;
        // A Lost/Outdated surface is a resource-failure boundary, not an
        // ordinary deferred resize. Fence the claimed packet before
        // reconfiguration so recovery cannot finish stale work.
        if self.window.native_resources.is_none() {
            return false;
        }
        self.fence_native_surface_target();
        if let Some(resources) = self.window.native_resources.as_mut() {
            if !adapter.resize_surface(&mut resources.render_surface, size.width, size.height) {
                return false;
            }
            self.complete_target_transition();
            return true;
        }
        false
    }

    fn complete_target_transition(&mut self) {
        // Ordinary deferred resize is applied after RedrawRequested claims its
        // packet. Advance only the physical target evidence here; the
        // target-unbound packet must survive to its normal finish boundary.
        self.fence_native_surface_target_for_transition();
        if self.window.target_generation.advance() {
            self.window.native_surface_target_fenced = false;
        }
        self.window.surface_recovery.rearm_transient_retry();
    }

    pub(super) fn complete_native_recovery_target_transition(&mut self) {
        self.complete_target_transition();
    }

    /// Fence packet ownership for a true surface/resource or lifecycle
    /// boundary. Ordinary deferred resize uses the transition-only helper
    /// above so its already-claimed target-unbound packet can finish.
    fn fence_native_surface_target(&mut self) {
        let already_fenced = self.window.native_surface_target_fenced;
        let had_packet_work = self.window.native_visual_requests.has_work();
        self.clear_native_visual_request_wake();
        if already_fenced {
            if had_packet_work {
                let _ = self.invalidate_native_visual_requests();
            }
            return;
        }
        self.fence_native_surface_target_for_transition();
        let _ = self.invalidate_native_visual_requests();
    }

    fn fence_native_surface_target_for_transition(&mut self) {
        if self.window.native_surface_target_fenced {
            return;
        }
        self.window.native_surface_target_fenced = true;
        self.frame.clear_native_paint_segment_artifacts();
        self.frame.invalidate_native_scene_context();
        self.frame.mark_scene_texture_dirty();
        self.frame.mark_composited_base_dirty();
        self.window.target_generation.invalidate_unknown();
    }

    fn handle_other_surface_acquire_failure_for_packet(
        &mut self,
        size: PhysicalSize<u32>,
        requested_packet: bool,
    ) {
        self.window
            .surface_recovery
            .observe_acquire_error(&NativeSurfaceAcquireFailure::Other);
        self.fence_native_surface_target();
        if matches!(
            surface_acquire_policy(NativeSurfaceAcquireFailure::Other, size),
            SurfaceAcquirePolicy::ConservativeFence
        ) && self
            .window
            .surface_recovery
            .record_other_retry_request(requested_packet && size.width > 0 && size.height > 0)
        {
            self.request_redraw_for_recovery();
        }
    }

    pub(super) fn prepare_successful_surface_acquisition(&mut self) {
        if !self.window.target_generation.is_known() && self.window.target_generation.advance() {
            self.window.native_surface_target_fenced = false;
        }
        self.window.surface_recovery.rearm_transient_retry();
    }

    pub(super) fn update_native_dpi_scale(&mut self, scale_factor: f64) {
        self.window.native_dpi_scale = DpiScale::new(scale_factor);
        let scale_changed = self.apply_active_dpi_scale_to_viewport();
        if let Some(window) = self.window.window.as_ref()
            && let Some(fingerprint) = current_monitor_fingerprint(window)
        {
            self.window.monitor_fingerprint = Some(fingerprint);
        }
        let environment = environment_for_native_state(
            self.window.dpi_scale,
            self.window.environment.color_scheme(),
            self.window.accessibility_display,
        );
        let environment_changed = self.update_window_environment(environment);
        if scale_changed || environment_changed {
            self.queue_window_environment_change_with_reason(
                crate::runtime::WindowEnvironmentChange::DisplayScaleOrMonitor,
                FrameWorkReason::NativeDpiScale,
            );
        }
    }

    pub(super) fn set_dpi_scale_override(&mut self, scale: DpiScale) {
        self.window.dpi_scale_override = Some(scale);
        let scale_changed = self.apply_active_dpi_scale_to_viewport();
        let environment = environment_for_native_state(
            self.window.dpi_scale,
            self.window.environment.color_scheme(),
            self.window.accessibility_display,
        );
        if self.update_window_environment(environment) || scale_changed {
            self.queue_window_environment_change_with_reason(
                crate::runtime::WindowEnvironmentChange::DisplayScaleOrMonitor,
                FrameWorkReason::NativeDpiScale,
            );
        }
    }

    pub(super) fn set_window_logical_size(&mut self, size: Vector2) {
        let width = size.x.max(1.0);
        let height = size.y.max(1.0);
        if let Some(window) = self.window.window.as_ref() {
            let physical_size = PhysicalSize::new(
                self.window.dpi_scale.logical_to_physical(width).ceil() as u32,
                self.window.dpi_scale.logical_to_physical(height).ceil() as u32,
            );
            if let Some(applied_size) = window.request_inner_size(physical_size) {
                self.defer_surface_resize_with_reason(applied_size, FrameWorkReason::CommandResize);
            }
        }
    }

    fn apply_active_dpi_scale_to_viewport(&mut self) -> bool {
        let next = self.active_dpi_scale();
        if next == self.window.dpi_scale {
            return false;
        }
        self.window.dpi_scale = next;
        if self.window.native_resources.is_some()
            && self.window.target_generation.is_known()
            && !self.window.native_surface_target_fenced
        {
            self.window.target_generation.advance();
        }
        self.window.surface_recovery.rearm_transient_retry();
        self.frame.clear_native_paint_segment_artifacts();
        if let Some(window) = self.window.window.as_ref() {
            self.core
                .set_viewport(logical_viewport_for_size(window.inner_size(), next));
        }
        true
    }

    fn active_dpi_scale(&self) -> DpiScale {
        self.window
            .dpi_scale_override
            .unwrap_or(self.window.native_dpi_scale)
    }

    pub(super) fn acquire_present_surface_texture(
        &mut self,
        adapter: &GenericNativeAdapterOwner,
    ) -> Result<wgpu::SurfaceTexture, NativeSurfaceAcquireError> {
        if self.window.native_resources.is_none() {
            return Err(NativeSurfaceAcquireError::MissingResources);
        }
        let registration = adapter.capture_device_loss_registration();
        if let Some(registration) = registration.as_ref() {
            registration.begin_surface_acquire();
        }
        let texture = {
            let Some(resources) = self.window.native_resources.as_mut() else {
                return Err(NativeSurfaceAcquireError::MissingResources);
            };
            resources.render_surface.surface.get_current_texture()
        };
        let uncaptured_error =
            registration.and_then(|registration| registration.finish_surface_acquire());
        match texture {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => Ok(texture),
            texture => Err(NativeSurfaceAcquireError::Surface(
                map_current_surface_texture_error(&texture, uncaptured_error)
                    .expect("non-success current surface texture must map to an acquisition error"),
            )),
        }
    }

    pub(super) fn handle_present_surface_acquire_error(
        &mut self,
        event_loop: &ActiveEventLoop,
        adapter: &GenericNativeAdapterOwner,
        requested_packet: bool,
        error: NativeSurfaceAcquireError,
    ) -> NativeVisualRequestDisposition {
        let NativeSurfaceAcquireError::Surface(error) = error else {
            let _ = self.admit_native_resources(adapter);
            return NativeVisualRequestDisposition::DropPacket;
        };
        match error {
            error @ (NativeSurfaceAcquireFailure::Lost | NativeSurfaceAcquireFailure::Outdated) => {
                self.mark_cpu_frame_observation_recovery();
                self.window.surface_recovery.observe_acquire_error(&error);
                let Some(size) = self
                    .window
                    .window
                    .as_ref()
                    .map(|window| window.inner_size())
                else {
                    return NativeVisualRequestDisposition::DropPacket;
                };
                match surface_acquire_policy(error, size) {
                    SurfaceAcquirePolicy::ReconfigureAndRetry
                        if self.resize_surface_now_for_recovery(size, adapter) =>
                    {
                        self.window.surface_recovery.record_completed_reconfigure();
                        self.window.surface_recovery.record_retry_request();
                        if requested_packet {
                            self.request_redraw_for_recovery();
                        }
                    }
                    SurfaceAcquirePolicy::Defer => {
                        self.window.surface_recovery.record_zero_size_deferral();
                    }
                    _ => {}
                }
                NativeVisualRequestDisposition::DropPacket
            }
            error @ NativeSurfaceAcquireFailure::Occluded => {
                self.window.surface_occluded = true;
                self.window.surface_recovery.observe_acquire_error(&error);
                NativeVisualRequestDisposition::RetainUntilUnoccluded
            }
            error @ NativeSurfaceAcquireFailure::Timeout => {
                self.mark_cpu_frame_observation_recovery();
                self.window.surface_recovery.observe_acquire_error(&error);
                let size = self
                    .window
                    .window
                    .as_ref()
                    .map_or(PhysicalSize::new(0, 0), |window| window.inner_size());
                if matches!(
                    surface_acquire_policy(error, size),
                    SurfaceAcquirePolicy::Timeout
                ) && self
                    .window
                    .surface_recovery
                    .record_timeout_retry_request(size.width > 0 && size.height > 0)
                {
                    return NativeVisualRequestDisposition::RetrySamePacket;
                }
                NativeVisualRequestDisposition::DropPacket
            }
            NativeSurfaceAcquireFailure::OutOfMemory => {
                self.mark_cpu_frame_observation_recovery();
                error!("radiant generic native vello: out of memory acquiring surface");
                self.admit_native_shutdown(
                    event_loop,
                    Some(NativeGenericRunError::SurfaceAcquireOutOfMemory),
                );
                NativeVisualRequestDisposition::DropPacket
            }
            NativeSurfaceAcquireFailure::Other => {
                self.mark_cpu_frame_observation_recovery();
                let size = self
                    .window
                    .window
                    .as_ref()
                    .map_or(PhysicalSize::new(0, 0), |window| window.inner_size());
                self.handle_other_surface_acquire_failure_for_packet(size, requested_packet);
                warn!(
                    "radiant generic native vello: conservatively fenced surface after other acquire error"
                );
                NativeVisualRequestDisposition::DropPacket
            }
        }
    }

    #[cfg(test)]
    pub(super) fn handle_other_surface_acquire_failure(&mut self, size: PhysicalSize<u32>) {
        self.handle_other_surface_acquire_failure_for_packet(size, false);
    }

    pub(super) fn fence_native_presentation(&mut self) {
        self.timing.pending_surface_resize = None;
        self.timing.pending_surface_resize_reason = None;
        self.timing.pending_viewport_resize = None;
        self.timing.pending_viewport_resize_reason = None;
        let _ = self.invalidate_native_visual_requests();
        self.fence_native_surface_target_for_transition();
        self.apply_native_window_visibility(false);
    }

    /// Admit the active window bundle against the owner's exact current
    /// generation. A mismatch is fenced once and moved out of the active
    /// path; no native work or redraw retry is requested.
    pub(super) fn admit_native_resources(&mut self, adapter: &GenericNativeAdapterOwner) -> bool {
        if !self.is_running() {
            return false;
        }
        let Some(generation) = self
            .window
            .native_resources
            .as_ref()
            .map(|resources| resources.generation)
        else {
            self.fence_native_surface_target();
            return false;
        };
        if adapter.admit_generation(generation) {
            return true;
        }
        if let Some(resources) = self.window.native_resources.as_mut() {
            resources
                .gpu_resources
                .gpu_surface_renderer
                .discard_presentation_staging_belt();
        }
        let _ = self.window.isolate_native_resources();
        self.fence_native_surface_target();
        false
    }
}

fn native_initialization_error(
    stage: NativeInitializationStage,
    error: impl std::fmt::Display,
) -> NativeGenericRunError {
    NativeGenericRunError::NativeInitialization {
        stage,
        message: error.to_string(),
    }
}

#[cfg(test)]
impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn defer_surface_resize(&mut self, size: PhysicalSize<u32>) {
        self.defer_surface_resize_with_reason(size, FrameWorkReason::NativeResize);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NativeGenericRunError, NativeInitializationStage, NativeRenderDeviceErrorKind,
        map_current_surface_texture_error, native_initialization_error,
    };

    #[test]
    fn native_initialization_error_maps_each_production_stage_with_owned_message() {
        let stages = [
            NativeInitializationStage::WindowCreation,
            NativeInitializationStage::WgpuSurfaceCreation,
            NativeInitializationStage::DeviceAcquisition,
            NativeInitializationStage::RenderSurfaceCreation,
            NativeInitializationStage::RendererCreation,
        ];

        for stage in stages {
            let detail = String::from("backend detail");
            let error = native_initialization_error(stage, detail.as_str());
            drop(detail);

            assert_eq!(
                error,
                NativeGenericRunError::NativeInitialization {
                    stage,
                    message: String::from("backend detail"),
                }
            );
        }
    }

    #[test]
    fn current_surface_texture_mapping_keeps_occlusion_and_correlates_only_oom_validation() {
        assert_eq!(
            map_current_surface_texture_error(&vello::wgpu::CurrentSurfaceTexture::Occluded, None),
            Some(super::super::runner_state::NativeSurfaceAcquireFailure::Occluded)
        );
        assert_eq!(
            map_current_surface_texture_error(&vello::wgpu::CurrentSurfaceTexture::Timeout, None),
            Some(super::super::runner_state::NativeSurfaceAcquireFailure::Timeout)
        );
        assert_eq!(
            map_current_surface_texture_error(
                &vello::wgpu::CurrentSurfaceTexture::Validation,
                Some(NativeRenderDeviceErrorKind::OutOfMemory),
            ),
            Some(super::super::runner_state::NativeSurfaceAcquireFailure::OutOfMemory)
        );
        for kind in [None, Some(NativeRenderDeviceErrorKind::Validation)] {
            assert_eq!(
                map_current_surface_texture_error(
                    &vello::wgpu::CurrentSurfaceTexture::Validation,
                    kind,
                ),
                Some(super::super::runner_state::NativeSurfaceAcquireFailure::Other)
            );
        }
    }
}
