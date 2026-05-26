use super::{
    GenericNativeVelloRunner, RenderFrameProfile, RenderSurfacePixelSize,
    hide_window_after_first_present, maybe_log_render_profile, post_gpu_overlay,
    reveal_window_after_first_present,
};
use crate::runtime::RuntimeBridge;
use std::time::Instant;
use vello::wgpu;
use winit::event_loop::ActiveEventLoop;

mod diagnostics;

use super::composited_base::{BaseFramePresentState, BaseFramePresentTarget, present_base_frame};
use super::scene_texture::render_scene_texture_if_needed;
use diagnostics::{NativeFrameDiagnosticsParts, native_frame_diagnostics};

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        self.timing.redraw_requested = false;
        if !self.timing.first_frame_presented {
            self.timing.startup_timing.mark_first_redraw_started();
        }
        self.apply_pending_surface_resize_if_needed();
        let Some(window) = self.window.window.clone() else {
            return;
        };
        let Some(dev_id) = self
            .window
            .render_surface
            .as_ref()
            .map(|surface| surface.dev_id)
        else {
            return;
        };
        let Some(surface_texture) = self.acquire_present_surface_texture(event_loop, &window)
        else {
            return;
        };
        let mut profile = RenderFrameProfile::default();
        self.flush_pending_gpu_surface_wheel(&mut profile);
        self.refresh_deferred_surface_if_needed(&mut profile);
        self.rebuild_deferred_scene_if_needed(&mut profile);
        self.paint_transient_overlays(&mut profile);
        let Some(surface) = self.window.render_surface.as_mut() else {
            return;
        };
        let Some(render_ctx) = self.window.render_ctx.as_ref() else {
            return;
        };
        let Some(renderer) = self.window.renderer.as_mut() else {
            return;
        };
        let dev_handle = &render_ctx.devices[dev_id];
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let Some(render_to_texture_elapsed) = render_scene_texture_if_needed(
            &mut self.frame,
            renderer,
            &dev_handle.device,
            &dev_handle.queue,
            surface,
            self.window.dpi_scale,
            event_loop,
        ) else {
            return;
        };
        let mut encoder =
            dev_handle
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("generic_native_vello_present_blit"),
                });
        let started = Instant::now();
        let gpu_surface_stats = present_base_frame(
            &mut BaseFramePresentState {
                base_frame: &mut self.frame.composited_base_frame,
                base_dirty: &mut self.frame.composited_base_dirty,
                gpu_surface_renderer: &mut self.frame.gpu_surface_renderer,
                profile: &mut profile,
            },
            surface,
            &mut BaseFramePresentTarget {
                device: &dev_handle.device,
                queue: &dev_handle.queue,
                encoder: &mut encoder,
                surface_view: &surface_view,
                dpi_scale: self.window.dpi_scale,
            },
            &self.frame.last_paint_plan,
            &self.frame.transient_overlay_primitives,
        );
        profile.full_screen_blit = started.elapsed();
        let surface_size = RenderSurfacePixelSize::from_surface(surface);
        self.frame.post_gpu_overlay_renderer.render_layers(
            &mut post_gpu_overlay::PostGpuOverlayRenderTarget {
                device: &dev_handle.device,
                queue: &dev_handle.queue,
                encoder: &mut encoder,
                target_view: &surface_view,
                format: surface.config.format,
                size: surface_size.logical_size(self.window.dpi_scale),
            },
            &self.frame.last_paint_plan.primitives,
            &self.frame.transient_overlay_primitives,
        );
        let started = Instant::now();
        dev_handle.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
        profile.submit_present = started.elapsed();
        let text_stats = self.frame.text_renderer.take_layout_profile_counters();
        maybe_log_render_profile(
            "present",
            self.frame.last_scene_stats,
            text_stats,
            render_to_texture_elapsed,
            profile,
            gpu_surface_stats,
            self.timing.last_redraw.elapsed(),
        );
        let diagnostics = native_frame_diagnostics(NativeFrameDiagnosticsParts {
            stats: self.frame.last_scene_stats,
            text_stats,
            retained_policy: self.frame.retained_surface_cache.policy(),
            retained_entries: self.frame.retained_surface_cache.entry_count(),
            gpu_surface_stats,
            profile,
            render_to_texture_elapsed,
            since_last_present: self.timing.last_redraw.elapsed(),
        });
        self.core
            .runtime
            .bridge_mut()
            .observe_frame_diagnostics(diagnostics);
        self.timing.last_redraw = Instant::now();
        self.mark_first_presented();
    }

    fn mark_first_presented(&mut self) {
        if !self.timing.first_frame_presented {
            self.timing.first_frame_presented = true;
            if reveal_window_after_first_present(&self.options)
                && let Some(window) = self.window.window.as_ref()
            {
                window.set_visible(true);
                self.timing.startup_timing.mark_window_revealed();
            }
            if hide_window_after_first_present(&self.options)
                && let Some(window) = self.window.window.as_ref()
            {
                window.set_visible(false);
            }
            self.timing.startup_timing.mark_first_presented();
            self.timing.startup_timing.maybe_emit_summary();
        }
    }
}
