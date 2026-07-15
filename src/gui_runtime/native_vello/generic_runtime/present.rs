use super::{
    GenericNativeVelloRunner, RenderFrameProfile, RenderSurfacePixelSize,
    hide_window_after_first_present, maybe_log_render_profile, maybe_log_slow_render_profile,
    post_gpu_overlay, render_profile_enabled, reveal_window_after_first_present,
    slow_render_profile_enabled,
};
use crate::runtime::RuntimeBridge;
use std::time::Instant;
use vello::wgpu;
use winit::event_loop::ActiveEventLoop;

mod diagnostics;

use super::composited_base::{BaseFramePresentState, BaseFramePresentTarget, present_base_frame};
use super::scene_texture::{
    SceneTextureContext, render_scene_texture_if_needed, render_scene_to_surface_view,
};
use diagnostics::{NativeFrameDiagnosticsParts, native_frame_diagnostics};

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        self.timing.redraw_requested = false;
        self.timing.redraw_requested_at = None;
        self.timing.surface_resize_applied_this_frame = false;
        if !self.timing.first_frame_presented {
            self.timing.startup_timing.mark_first_redraw_started();
        }
        self.apply_pending_surface_resize_if_needed();
        if self.window.window.is_none() {
            return;
        }
        let Some(surface_texture) = self.acquire_present_surface_texture(event_loop) else {
            return;
        };
        let profile_enabled = render_profile_enabled();
        let diagnostics_requested = self.frame_diagnostics_enabled;
        let slow_profile_enabled = slow_render_profile_enabled();
        let mut profile = RenderFrameProfile::recording(
            profile_enabled || diagnostics_requested || slow_profile_enabled,
        );
        self.flush_pending_scrollbar_drag_now();
        self.flush_pending_gpu_surface_wheel(&mut profile);
        self.flush_pending_scroll_container_wheel(&mut profile);
        self.refresh_deferred_surface_if_needed(&mut profile);
        self.rebuild_deferred_scene_if_needed(&mut profile);
        self.sync_deferred_auxiliary_windows_if_needed(event_loop);
        self.paint_transient_overlays(&mut profile);
        let frame_work = self.take_pending_frame_work();
        let render_resize_frame_directly = self.should_render_resize_frame_directly();
        let Some(surface) = self.window.render_surface.as_mut() else {
            return;
        };
        let dev_id = surface.dev_id;
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
        let mut scene_texture_context = SceneTextureContext {
            renderer,
            device: &dev_handle.device,
            queue: &dev_handle.queue,
            surface,
            dpi_scale: self.window.dpi_scale,
            record_timing: profile.record_timings,
            event_loop,
        };
        if render_resize_frame_directly {
            let Some(render_to_texture_elapsed) = render_scene_to_surface_view(
                &mut self.frame,
                &mut scene_texture_context,
                &surface_view,
            ) else {
                return;
            };
            let (_, elapsed) = profile.measure(|| surface_texture.present());
            profile.submit_present = elapsed;
            self.finish_direct_resize_present(
                render_to_texture_elapsed,
                profile,
                profile_enabled,
                diagnostics_requested,
                frame_work,
            );
            return;
        }
        let Some(render_to_texture_elapsed) =
            render_scene_texture_if_needed(&mut self.frame, &mut scene_texture_context)
        else {
            return;
        };
        let mut encoder =
            dev_handle
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("generic_native_vello_present_blit"),
                });
        let started = profile.record_timings.then(Instant::now);
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
            &self.frame.surface_occlusion_plan,
            &self.frame.transient_overlay_primitives,
            self.frame.last_scene_stats.gpu_surface_count > 0,
        );
        profile.full_screen_blit = started.map(|started| started.elapsed()).unwrap_or_default();
        if self.frame.has_post_gpu_overlay_work() {
            let surface_size = RenderSurfacePixelSize::from_surface(surface);
            self.frame
                .render_post_gpu_overlay(&mut post_gpu_overlay::PostGpuOverlayRenderTarget {
                    device: &dev_handle.device,
                    queue: &dev_handle.queue,
                    encoder: &mut encoder,
                    target_view: &surface_view,
                    format: surface.config.format,
                    size: surface_size.logical_size(self.window.dpi_scale),
                });
        }
        let (_, elapsed) = profile.measure(|| {
            dev_handle.queue.submit(std::iter::once(encoder.finish()));
            surface_texture.present();
        });
        profile.submit_present = elapsed;
        let text_stats = if profile_enabled || diagnostics_requested {
            self.frame.text_renderer.take_layout_profile_counters()
        } else {
            Default::default()
        };
        let now = Instant::now();
        let since_last_present = now.duration_since(self.timing.last_redraw);
        if profile_enabled {
            maybe_log_render_profile(
                "present",
                self.frame.last_scene_stats,
                text_stats,
                render_to_texture_elapsed,
                profile,
                gpu_surface_stats,
                since_last_present,
            );
        }
        maybe_log_slow_render_profile(
            "present",
            self.frame.last_scene_stats,
            render_to_texture_elapsed,
            profile,
            gpu_surface_stats,
            since_last_present,
        );
        let (surface_refresh, surface_refresh_total) =
            self.core.runtime.take_frame_refresh_diagnostics();
        if diagnostics_requested {
            let diagnostics = native_frame_diagnostics(NativeFrameDiagnosticsParts {
                stats: self.frame.last_scene_stats,
                text_stats,
                retained_policy: self.frame.retained_surface_cache.policy(),
                retained_entries: self.frame.retained_surface_cache.entry_count(),
                gpu_surface_stats,
                profile,
                render_to_texture_elapsed,
                since_last_present,
                frame_work,
                surface_refresh,
                surface_refresh_total,
            });
            self.core
                .runtime
                .host_observe_frame_diagnostics(diagnostics);
        }
        self.timing.last_redraw = now;
        self.mark_first_presented();
    }

    pub(super) fn should_render_resize_frame_directly(&self) -> bool {
        self.timing.surface_resize_applied_this_frame
            && self.frame.scene_texture_dirty
            && self.frame.transient_overlay_primitives.is_empty()
            && self.frame.last_scene_stats.gpu_surface_count == 0
            && !self.frame.has_post_gpu_overlay_work()
    }

    fn finish_direct_resize_present(
        &mut self,
        render_to_texture_elapsed: std::time::Duration,
        profile: RenderFrameProfile,
        profile_enabled: bool,
        diagnostics_requested: bool,
        frame_work: super::FrameWork,
    ) {
        let text_stats = if profile_enabled || diagnostics_requested {
            self.frame.text_renderer.take_layout_profile_counters()
        } else {
            Default::default()
        };
        let now = Instant::now();
        let since_last_present = now.duration_since(self.timing.last_redraw);
        let gpu_surface_stats = Default::default();
        if profile_enabled {
            maybe_log_render_profile(
                "present",
                self.frame.last_scene_stats,
                text_stats,
                render_to_texture_elapsed,
                profile,
                gpu_surface_stats,
                since_last_present,
            );
        }
        maybe_log_slow_render_profile(
            "present.resize_direct",
            self.frame.last_scene_stats,
            render_to_texture_elapsed,
            profile,
            gpu_surface_stats,
            since_last_present,
        );
        let (surface_refresh, surface_refresh_total) =
            self.core.runtime.take_frame_refresh_diagnostics();
        if diagnostics_requested {
            let diagnostics = native_frame_diagnostics(NativeFrameDiagnosticsParts {
                stats: self.frame.last_scene_stats,
                text_stats,
                retained_policy: self.frame.retained_surface_cache.policy(),
                retained_entries: self.frame.retained_surface_cache.entry_count(),
                gpu_surface_stats,
                profile,
                render_to_texture_elapsed,
                since_last_present,
                frame_work,
                surface_refresh,
                surface_refresh_total,
            });
            self.core
                .runtime
                .host_observe_frame_diagnostics(diagnostics);
        }
        self.timing.last_redraw = now;
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
