use super::*;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        self.redraw_requested = false;
        if !self.first_frame_presented {
            self.startup_timing.mark_first_redraw_started();
        }
        let Some(window) = self.window.clone() else {
            return;
        };
        let Some(dev_id) = self.render_surface.as_ref().map(|surface| surface.dev_id) else {
            return;
        };
        let Some(surface_texture) = self.acquire_present_surface_texture(event_loop, &window)
        else {
            return;
        };
        let mut profile = RenderFrameProfile::default();
        self.flush_pending_gpu_surface_wheel(&mut profile);
        self.refresh_deferred_surface_if_needed(&mut profile);
        self.paint_transient_overlays(&mut profile);
        let Some(surface) = self.render_surface.as_mut() else {
            return;
        };
        let Some(render_ctx) = self.render_ctx.as_ref() else {
            return;
        };
        let Some(renderer) = self.renderer.as_mut() else {
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
                size: surface_size.logical_size(),
            },
            &self.frame.last_paint_plan.primitives,
            &self.frame.transient_overlay_primitives,
        );
        let started = Instant::now();
        dev_handle.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
        profile.submit_present = started.elapsed();
        maybe_log_render_profile(
            "present",
            self.frame.last_scene_stats,
            render_to_texture_elapsed,
            profile,
            gpu_surface_stats,
            self.last_redraw.elapsed(),
        );
        let diagnostics = native_frame_diagnostics(
            self.frame.last_scene_stats,
            self.frame.retained_surface_cache.policy(),
            self.frame.retained_surface_cache.entry_count(),
            gpu_surface_stats,
            profile,
            render_to_texture_elapsed,
            self.last_redraw.elapsed(),
        );
        self.core
            .runtime
            .bridge_mut()
            .observe_frame_diagnostics(diagnostics);
        self.last_redraw = Instant::now();
        self.mark_first_presented();
    }

    fn mark_first_presented(&mut self) {
        if !self.first_frame_presented {
            self.first_frame_presented = true;
            if reveal_window_after_first_present(&self.options)
                && let Some(window) = self.window.as_ref()
            {
                window.set_visible(true);
                self.startup_timing.mark_window_revealed();
            }
            if hide_window_after_first_present(&self.options)
                && let Some(window) = self.window.as_ref()
            {
                window.set_visible(false);
            }
            self.startup_timing.mark_first_presented();
            self.startup_timing.maybe_emit_summary();
        }
    }
}

fn native_frame_diagnostics(
    stats: RetainedSurfaceEncodeStats,
    retained_policy: crate::runtime::RetainedSurfaceCachePolicy,
    retained_entries: usize,
    gpu_surface_stats: gpu_surface::GpuSurfaceRenderStats,
    profile: RenderFrameProfile,
    render_to_texture_elapsed: Duration,
    since_last_present: Duration,
) -> crate::runtime::NativeFrameDiagnostics {
    crate::runtime::NativeFrameDiagnostics {
        scene: crate::runtime::NativeSceneDiagnostics {
            paint_plan_primitives: stats.paint_plan_primitives,
            clip_layer_count: stats.clip_layer_count,
            text_primitive_count: stats.text_primitive_count,
            text_input_count: stats.text_input_count,
            image_count: stats.image_count,
            svg_document_count: stats.svg_document_count,
            gpu_surface_count: stats.gpu_surface_count,
            custom_surface_count: stats.custom_surface_count,
            custom_surface_fallback_count: stats.custom_surface_fallback_count,
            text_run_count: stats.text_run_count,
        },
        retained_surfaces: crate::runtime::NativeRetainedSurfaceDiagnostics {
            cache_capacity: retained_policy.max_frames,
            cache_entries: retained_entries,
            bridge_calls: stats.bridge_calls,
            cache_hits: stats.cache_hits,
            miss_count: stats.retained_surface_miss_count,
            retained_frame_primitive_count: stats.retained_frame_primitive_count,
            retained_frame_text_run_count: stats.retained_frame_text_run_count,
        },
        gpu_surfaces: crate::runtime::NativeGpuSurfaceDiagnostics {
            atlas_texture_uploads: gpu_surface_stats.atlas_texture_uploads,
            atlas_texture_cache_hits: gpu_surface_stats.atlas_texture_cache_hits,
            signal_summary_builds: gpu_surface_stats.signal_summary_builds,
            signal_summary_cache_hits: gpu_surface_stats.signal_summary_cache_hits,
            signal_body_renders: gpu_surface_stats.signal_body_renders,
            signal_body_cache_hits: gpu_surface_stats.signal_body_cache_hits,
            composite_binding_rebuilds: gpu_surface_stats.composite_binding_rebuilds,
            composite_binding_cache_hits: gpu_surface_stats.composite_binding_cache_hits,
        },
        timings: crate::runtime::NativeFrameTimingDiagnostics {
            coalesced_wheel_route: profile.coalesced_wheel_route,
            refresh_surface: profile.refresh_surface,
            paint_plan: profile.paint_plan,
            render_to_texture: render_to_texture_elapsed,
            full_screen_blit: profile.full_screen_blit,
            composited_base_refresh: profile.composited_base_refresh,
            composited_base_cache_hit: profile.composited_base_cache_hit,
            transient_overlay_paint: profile.transient_overlay_paint,
            transient_overlay_primitives: profile.transient_overlay_primitives,
            submit_present: profile.submit_present,
            since_last_present,
        },
    }
}
