//! Window, surface, and renderer setup for the generic native Vello runner.

use super::runner_state::{SurfaceAcquirePolicy, surface_acquire_policy};
use super::{
    FrameWork, FrameWorkReason, GenericNativeVelloRunner, NativeGenericRunError,
    NativeInitializationStage, SceneRebuildMode, configure_created_top_level_window,
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
    gui_runtime::native_vello::{select_present_mode, startup_renderer_options},
    runtime::RuntimeBridge,
    theme::DpiScale,
};
use std::{sync::Arc, time::Instant};
use tracing::{error, info, warn};
use vello::{Renderer, wgpu};
use winit::{dpi::PhysicalSize, event_loop::ActiveEventLoop};

mod backend;
mod viewport;

use backend::render_context_for_options;
use viewport::{logical_viewport_for_size, surface_size_changed};

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn initialize_runtime(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), NativeGenericRunError> {
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
        self.window.native_dpi_scale = DpiScale::new(window.scale_factor());
        self.window.dpi_scale = self.active_dpi_scale();
        self.window.monitor_fingerprint = current_monitor_fingerprint(&window);
        self.window.accessibility_display = accessibility::current_snapshot();
        self.window.environment = environment_for_native_state(
            self.window.dpi_scale,
            window_color_scheme(window.theme()),
            self.window.accessibility_display,
        );
        if self
            .core
            .runtime
            .set_window_environment(self.window.environment)
        {
            // Startup is the one non-deferred environment transition: the
            // first scene must be projected with the native values already
            // known from the created window.
            self.core.refresh_surface();
        }

        let mut render_ctx = render_context_for_options(&self.options);
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        self.core
            .set_viewport(logical_viewport_for_size(size, self.window.dpi_scale));
        let surface = render_ctx
            .instance
            .create_surface(window.clone())
            .map_err(|err| {
                native_initialization_error(NativeInitializationStage::WgpuSurfaceCreation, err)
            })?;
        self.timing.startup_timing.mark_wgpu_surface_created();
        let Some(dev_id) = pollster::block_on(render_ctx.device(Some(&surface))) else {
            return Err(native_initialization_error(
                NativeInitializationStage::DeviceAcquisition,
                "no compatible render device found",
            ));
        };
        self.timing.startup_timing.mark_wgpu_device_ready();
        let supported_present_modes = surface
            .get_capabilities(render_ctx.devices[dev_id].adapter())
            .present_modes;
        let present_mode = select_present_mode(
            self.options.normalized_target_fps(),
            &supported_present_modes,
        );
        let render_surface = pollster::block_on(render_ctx.create_render_surface(
            surface,
            width,
            height,
            present_mode,
        ))
        .map_err(|err| {
            native_initialization_error(NativeInitializationStage::RenderSurfaceCreation, err)
        })?;
        self.timing.startup_timing.mark_surface_ready();
        let dev_handle = &render_ctx.devices[render_surface.dev_id];
        self.timing.startup_timing.mark_renderer_started();
        let renderer =
            Renderer::new(&dev_handle.device, startup_renderer_options()).map_err(|err| {
                native_initialization_error(NativeInitializationStage::RendererCreation, err)
            })?;
        self.timing.startup_timing.mark_renderer_ready();
        self.window.id = Some(window.id());
        self.window.window = Some(Arc::clone(&window));
        self.window.render_ctx = Some(render_ctx);
        self.window.render_surface = Some(render_surface);
        self.window.renderer = Some(renderer);
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
        self.sync_auxiliary_windows(event_loop)?;
        Ok(())
    }

    pub(super) fn resize_surface(&mut self, size: PhysicalSize<u32>) {
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
        self.timing.pending_surface_resize = Some(size);
        self.timing.pending_surface_resize_reason = Some(reason);
    }

    pub(super) fn apply_pending_surface_resize_if_needed(&mut self) {
        let Some(size) = self.timing.pending_surface_resize.take() else {
            return;
        };
        let reason = self
            .timing
            .pending_surface_resize_reason
            .take()
            .unwrap_or(FrameWorkReason::NativeResize);
        let applied = self.resize_surface_now(size, false, reason);
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
    ) -> bool {
        if size.width == 0 || size.height == 0 {
            return false;
        }
        self.timing.pending_surface_resize = None;
        self.timing.pending_surface_resize_reason = None;
        if let (Some(render_ctx), Some(surface)) = (
            self.window.render_ctx.as_ref(),
            self.window.render_surface.as_mut(),
        ) {
            if !surface_size_changed(surface.config.width, surface.config.height, size) {
                return false;
            }
            render_ctx.resize_surface(surface, size.width, size.height);
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

    fn resize_surface_now_for_recovery(&mut self, size: PhysicalSize<u32>) -> bool {
        if size.width == 0 || size.height == 0 {
            return false;
        }
        self.timing.pending_surface_resize = None;
        self.timing.pending_surface_resize_reason = None;
        if let (Some(render_ctx), Some(surface)) = (
            self.window.render_ctx.as_ref(),
            self.window.render_surface.as_mut(),
        ) {
            render_ctx.resize_surface(surface, size.width, size.height);
            self.complete_target_transition();
            return true;
        }
        false
    }

    fn complete_target_transition(&mut self) {
        self.fence_native_surface_target();
        self.window.target_generation.advance();
        self.window.surface_recovery.rearm_transient_retry();
    }

    fn fence_native_surface_target(&mut self) {
        self.frame.clear_native_paint_segment_artifacts();
        self.frame.invalidate_native_scene_context();
        self.frame.mark_scene_texture_dirty();
        self.frame.mark_composited_base_dirty();
        self.window.target_generation.invalidate_unknown();
    }

    pub(super) fn handle_other_surface_acquire_failure(&mut self, size: PhysicalSize<u32>) {
        self.window
            .surface_recovery
            .observe_acquire_error(&wgpu::SurfaceError::Other);
        self.fence_native_surface_target();
        if matches!(
            surface_acquire_policy(wgpu::SurfaceError::Other, size),
            SurfaceAcquirePolicy::ConservativeFence
        ) && self
            .window
            .surface_recovery
            .record_other_retry_request(size.width > 0 && size.height > 0)
        {
            self.request_redraw_for_frame_work(FrameWork::None);
        }
    }

    pub(super) fn prepare_successful_surface_acquisition(&mut self) {
        if !self.window.target_generation.is_known() {
            self.window.target_generation.advance();
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
        self.window.target_generation.advance();
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
        event_loop: &ActiveEventLoop,
    ) -> Option<wgpu::SurfaceTexture> {
        let texture = {
            let surface = self.window.render_surface.as_mut()?;
            surface.surface.get_current_texture()
        };
        match texture {
            Ok(frame) => {
                self.prepare_successful_surface_acquisition();
                Some(frame)
            }
            Err(error @ (wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated)) => {
                self.window.surface_recovery.observe_acquire_error(&error);
                let size = self.window.window.as_ref()?.inner_size();
                match surface_acquire_policy(error, size) {
                    SurfaceAcquirePolicy::ReconfigureAndRetry
                        if self.resize_surface_now_for_recovery(size) =>
                    {
                        self.window.surface_recovery.record_completed_reconfigure();
                        self.window.surface_recovery.record_retry_request();
                        self.request_redraw_for_frame_work(FrameWork::None);
                    }
                    SurfaceAcquirePolicy::Defer => {
                        self.window.surface_recovery.record_zero_size_deferral();
                    }
                    _ => {}
                }
                None
            }
            Err(error @ wgpu::SurfaceError::Timeout) => {
                self.window.surface_recovery.observe_acquire_error(&error);
                let size = self.window.window.as_ref()?.inner_size();
                if matches!(
                    surface_acquire_policy(error, size),
                    SurfaceAcquirePolicy::Timeout
                ) && self
                    .window
                    .surface_recovery
                    .record_timeout_retry_request(size.width > 0 && size.height > 0)
                {
                    self.request_redraw_for_frame_work(FrameWork::None);
                }
                None
            }
            Err(wgpu::SurfaceError::Other) => {
                let size = self
                    .window
                    .window
                    .as_ref()
                    .map_or(PhysicalSize::new(0, 0), |window| window.inner_size());
                self.handle_other_surface_acquire_failure(size);
                warn!(
                    "radiant generic native vello: conservatively fenced surface after other acquire error"
                );
                None
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                error!("radiant generic native vello: out of memory acquiring surface");
                self.record_terminal_cause(NativeGenericRunError::SurfaceAcquireOutOfMemory);
                event_loop.exit();
                None
            }
        }
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
