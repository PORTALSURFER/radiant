use super::gpu_surface::{
    GpuSurfaceRenderCanvasUploadPlan, GpuSurfaceRenderCanvasUploadPlanContext,
    GpuSurfaceRenderCanvasUploadTarget,
};
use super::gpu_timing::GpuTimingAdmission;
use super::native_encode_present::NativeEncodePresentPath;
use super::native_visual_packet::{NativeVisualRequestBegin, NativeVisualRequestDisposition};
use super::{
    CpuFrameStage, GenericNativeAdapterOwner, GenericNativeVelloRunner,
    NativeAdapterAtlasResidencyProfile, NativeAdapterCustomShaderResidencyProfile,
    NativeAdapterRenderCanvasUploadProfile, NativeAdapterSignalResidencyProfile,
    NativeRenderProfileGpuSurface, RenderFrameProfile, RenderSurfacePixelSize,
    hide_window_after_first_present, maybe_log_render_profile, maybe_log_slow_render_profile,
    post_gpu_overlay, render_profile_enabled, reveal_window_after_first_present,
    slow_render_profile_enabled,
};
use crate::runtime::RuntimeBridge;
use std::time::Instant;
use vello::wgpu;
use winit::event_loop::ActiveEventLoop;

mod diagnostics;

use super::composited_base::{
    BaseFramePresentRequest, BaseFramePresentState, BaseFramePresentTarget, present_base_frame,
};
use super::device::wgpu_device_id;
use super::scene_texture::{
    NativeFrameRenderFailure, SceneTextureContext, render_scene_texture_if_needed,
    render_scene_to_surface_view,
};
use diagnostics::{NativeFrameDiagnosticsParts, native_frame_diagnostics};

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn redraw(
        &mut self,
        event_loop: &ActiveEventLoop,
        adapter: &mut GenericNativeAdapterOwner,
        requested_packet: bool,
        packet_identity: super::native_visual_packet::NativeVisualRequestIdentity,
    ) -> Result<NativeVisualRequestDisposition, NativeFrameRenderFailure> {
        self.cpu_frame_observation_capture.reset();
        self.clear_native_visual_request_wake_timing();
        self.timing.surface_resize_applied_this_frame = false;
        if !self.resume_deferred_deadline_before_redraw(event_loop, adapter) {
            return Ok(NativeVisualRequestDisposition::DropPacket);
        }
        if !self.admit_native_resources(adapter) {
            return Ok(NativeVisualRequestDisposition::DropPacket);
        }
        if !self.timing.first_frame_presented {
            self.timing.startup_timing.mark_first_redraw_started();
        }
        self.apply_pending_surface_resize_if_needed(adapter);
        if self.window.window.is_none() {
            return Ok(NativeVisualRequestDisposition::DropPacket);
        }
        self.cpu_frame_observation_capture.mark_frame_path_started();
        let profile_enabled = render_profile_enabled();
        let diagnostics_requested = self.frame_observation_enabled;
        let slow_profile_enabled = slow_render_profile_enabled();
        let mut profile = RenderFrameProfile::recording(
            profile_enabled || diagnostics_requested || slow_profile_enabled,
        );
        profile.window_identity = self.timing.native_window_diagnostic_identity;
        let had_coalesced_wheel_route = self.input.pending_gpu_surface_wheel.is_some()
            || self.input.pending_scroll_container_wheel.is_some();
        let had_deferred_surface_refresh = self.timing.deferred_surface_refresh;
        let had_deferred_scene_rebuild = self.timing.deferred_scene_rebuild;
        let had_deferred_scene_refresh = self.timing.deferred_surface_refresh_scope.is_some();
        let had_transient_overlay =
            self.core.has_transient_overlay_painter() || self.core.has_runtime_overlay_paint();
        self.flush_pending_scrollbar_drag_now();
        self.flush_pending_gpu_surface_wheel(&mut profile);
        self.flush_pending_scroll_container_wheel(&mut profile);
        self.refresh_deferred_surface_if_needed(&mut profile);
        let scene_rebuild_completed = self.rebuild_deferred_scene_if_needed(&mut profile);
        self.sync_deferred_auxiliary_windows_if_needed(event_loop, adapter);
        self.paint_transient_overlays(&mut profile);
        let frame_work = self.take_pending_frame_work();
        self.cpu_frame_observation_capture
            .record_frame_work(frame_work);
        self.cpu_frame_observation_capture.record_profile_stage(
            CpuFrameStage::CoalescedWheelRoute,
            had_coalesced_wheel_route,
            profile.record_timings,
            profile.coalesced_wheel_route,
        );
        let refresh_only_path = had_deferred_surface_refresh && !had_deferred_scene_rebuild;
        self.cpu_frame_observation_capture.record_profile_stage(
            CpuFrameStage::RefreshSurface,
            refresh_only_path,
            profile.record_timings,
            profile.refresh_surface,
        );
        self.cpu_frame_observation_capture.record_profile_stage(
            CpuFrameStage::PaintPlan,
            refresh_only_path,
            profile.record_timings,
            profile.paint_plan,
        );
        let immediate_scene_rebuild_completed =
            frame_work.needs_scene_rebuild() && !had_deferred_scene_rebuild;
        self.cpu_frame_observation_capture.record_stage(
            CpuFrameStage::PaintPlan,
            scene_rebuild_completed || immediate_scene_rebuild_completed,
            super::CpuFrameDuration::Unknown,
        );
        self.cpu_frame_observation_capture.record_profile_stage(
            CpuFrameStage::DeferredSceneRebuild,
            scene_rebuild_completed,
            profile.record_timings,
            profile.deferred_scene_rebuild,
        );
        if had_deferred_scene_refresh && !refresh_only_path {
            self.cpu_frame_observation_capture.record_stage(
                CpuFrameStage::RefreshSurface,
                true,
                super::CpuFrameDuration::Unknown,
            );
        }
        self.cpu_frame_observation_capture.record_profile_stage(
            CpuFrameStage::TransientOverlayPaint,
            had_transient_overlay,
            profile.record_timings,
            profile.transient_overlay_paint,
        );
        let render_resize_frame_directly = self.should_render_resize_frame_directly();
        if !self.admit_native_resources(adapter) {
            return Ok(NativeVisualRequestDisposition::DropPacket);
        }
        let Some(adapter_generation) = adapter.capture_generation() else {
            return Ok(NativeVisualRequestDisposition::DropPacket);
        };
        let pending_signal_summary_installs =
            self.reconcile_signal_summary_interests(adapter_generation);
        let target_generation = self.window.target_generation;
        let target_fenced = self.window.native_surface_target_fenced;
        // Volatile GPU updates are staged before the final stage-owner ticket
        // and before get_current_texture. Every veto below aborts this
        // snapshot; only a successful present commits it.
        self.core.runtime.snapshot_gpu_shader_presentation_updates();
        let Some(snapshot_revision) = self.allocate_native_frame_snapshot_revision() else {
            self.core.runtime.abort_gpu_shader_presentation_updates();
            return Ok(NativeVisualRequestDisposition::DropPacket);
        };
        let path = if render_resize_frame_directly {
            NativeEncodePresentPath::DirectResize
        } else {
            NativeEncodePresentPath::Composited
        };
        let Some(ticket) = self.admit_native_encode_present(
            packet_identity,
            adapter_generation,
            path,
            snapshot_revision,
        ) else {
            self.core.runtime.abort_gpu_shader_presentation_updates();
            return Ok(NativeVisualRequestDisposition::DropPacket);
        };
        let mut ticket = Some(ticket);
        let surface_texture = match self.acquire_present_surface_texture(adapter) {
            Ok(surface_texture) => {
                self.prepare_successful_surface_acquisition();
                surface_texture
            }
            Err(error) => {
                if let Some(ticket) = ticket.take() {
                    let _ = self.veto_native_encode_present(ticket);
                }
                self.core.runtime.abort_gpu_shader_presentation_updates();
                let disposition = self.handle_present_surface_acquire_error(
                    event_loop,
                    adapter,
                    requested_packet,
                    error,
                );
                return Ok(disposition);
            }
        };
        let Some(ticket_ref) = ticket.as_ref() else {
            self.core.runtime.abort_gpu_shader_presentation_updates();
            return Ok(NativeVisualRequestDisposition::DropPacket);
        };
        if !self.native_presentation_target_is_ready(adapter)
            || !self.native_encode_present_ticket_is_current(
                ticket_ref,
                packet_identity,
                adapter,
                path,
            )
        {
            if let Some(ticket) = ticket.take() {
                let _ = self.veto_native_encode_present(ticket);
            }
            self.core.runtime.abort_gpu_shader_presentation_updates();
            return Ok(NativeVisualRequestDisposition::DropPacket);
        }
        let mut gpu_timing_admission =
            if render_resize_frame_directly || !self.frame_gpu_timing_enabled {
                None
            } else {
                self.window
                    .native_resources
                    .as_mut()
                    .map(|resources| resources.reserve_gpu_timing())
            };
        self.start_native_gpu_timing(&mut gpu_timing_admission);
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let render_to_texture_required =
            render_resize_frame_directly || self.frame.scene_texture_dirty;
        let render_to_texture_elapsed = {
            let Some(resources) = self.window.native_resources.as_mut() else {
                self.cancel_native_gpu_timing(&mut gpu_timing_admission);
                if let Some(ticket) = ticket.take() {
                    let _ = self.veto_native_encode_present(ticket);
                }
                self.core.runtime.abort_gpu_shader_presentation_updates();
                return Ok(NativeVisualRequestDisposition::DropPacket);
            };
            let surface = &mut resources.render_surface;
            let Some(dev_handle) = adapter.device_handle_for_surface(surface) else {
                self.cancel_native_gpu_timing(&mut gpu_timing_admission);
                if let Some(ticket) = ticket.take() {
                    let _ = self.veto_native_encode_present(ticket);
                }
                self.core.runtime.abort_gpu_shader_presentation_updates();
                return Ok(NativeVisualRequestDisposition::DropPacket);
            };
            let mut scene_texture_context = SceneTextureContext {
                renderer: &mut resources.renderer,
                completion_witness: &mut resources.completion_witness,
                device: &dev_handle.device,
                queue: &dev_handle.queue,
                surface,
                dpi_scale: self.window.dpi_scale,
                record_timing: profile.record_timings,
            };
            let mut render = || -> Result<_, NativeFrameRenderFailure> {
                if render_resize_frame_directly {
                    Ok(render_scene_to_surface_view(
                        &mut self.frame,
                        &mut scene_texture_context,
                        &surface_view,
                    )?)
                } else {
                    Ok(render_scene_texture_if_needed(
                        &mut self.frame,
                        &mut scene_texture_context,
                    )?)
                }
            };
            render()
        };
        let render_to_texture_elapsed = match render_to_texture_elapsed {
            Ok(elapsed) => elapsed,
            Err(failure) => {
                self.cancel_native_gpu_timing(&mut gpu_timing_admission);
                if let Some(ticket) = ticket.take() {
                    let _ = self.veto_native_encode_present(ticket);
                }
                self.core.runtime.abort_gpu_shader_presentation_updates();
                return Err(failure);
            }
        };
        self.cpu_frame_observation_capture.record_profile_stage(
            CpuFrameStage::RenderToTexture,
            render_to_texture_required,
            profile.record_timings,
            render_to_texture_elapsed,
        );
        if render_resize_frame_directly {
            let Some(ticket_ref) = ticket.as_ref() else {
                self.core.runtime.abort_gpu_shader_presentation_updates();
                return Ok(NativeVisualRequestDisposition::DropPacket);
            };
            if !self.native_encode_present_ticket_is_current(
                ticket_ref,
                packet_identity,
                adapter,
                path,
            ) {
                if let Some(ticket) = ticket.take() {
                    let _ = self.veto_native_encode_present(ticket);
                }
                self.core.runtime.abort_gpu_shader_presentation_updates();
                return Ok(NativeVisualRequestDisposition::DropPacket);
            }
            let (_, elapsed) = profile.measure(|| surface_texture.present());
            profile.submit_present = elapsed;
            let Some(ticket) = ticket.take() else {
                self.core.runtime.abort_gpu_shader_presentation_updates();
                return Ok(NativeVisualRequestDisposition::DropPacket);
            };
            if !self.complete_native_encode_present(ticket) {
                self.core.runtime.abort_gpu_shader_presentation_updates();
                return Ok(NativeVisualRequestDisposition::DropPacket);
            }
            self.core.runtime.commit_gpu_shader_presentation_updates();
            profile.frame_sequence = self.timing.allocate_frame_sequence();
            self.contribute_render_canvas_uploads(
                adapter,
                profile.frame_sequence,
                Default::default(),
                None,
                None,
            );
            let input_to_present_latency_us =
                self.timing.take_input_to_present_latency_us(Instant::now());
            let application_atlas_residency =
                self.capture_atlas_residency_profile(adapter, profile_enabled);
            let application_signal_residency =
                self.capture_signal_residency_profile(adapter, profile_enabled);
            let application_custom_shader_residency =
                self.capture_custom_shader_residency_profile(adapter, profile_enabled);
            let application_render_canvas_uploads =
                self.capture_render_canvas_upload_profile(adapter, profile_enabled);
            self.finish_direct_resize_present(
                render_to_texture_elapsed,
                profile,
                profile_enabled,
                diagnostics_requested,
                frame_work,
                input_to_present_latency_us,
                application_atlas_residency,
                application_signal_residency,
                application_custom_shader_residency,
                application_render_canvas_uploads,
            );
            return Ok(NativeVisualRequestDisposition::Presented);
        }
        let Some(ticket_ref) = ticket.as_ref() else {
            self.cancel_native_gpu_timing(&mut gpu_timing_admission);
            self.core.runtime.abort_gpu_shader_presentation_updates();
            return Ok(NativeVisualRequestDisposition::DropPacket);
        };
        if !self.native_encode_present_ticket_is_current(ticket_ref, packet_identity, adapter, path)
        {
            self.cancel_native_gpu_timing(&mut gpu_timing_admission);
            if let Some(ticket) = ticket.take() {
                let _ = self.veto_native_encode_present(ticket);
            }
            self.core.runtime.abort_gpu_shader_presentation_updates();
            return Ok(NativeVisualRequestDisposition::DropPacket);
        }
        let Some(dev_handle) = self
            .window
            .native_resources
            .as_ref()
            .and_then(|resources| adapter.device_handle_for_surface(&resources.render_surface))
        else {
            self.cancel_native_gpu_timing(&mut gpu_timing_admission);
            if let Some(ticket) = ticket.take() {
                let _ = self.veto_native_encode_present(ticket);
            }
            self.core.runtime.abort_gpu_shader_presentation_updates();
            return Ok(NativeVisualRequestDisposition::DropPacket);
        };
        let encode_present_plan_context = ticket_ref.plan_context();
        let upload_plan_context = self.window.native_resources.as_ref().and_then(|resources| {
            GpuSurfaceRenderCanvasUploadPlanContext::new(
                encode_present_plan_context,
                resources.generation,
                GpuSurfaceRenderCanvasUploadTarget::new(
                    wgpu_device_id(&dev_handle.device),
                    resources.render_surface.config.format,
                    resources.render_surface.config.width,
                    resources.render_surface.config.height,
                ),
            )
        });
        let mut encoder =
            dev_handle
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("generic_native_vello_present_blit"),
                });
        let presentation_updates = self.core.runtime.staged_gpu_shader_presentation_updates();
        let started = profile.record_timings.then(Instant::now);
        let gpu_surface_stats = {
            let Some(resources) = self.window.native_resources.as_mut() else {
                self.cancel_native_gpu_timing(&mut gpu_timing_admission);
                if let Some(ticket) = ticket.take() {
                    let _ = self.veto_native_encode_present(ticket);
                }
                self.core.runtime.abort_gpu_shader_presentation_updates();
                return Ok(NativeVisualRequestDisposition::DropPacket);
            };
            let resource_generation = resources.generation;
            let completion_identity = resources.completion_witness.retirement_identity();
            let surface = &mut resources.render_surface;
            let gpu_resources = &mut resources.gpu_resources;
            for pending in pending_signal_summary_installs {
                let _ = gpu_resources
                    .gpu_surface_renderer
                    .install_prepared_signal_summary(
                        pending.surface_key,
                        pending.revision,
                        &pending.content,
                        pending.prepared,
                    );
            }
            let request = BaseFramePresentRequest {
                paint_plan: &self.frame.last_paint_plan,
                occlusion_plan: &self.frame.surface_occlusion_plan,
                transient_overlay_primitives: &self.frame.transient_overlay_primitives,
                presentation_updates,
                collect_upload_plan: profile_enabled,
                upload_plan_context,
            };
            let gpu_surface_stats = present_base_frame(
                &mut BaseFramePresentState {
                    base_frame: &mut gpu_resources.composited_base_frame,
                    retired_base_frame: &mut gpu_resources.composited_base_frame_retirement,
                    base_dirty: &mut self.frame.composited_base_dirty,
                    gpu_surface_renderer: &mut gpu_resources.gpu_surface_renderer,
                    profile: &mut profile,
                },
                surface,
                &mut BaseFramePresentTarget {
                    device: &dev_handle.device,
                    queue: &dev_handle.queue,
                    encoder: &mut encoder,
                    surface_view: &surface_view,
                    dpi_scale: self.window.dpi_scale,
                    adapter_generation,
                    resource_generation,
                    target_generation,
                    target_fenced,
                    completion_identity,
                },
                &request,
            );
            profile.full_screen_blit = started.map(|started| started.elapsed()).unwrap_or_default();
            if self.frame.has_post_gpu_overlay_work() {
                let surface_size = RenderSurfacePixelSize::from_surface(surface);
                self.frame.render_post_gpu_overlay(
                    &mut gpu_resources.post_gpu_overlay_renderer,
                    &mut post_gpu_overlay::PostGpuOverlayRenderTarget {
                        device: &dev_handle.device,
                        queue: &dev_handle.queue,
                        encoder: &mut encoder,
                        target_view: &surface_view,
                        format: surface.config.format,
                        size: surface_size.logical_size(self.window.dpi_scale),
                    },
                );
            }
            gpu_resources
                .gpu_surface_renderer
                .finish_presentation_staging_belt();
            gpu_surface_stats
        };
        self.cpu_frame_observation_capture.record_profile_stage(
            CpuFrameStage::CompositedBaseRefresh,
            !self.frame.transient_overlay_primitives.is_empty()
                && !profile.composited_base_cache_hit,
            profile.record_timings,
            profile.composited_base_refresh,
        );
        self.cpu_frame_observation_capture.record_profile_stage(
            CpuFrameStage::FullScreenBlit,
            true,
            profile.record_timings,
            profile.full_screen_blit,
        );
        let Some(ticket_ref) = ticket.as_ref() else {
            self.cancel_native_gpu_timing(&mut gpu_timing_admission);
            self.core.runtime.abort_gpu_shader_presentation_updates();
            return Ok(NativeVisualRequestDisposition::DropPacket);
        };
        if !self.native_encode_present_ticket_is_current(ticket_ref, packet_identity, adapter, path)
        {
            self.cancel_native_gpu_timing(&mut gpu_timing_admission);
            if let Some(ticket) = ticket.take() {
                let _ = self.veto_native_encode_present(ticket);
            }
            self.core.runtime.abort_gpu_shader_presentation_updates();
            return Ok(NativeVisualRequestDisposition::DropPacket);
        }
        if let Some(GpuTimingAdmission::Reserved(reservation)) = gpu_timing_admission {
            let encoded = self
                .window
                .native_resources
                .as_mut()
                .is_some_and(|resources| {
                    resources.encode_gpu_timing_end(reservation, &mut encoder)
                });
            if !encoded {
                self.cancel_native_gpu_timing(&mut gpu_timing_admission);
                self.core.runtime.abort_gpu_shader_presentation_updates();
                return Ok(NativeVisualRequestDisposition::DropPacket);
            }
        }
        let (_, elapsed) = profile.measure(|| {
            dev_handle.queue.submit(std::iter::once(encoder.finish()));
            if let Some(resources) = self.window.native_resources.as_mut() {
                resources
                    .gpu_resources
                    .gpu_surface_renderer
                    .recall_presentation_staging_belt();
            }
            self.record_successful_native_submission();
            surface_texture.present();
        });
        let application_atlas_residency =
            self.capture_atlas_residency_profile(adapter, profile_enabled);
        let application_signal_residency =
            self.capture_signal_residency_profile(adapter, profile_enabled);
        let application_custom_shader_residency =
            self.capture_custom_shader_residency_profile(adapter, profile_enabled);
        let Some(ticket) = ticket.take() else {
            self.cancel_native_gpu_timing(&mut gpu_timing_admission);
            self.core.runtime.abort_gpu_shader_presentation_updates();
            return Ok(NativeVisualRequestDisposition::DropPacket);
        };
        if !self.complete_native_encode_present(ticket) {
            self.cancel_native_gpu_timing(&mut gpu_timing_admission);
            self.core.runtime.abort_gpu_shader_presentation_updates();
            return Ok(NativeVisualRequestDisposition::DropPacket);
        }
        self.core.runtime.commit_gpu_shader_presentation_updates();
        profile.submit_present = elapsed;
        profile.frame_sequence = self.timing.allocate_frame_sequence();
        let mut current_plan_context = None;
        self.contribute_render_canvas_uploads(
            adapter,
            profile.frame_sequence,
            gpu_surface_stats.render_canvas_uploads,
            match gpu_surface_stats.render_canvas_upload_plan {
                Some(observation) => {
                    if let Some(context) = self.current_render_canvas_upload_plan_context(
                        adapter,
                        encode_present_plan_context,
                    ) {
                        current_plan_context = Some(context);
                        Some(GpuSurfaceRenderCanvasUploadPlan::from_observation(
                            context,
                            observation,
                        ))
                    } else {
                        None
                    }
                }
                None => None,
            },
            current_plan_context,
        );
        let application_render_canvas_uploads =
            self.capture_render_canvas_upload_profile(adapter, profile_enabled);
        let timing_readback_submitted =
            if let Some(GpuTimingAdmission::Reserved(reservation)) = gpu_timing_admission {
                self.window
                    .native_resources
                    .as_mut()
                    .is_some_and(|resources| resources.submit_gpu_timing_readback(reservation))
            } else {
                true
            };
        if !timing_readback_submitted {
            self.cancel_native_gpu_timing(&mut gpu_timing_admission);
        }
        self.finalize_native_gpu_timing(gpu_timing_admission.take(), profile.frame_sequence);
        let now = Instant::now();
        let input_to_present_latency_us = self.timing.take_input_to_present_latency_us(now);
        self.cpu_frame_observation_capture.record_profile_stage(
            CpuFrameStage::SubmitPresent,
            true,
            profile.record_timings,
            profile.submit_present,
        );
        self.cpu_frame_observation_capture
            .mark_successful_presentation();
        let text_stats = if profile_enabled || diagnostics_requested {
            self.frame.text_renderer.take_layout_profile_counters()
        } else {
            Default::default()
        };
        let since_last_present = now.duration_since(self.timing.last_redraw);
        if profile_enabled {
            maybe_log_render_profile(
                "present",
                self.frame.last_scene_stats,
                text_stats,
                render_to_texture_elapsed,
                profile,
                NativeRenderProfileGpuSurface {
                    stats: gpu_surface_stats,
                    atlas_residency: self.window.atlas_residency_snapshots(),
                    signal_residency: self.window.signal_residency_snapshots(),
                    custom_shader_residency: self.window.custom_shader_residency_snapshots(),
                    composited_base_residency: self.window.composited_base_residency_snapshots(),
                    target_residency: self.window.target_residency_snapshots(),
                    application_atlas_residency,
                    application_signal_residency,
                    application_custom_shader_residency,
                    application_render_canvas_uploads,
                },
                since_last_present,
            );
        }
        maybe_log_slow_render_profile(
            "present",
            self.frame.last_scene_stats,
            render_to_texture_elapsed,
            profile,
            gpu_surface_stats,
            application_render_canvas_uploads,
            since_last_present,
        );
        let frame_refresh = self.core.runtime.take_frame_refresh_diagnostics();
        let surface_refresh = frame_refresh.refresh;
        let surface_refresh_total = frame_refresh.total;
        self.cpu_frame_observation_capture
            .record_refresh_diagnostics(
                surface_refresh,
                surface_refresh_total,
                frame_refresh.effective_scope,
            );
        if diagnostics_requested {
            let diagnostics = native_frame_diagnostics(NativeFrameDiagnosticsParts {
                stats: self.frame.last_scene_stats,
                scene_encode_count: self.frame.scene_encode_count,
                scene_reuse_count: self.frame.scene_reuse_count,
                scene_assembly_count: self.frame.scene_assembly_count,
                scene_assembly_veto_count: self.frame.scene_assembly_veto_count,
                scene_mixed_assembly_count: self.frame.scene_mixed_assembly_count,
                scene_assembly_fresh_count: self.frame.scene_assembly_fresh_count,
                scene_assembly_reused_count: self.frame.scene_assembly_reused_count,
                scene_assembly_append_count: self.frame.scene_assembly_append_count,
                scene_build_outcome: self.frame.scene_build_outcome.name(),
                text_stats,
                retained_policy: self.frame.retained_surface_cache.policy(),
                retained_entries: self.frame.retained_surface_cache.entry_count(),
                gpu_surface_stats,
                profile,
                input_to_present_latency_us,
                render_to_texture_elapsed,
                since_last_present,
                frame_work,
                surface_refresh,
                surface_refresh_total,
                surface_recovery: self.window.surface_recovery.diagnostics(),
            });
            self.stage_frame_diagnostics(diagnostics);
        }
        self.timing.last_redraw = now;
        self.mark_first_presented();
        Ok(NativeVisualRequestDisposition::Presented)
    }

    pub(super) fn redraw_and_exit_on_error(&mut self, event_loop: &ActiveEventLoop) {
        let Some(mut adapter) = self.adapter.take() else {
            let _ = self.veto_native_visual_request_at_callback_boundary();
            return;
        };
        let packet = match self.begin_native_visual_request(&adapter) {
            NativeVisualRequestBegin::Requested(packet) => Some((packet, true)),
            NativeVisualRequestBegin::UnsolicitedFallback(packet) => Some((packet, false)),
            NativeVisualRequestBegin::Stale => {
                self.clear_native_visual_request_wake();
                None
            }
            NativeVisualRequestBegin::RequestedVetoed
            | NativeVisualRequestBegin::WrongWindow
            | NativeVisualRequestBegin::Ineligible
            | NativeVisualRequestBegin::Exhausted => None,
        };
        let Some((packet, requested_packet)) = packet else {
            self.adapter = Some(adapter);
            return;
        };
        let packet_identity = packet.identity();
        let admission =
            self.begin_cpu_frame_observation(super::FrameScheduleKey::Primary, Instant::now());
        let result = self.redraw(event_loop, &mut adapter, requested_packet, packet_identity);
        let (disposition, redraw_failed) = match result {
            Ok(disposition) => (disposition, false),
            Err(failure) => {
                // `redraw` has returned, so its acquired SurfaceTexture is gone
                // before reconstruction can touch the native bundle.
                self.mark_cpu_frame_observation_recovery();
                let _ = self.recover_frame_render_failure(
                    event_loop,
                    &mut adapter,
                    failure,
                    super::NativeRendererRecoveryWindowKind::Primary,
                );
                (NativeVisualRequestDisposition::DropPacket, true)
            }
        };
        let _ = self.finish_native_visual_request(packet, disposition);
        self.finish_cpu_frame_observation(admission, redraw_failed);
        self.adapter = Some(adapter);
    }

    pub(super) fn should_render_resize_frame_directly(&self) -> bool {
        // A native window resize presents a wgpu SurfaceTexture whose usage is
        // limited to render attachment.  Vello's storage-backed render target
        // is the Radiant-owned scene texture, which is then composed into the
        // acquired surface by `present_base_frame`.
        false
    }

    // This private orchestration helper keeps the direct-resize present state explicit.
    #[allow(clippy::too_many_arguments)]
    fn finish_direct_resize_present(
        &mut self,
        render_to_texture_elapsed: std::time::Duration,
        profile: RenderFrameProfile,
        profile_enabled: bool,
        diagnostics_requested: bool,
        frame_work: super::FrameWork,
        input_to_present_latency_us: Option<u64>,
        application_atlas_residency: NativeAdapterAtlasResidencyProfile,
        application_signal_residency: NativeAdapterSignalResidencyProfile,
        application_custom_shader_residency: NativeAdapterCustomShaderResidencyProfile,
        application_render_canvas_uploads: NativeAdapterRenderCanvasUploadProfile,
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
                NativeRenderProfileGpuSurface {
                    stats: gpu_surface_stats,
                    atlas_residency: self.window.atlas_residency_snapshots(),
                    signal_residency: self.window.signal_residency_snapshots(),
                    custom_shader_residency: self.window.custom_shader_residency_snapshots(),
                    composited_base_residency: self.window.composited_base_residency_snapshots(),
                    target_residency: self.window.target_residency_snapshots(),
                    application_atlas_residency,
                    application_signal_residency,
                    application_custom_shader_residency,
                    application_render_canvas_uploads,
                },
                since_last_present,
            );
        }
        maybe_log_slow_render_profile(
            "present.resize_direct",
            self.frame.last_scene_stats,
            render_to_texture_elapsed,
            profile,
            gpu_surface_stats,
            application_render_canvas_uploads,
            since_last_present,
        );
        let frame_refresh = self.core.runtime.take_frame_refresh_diagnostics();
        let surface_refresh = frame_refresh.refresh;
        let surface_refresh_total = frame_refresh.total;
        self.cpu_frame_observation_capture
            .record_refresh_diagnostics(
                surface_refresh,
                surface_refresh_total,
                frame_refresh.effective_scope,
            );
        self.cpu_frame_observation_capture.record_profile_stage(
            CpuFrameStage::SubmitPresent,
            true,
            profile.record_timings,
            profile.submit_present,
        );
        self.cpu_frame_observation_capture
            .mark_successful_presentation();
        if diagnostics_requested {
            let diagnostics = native_frame_diagnostics(NativeFrameDiagnosticsParts {
                stats: self.frame.last_scene_stats,
                scene_encode_count: self.frame.scene_encode_count,
                scene_reuse_count: self.frame.scene_reuse_count,
                scene_assembly_count: self.frame.scene_assembly_count,
                scene_assembly_veto_count: self.frame.scene_assembly_veto_count,
                scene_mixed_assembly_count: self.frame.scene_mixed_assembly_count,
                scene_assembly_fresh_count: self.frame.scene_assembly_fresh_count,
                scene_assembly_reused_count: self.frame.scene_assembly_reused_count,
                scene_assembly_append_count: self.frame.scene_assembly_append_count,
                scene_build_outcome: self.frame.scene_build_outcome.name(),
                text_stats,
                retained_policy: self.frame.retained_surface_cache.policy(),
                retained_entries: self.frame.retained_surface_cache.entry_count(),
                gpu_surface_stats,
                profile,
                input_to_present_latency_us,
                render_to_texture_elapsed,
                since_last_present,
                frame_work,
                surface_refresh,
                surface_refresh_total,
                surface_recovery: self.window.surface_recovery.diagnostics(),
            });
            self.stage_frame_diagnostics(diagnostics);
        }
        self.timing.last_redraw = now;
        self.mark_first_presented();
    }

    fn mark_first_presented(&mut self) {
        if !self.timing.first_frame_presented {
            self.timing.first_frame_presented = true;
            if reveal_window_after_first_present(&self.options) {
                self.set_native_window_visibility(true);
                self.timing.startup_timing.mark_window_revealed();
            }
            if hide_window_after_first_present(&self.options) {
                self.set_native_window_visibility(false);
            }
            self.timing.startup_timing.mark_first_presented();
            self.timing.startup_timing.maybe_emit_summary();
        }
    }
}
