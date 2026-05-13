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
            self.composited_base_dirty = true;
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
            self.composited_base_dirty = true;
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
        self.transient_overlay_primitives.clear();
        let started = Instant::now();
        self.core.paint_transient_overlay(
            &self.last_paint_plan,
            &mut self.transient_overlay_primitives,
            self.animation_origin.elapsed(),
        );
        self.core
            .paint_runtime_overlay(&mut self.transient_overlay_primitives);
        profile.transient_overlay_paint = started.elapsed();
        profile.transient_overlay_primitives = self.transient_overlay_primitives.len();
        let started = Instant::now();
        let gpu_surface_stats = present_base_frame(
            &mut BaseFramePresentState {
                base_frame: &mut self.composited_base_frame,
                base_dirty: &mut self.composited_base_dirty,
                gpu_surface_renderer: &mut self.gpu_surface_renderer,
                profile: &mut profile,
            },
            surface,
            &mut BaseFramePresentTarget {
                device: &dev_handle.device,
                queue: &dev_handle.queue,
                encoder: &mut encoder,
                surface_view: &surface_view,
            },
            &self.last_paint_plan,
            &self.transient_overlay_primitives,
        );
        profile.full_screen_blit = started.elapsed();
        self.post_gpu_overlay_renderer.render_layers(
            &mut post_gpu_overlay::PostGpuOverlayRenderTarget {
                device: &dev_handle.device,
                queue: &dev_handle.queue,
                encoder: &mut encoder,
                target_view: &surface_view,
                format: surface.config.format,
                size: Vector2::new(surface.config.width as f32, surface.config.height as f32),
            },
            &self.last_paint_plan.primitives,
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
