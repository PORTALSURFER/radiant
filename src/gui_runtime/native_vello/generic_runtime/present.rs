use super::*;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        self.redraw_requested = false;
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Some(dev_id) = self.render_surface.as_ref().map(|surface| surface.dev_id) else {
            return;
        };
        let surface_texture = {
            let Some(surface) = self.render_surface.as_mut() else {
                return;
            };
            match surface.surface.get_current_texture() {
                Ok(frame) => frame,
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    let size = window.inner_size();
                    self.resize_surface(size);
                    return;
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    error!("radiant generic native vello: out of memory acquiring surface");
                    event_loop.exit();
                    return;
                }
                Err(err) => {
                    warn!(
                        "radiant generic native vello: non-fatal surface acquire error: {:?}",
                        err
                    );
                    return;
                }
            }
        };
        let mut profile = RenderFrameProfile::default();
        self.flush_pending_gpu_surface_wheel(&mut profile);
        if self.deferred_surface_refresh {
            let started = Instant::now();
            self.core.refresh_surface();
            self.deferred_surface_refresh = false;
            profile.refresh_surface = started.elapsed();
            let started = Instant::now();
            self.core.paint_plan_into(&mut self.last_paint_plan);
            profile.paint_plan = started.elapsed();
            collect_gpu_surface_interaction_regions(
                &self.last_paint_plan.primitives,
                &mut self.gpu_surface_interaction_regions,
            );
        }
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
        let render_to_texture_elapsed = if self.scene_texture_dirty {
            let render_started = Instant::now();
            let render_result = renderer.render_to_texture(
                &dev_handle.device,
                &dev_handle.queue,
                &self.scene,
                &surface.target_view,
                &RenderParams {
                    base_color: color_from_rgba(self.last_paint_plan.clear_color),
                    width: surface.config.width,
                    height: surface.config.height,
                    antialiasing_method: AaConfig::Area,
                },
            );
            let elapsed = render_started.elapsed();
            if let Err(err) = render_result {
                error!(
                    "radiant generic native vello: render_to_texture failed: {:?}",
                    err
                );
                event_loop.exit();
                return;
            }
            self.scene_texture_dirty = false;
            elapsed
        } else {
            Duration::ZERO
        };
        let mut encoder =
            dev_handle
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("generic_native_vello_present_blit"),
                });
        let started = Instant::now();
        surface.blitter.copy(
            &dev_handle.device,
            &mut encoder,
            &surface.target_view,
            &surface_view,
        );
        profile.full_screen_blit = started.elapsed();
        let gpu_surface_stats = self.gpu_surface_renderer.render(
            &mut gpu_surface::GpuSurfaceRenderTarget {
                device: &dev_handle.device,
                queue: &dev_handle.queue,
                encoder: &mut encoder,
                target_view: &surface_view,
                format: surface.config.format,
                size: Vector2::new(surface.config.width as f32, surface.config.height as f32),
            },
            &self.last_paint_plan.primitives,
        );
        self.post_gpu_overlay_renderer.render(
            &mut post_gpu_overlay::PostGpuOverlayRenderTarget {
                device: &dev_handle.device,
                encoder: &mut encoder,
                target_view: &surface_view,
                format: surface.config.format,
                size: Vector2::new(surface.config.width as f32, surface.config.height as f32),
            },
            &self.last_paint_plan.primitives,
        );
        self.transient_overlay_primitives.clear();
        self.core.paint_transient_overlay(
            &self.last_paint_plan,
            &mut self.transient_overlay_primitives,
            self.animation_origin.elapsed(),
        );
        self.post_gpu_overlay_renderer.render_primitives(
            &mut post_gpu_overlay::PostGpuOverlayRenderTarget {
                device: &dev_handle.device,
                encoder: &mut encoder,
                target_view: &surface_view,
                format: surface.config.format,
                size: Vector2::new(surface.config.width as f32, surface.config.height as f32),
            },
            &self.transient_overlay_primitives,
        );
        let started = Instant::now();
        dev_handle.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
        profile.submit_present = started.elapsed();
        maybe_log_render_profile(
            "present",
            self.last_scene_stats,
            render_to_texture_elapsed,
            profile,
            gpu_surface_stats,
            self.last_redraw.elapsed(),
        );
        self.last_redraw = Instant::now();
        if !self.first_frame_presented {
            self.first_frame_presented = true;
            self.startup_timing.mark_first_presented();
            self.startup_timing.maybe_emit_summary();
        }
    }
}

fn maybe_log_render_profile(
    reason: &'static str,
    stats: RetainedSurfaceEncodeStats,
    render_to_texture_elapsed: Duration,
    frame: RenderFrameProfile,
    gpu_surface_stats: gpu_surface::GpuSurfaceRenderStats,
    since_last_present: Duration,
) {
    if !render_profile_enabled() {
        return;
    }
    info!(
        reason,
        paint_plan_primitives = stats.paint_plan_primitives,
        scene_clip_layers = stats.clip_layer_count,
        scene_text_primitives = stats.text_primitive_count,
        scene_text_inputs = stats.text_input_count,
        scene_text_runs = stats.text_run_count,
        scene_images = stats.image_count,
        scene_gpu_surfaces = stats.gpu_surface_count,
        scene_custom_surfaces = stats.custom_surface_count,
        retained_bridge_calls = stats.bridge_calls,
        retained_cache_hits = stats.cache_hits,
        retained_frame_primitives = stats.retained_frame_primitive_count,
        retained_frame_text_runs = stats.retained_frame_text_run_count,
        gpu_surface_atlas_texture_uploads = gpu_surface_stats.atlas_texture_uploads,
        gpu_signal_summary_builds = gpu_surface_stats.signal_summary_builds,
        gpu_signal_summary_cache_hits = gpu_surface_stats.signal_summary_cache_hits,
        refresh_surface_us = frame.refresh_surface.as_micros(),
        paint_plan_us = frame.paint_plan.as_micros(),
        render_to_texture_us = render_to_texture_elapsed.as_micros(),
        full_screen_blit_encode_us = frame.full_screen_blit.as_micros(),
        coalesced_wheel_route_us = frame.coalesced_wheel_route.as_micros(),
        gpu_signal_body_renders = gpu_surface_stats.signal_body_renders,
        gpu_signal_body_cache_hits = gpu_surface_stats.signal_body_cache_hits,
        gpu_signal_body_encode_us = gpu_surface_stats.signal_body_encode_elapsed.as_micros(),
        gpu_surface_composite_encode_us = gpu_surface_stats.composite_encode_elapsed.as_micros(),
        submit_present_us = frame.submit_present.as_micros(),
        since_last_present_us = since_last_present.as_micros(),
        "radiant native render profile"
    );
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct RenderFrameProfile {
    pub(super) coalesced_wheel_route: Duration,
    pub(super) refresh_surface: Duration,
    pub(super) paint_plan: Duration,
    pub(super) full_screen_blit: Duration,
    pub(super) submit_present: Duration,
}
