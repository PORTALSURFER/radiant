//! Window, surface, and renderer setup for the generic native Vello runner.

use super::{
    GenericNativeVelloRunner, generic_window_attributes, reveal_window_after_surface_setup,
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
    pub(super) fn initialize_runtime(&mut self, event_loop: &ActiveEventLoop) {
        info!("radiant generic native vello: initializing runtime window and surface");
        self.timing.startup_timing.mark_init_started();
        let window = match event_loop.create_window(generic_window_attributes(&self.options)) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                error!(
                    "radiant generic native vello: failed to create window: {:?}",
                    err
                );
                event_loop.exit();
                return;
            }
        };
        self.timing.startup_timing.mark_window_created();
        self.window.id = Some(window.id());
        self.window.native_dpi_scale = DpiScale::new(window.scale_factor());
        self.window.dpi_scale = self.active_dpi_scale();
        self.window.window = Some(Arc::clone(&window));

        let mut render_ctx = render_context_for_options(&self.options);
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        self.core
            .set_viewport(logical_viewport_for_size(size, self.window.dpi_scale));
        let surface = match render_ctx.instance.create_surface(window.clone()) {
            Ok(surface) => surface,
            Err(err) => {
                error!(
                    "radiant generic native vello: failed to create wgpu surface: {:?}",
                    err
                );
                event_loop.exit();
                return;
            }
        };
        self.timing.startup_timing.mark_wgpu_surface_created();
        let Some(dev_id) = pollster::block_on(render_ctx.device(Some(&surface))) else {
            error!("radiant generic native vello: no compatible render device found");
            event_loop.exit();
            return;
        };
        self.timing.startup_timing.mark_wgpu_device_ready();
        let supported_present_modes = surface
            .get_capabilities(render_ctx.devices[dev_id].adapter())
            .present_modes;
        let present_mode = select_present_mode(
            self.options.normalized_target_fps(),
            &supported_present_modes,
        );
        let render_surface = match pollster::block_on(render_ctx.create_render_surface(
            surface,
            width,
            height,
            present_mode,
        )) {
            Ok(render_surface) => render_surface,
            Err(err) => {
                error!(
                    "radiant generic native vello: failed to create render surface: {:?}",
                    err
                );
                event_loop.exit();
                return;
            }
        };
        self.timing.startup_timing.mark_surface_ready();
        let dev_handle = &render_ctx.devices[render_surface.dev_id];
        self.timing.startup_timing.mark_renderer_started();
        let renderer = match Renderer::new(&dev_handle.device, startup_renderer_options()) {
            Ok(renderer) => renderer,
            Err(err) => {
                error!(
                    "radiant generic native vello: failed to create renderer: {:?}",
                    err
                );
                event_loop.exit();
                return;
            }
        };
        self.timing.startup_timing.mark_renderer_ready();
        self.window.render_ctx = Some(render_ctx);
        self.window.render_surface = Some(render_surface);
        self.window.renderer = Some(renderer);
        self.rebuild_scene();
        self.timing.startup_timing.mark_first_scene_ready();
        if reveal_window_after_surface_setup(&self.options) {
            window.set_visible(true);
            self.timing.startup_timing.mark_window_revealed();
        }
        self.timing.last_redraw = Instant::now();
        self.request_redraw_if_needed();
        self.sync_auxiliary_windows(event_loop);
    }

    pub(super) fn resize_surface(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.defer_surface_resize(size);
        self.request_redraw_if_needed();
    }

    pub(super) fn defer_surface_resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.timing.pending_surface_resize = Some(size);
    }

    pub(super) fn apply_pending_surface_resize_if_needed(&mut self) {
        let Some(size) = self.timing.pending_surface_resize.take() else {
            return;
        };
        self.timing.surface_resize_applied_this_frame = self.resize_surface_now(size, false);
    }

    pub(super) fn resize_surface_now(
        &mut self,
        size: PhysicalSize<u32>,
        request_redraw: bool,
    ) -> bool {
        if size.width == 0 || size.height == 0 {
            return false;
        }
        self.timing.pending_surface_resize = None;
        if let (Some(render_ctx), Some(surface)) = (
            self.window.render_ctx.as_ref(),
            self.window.render_surface.as_mut(),
        ) {
            if !surface_size_changed(surface.config.width, surface.config.height, size) {
                return false;
            }
            render_ctx.resize_surface(surface, size.width, size.height);
            self.defer_viewport_resize(logical_viewport_for_size(size, self.window.dpi_scale));
            if request_redraw {
                self.request_redraw_if_needed();
            }
            return true;
        }
        false
    }

    pub(super) fn update_native_dpi_scale(&mut self, scale_factor: f64) {
        self.window.native_dpi_scale = DpiScale::new(scale_factor);
        if self.apply_active_dpi_scale_to_viewport() {
            self.rebuild_scene();
        }
        self.request_redraw_if_needed();
    }

    pub(super) fn set_dpi_scale_override(&mut self, scale: DpiScale) {
        self.window.dpi_scale_override = Some(scale);
        let _ = self.apply_active_dpi_scale_to_viewport();
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
                self.resize_surface(applied_size);
            }
        }
    }

    fn apply_active_dpi_scale_to_viewport(&mut self) -> bool {
        let next = self.active_dpi_scale();
        if next == self.window.dpi_scale {
            return false;
        }
        self.window.dpi_scale = next;
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
            Ok(frame) => Some(frame),
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                let window = self.window.window.as_ref()?;
                let size = window.inner_size();
                let _ = self.resize_surface_now(size, true);
                None
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                error!("radiant generic native vello: out of memory acquiring surface");
                event_loop.exit();
                None
            }
            Err(err) => {
                warn!(
                    "radiant generic native vello: non-fatal surface acquire error: {:?}",
                    err
                );
                None
            }
        }
    }
}
