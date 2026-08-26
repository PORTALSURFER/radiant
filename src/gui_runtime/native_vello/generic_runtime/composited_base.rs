//! Cached composed frame used by paint-only transient overlay presentations.

use super::runtime_helpers::SurfaceOcclusionPlan;
use super::submission_completion::NativeSubmissionCompletionIdentity;
use super::{GpuSurfaceRenderer, RenderFrameProfile, RenderSurfacePixelSize, gpu_surface};
#[cfg(test)]
use crate::gui::types::{Point, Rect as UiRect, Rgba8, Vector2};
use crate::runtime::{GpuShaderPresentationUniformUpdate, PaintPrimitive, SurfacePaintPlan};
use vello::{util::RenderSurface, wgpu};

mod frame;
pub(super) use frame::{
    CompositedBaseFrame, CompositedBaseFrameEnsureOutcome, CompositedBaseFrameEnsureRequest,
    CompositedBaseFrameRetirement, CompositedBaseFrameRetirementIdentity,
};

pub(super) struct BaseFramePresentTarget<'a> {
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) encoder: &'a mut wgpu::CommandEncoder,
    pub(super) surface_view: &'a wgpu::TextureView,
    pub(super) dpi_scale: crate::theme::DpiScale,
    pub(super) adapter_generation: super::NativeAdapterGeneration,
    pub(super) resource_generation: super::NativeAdapterGeneration,
    pub(super) target_generation: super::runner_state::NativeTargetGeneration,
    pub(super) target_fenced: bool,
    pub(super) completion_identity: Option<NativeSubmissionCompletionIdentity>,
}

pub(super) struct BaseFramePresentState<'a> {
    pub(super) base_frame: &'a mut Option<CompositedBaseFrame>,
    pub(super) retired_base_frame: &'a mut Option<CompositedBaseFrameRetirement>,
    pub(super) base_dirty: &'a mut bool,
    pub(super) gpu_surface_renderer: &'a mut GpuSurfaceRenderer,
    pub(super) profile: &'a mut RenderFrameProfile,
}

struct BaseFrameRefreshState<'a> {
    base_dirty: &'a mut bool,
    gpu_surface_renderer: &'a mut GpuSurfaceRenderer,
    profile: &'a mut RenderFrameProfile,
}

pub(super) struct BaseFramePresentRequest<'a> {
    pub(super) paint_plan: &'a SurfacePaintPlan,
    pub(super) occlusion_plan: &'a SurfaceOcclusionPlan,
    pub(super) transient_overlay_primitives: &'a [PaintPrimitive],
    pub(super) presentation_updates: &'a [GpuShaderPresentationUniformUpdate],
    pub(super) collect_upload_plan: bool,
    pub(super) upload_plan_context: Option<gpu_surface::GpuSurfaceRenderCanvasUploadPlanContext>,
}

fn preflight_render_canvas_upload_plan(
    renderer: &mut GpuSurfaceRenderer,
    context: gpu_surface::GpuSurfaceRenderCanvasUploadPlanContext,
    primitives: &[PaintPrimitive],
    dpi_scale: crate::theme::DpiScale,
    presentation_updates: &[GpuShaderPresentationUniformUpdate],
) -> gpu_surface::GpuSurfaceRenderCanvasUploadPlan {
    renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
        context,
        primitives,
        dpi_scale,
        presentation_updates,
    )
}

pub(super) fn present_base_frame(
    state: &mut BaseFramePresentState<'_>,
    surface: &RenderSurface<'_>,
    target: &mut BaseFramePresentTarget<'_>,
    request: &BaseFramePresentRequest<'_>,
) -> gpu_surface::GpuSurfaceRenderStats {
    if !should_use_composited_base(request.transient_overlay_primitives) {
        return present_live_base(state.gpu_surface_renderer, surface, target, request);
    }

    let ensure_outcome = CompositedBaseFrame::ensure(
        state.base_frame,
        state.retired_base_frame,
        CompositedBaseFrameEnsureRequest {
            device: target.device,
            width: surface.config.width,
            height: surface.config.height,
            format: surface.config.format,
            adapter_generation: target.adapter_generation,
            resource_generation: target.resource_generation,
            target_generation: target.target_generation,
            target_fenced: target.target_fenced,
            completion_identity: target.completion_identity,
        },
    );
    let frame_recreated = match ensure_outcome {
        CompositedBaseFrameEnsureOutcome::Reused => false,
        CompositedBaseFrameEnsureOutcome::Created => true,
        CompositedBaseFrameEnsureOutcome::Vetoed => {
            *state.base_dirty = true;
            return present_live_base(state.gpu_surface_renderer, surface, target, request);
        }
    };
    let Some(frame) = state.base_frame.as_ref() else {
        return present_live_base(state.gpu_surface_renderer, surface, target, request);
    };
    let needs_refresh = composited_base_needs_refresh(
        *state.base_dirty,
        frame_recreated,
        !request.presentation_updates.is_empty(),
    );
    let stats = if needs_refresh {
        let refresh_state = BaseFrameRefreshState {
            base_dirty: state.base_dirty,
            gpu_surface_renderer: state.gpu_surface_renderer,
            profile: state.profile,
        };
        refresh_composited_base_frame(frame, refresh_state, surface, target, request)
    } else {
        composited_base_cache_hit_stats(state.profile, request)
    };
    surface.blitter.copy(
        target.device,
        target.encoder,
        &frame.view,
        target.surface_view,
    );
    stats
}

fn present_live_base(
    gpu_surface_renderer: &mut GpuSurfaceRenderer,
    surface: &RenderSurface<'_>,
    target: &mut BaseFramePresentTarget<'_>,
    request: &BaseFramePresentRequest<'_>,
) -> gpu_surface::GpuSurfaceRenderStats {
    surface.blitter.copy(
        target.device,
        target.encoder,
        &surface.target_view,
        target.surface_view,
    );
    let surface_size = RenderSurfacePixelSize::from_surface(surface);
    let upload_plan_context = upload_plan_context(request);
    let upload_plan = upload_plan_context.map(|context| {
        preflight_render_canvas_upload_plan(
            gpu_surface_renderer,
            context,
            &request.paint_plan.primitives,
            target.dpi_scale,
            request.presentation_updates,
        )
    });
    gpu_surface_renderer.render(
        &mut gpu_surface::GpuSurfaceRenderTarget {
            device: target.device,
            queue: target.queue,
            encoder: target.encoder,
            target_view: target.surface_view,
            format: surface.config.format,
            size: surface_size.physical_size(),
            dpi_scale: target.dpi_scale,
            upload_plan_context,
            upload_plan,
            collect_upload_plan: request.collect_upload_plan,
        },
        &request.paint_plan.primitives,
        request.occlusion_plan,
        request.presentation_updates,
    )
}

fn refresh_composited_base_frame(
    frame: &CompositedBaseFrame,
    state: BaseFrameRefreshState<'_>,
    surface: &RenderSurface<'_>,
    target: &mut BaseFramePresentTarget<'_>,
    request: &BaseFramePresentRequest<'_>,
) -> gpu_surface::GpuSurfaceRenderStats {
    let (stats, elapsed) = state.profile.measure(|| {
        surface.blitter.copy(
            target.device,
            target.encoder,
            &surface.target_view,
            &frame.view,
        );
        let surface_size = RenderSurfacePixelSize::from_surface(surface);
        let upload_plan_context = upload_plan_context(request);
        let upload_plan = upload_plan_context.map(|context| {
            preflight_render_canvas_upload_plan(
                state.gpu_surface_renderer,
                context,
                &request.paint_plan.primitives,
                target.dpi_scale,
                request.presentation_updates,
            )
        });
        state.gpu_surface_renderer.render(
            &mut gpu_surface::GpuSurfaceRenderTarget {
                device: target.device,
                queue: target.queue,
                encoder: target.encoder,
                target_view: &frame.view,
                format: surface.config.format,
                size: surface_size.physical_size(),
                dpi_scale: target.dpi_scale,
                upload_plan_context,
                upload_plan,
                collect_upload_plan: request.collect_upload_plan,
            },
            &request.paint_plan.primitives,
            request.occlusion_plan,
            request.presentation_updates,
        )
    });
    *state.base_dirty = false;
    state.profile.composited_base_refresh = elapsed;
    stats
}

fn composited_base_needs_refresh(
    base_dirty: bool,
    frame_recreated: bool,
    has_presentation_updates: bool,
) -> bool {
    base_dirty || frame_recreated || has_presentation_updates
}

fn should_use_composited_base(transient_overlay_primitives: &[PaintPrimitive]) -> bool {
    !transient_overlay_primitives.is_empty()
}

fn composited_base_cache_hit_stats(
    profile: &mut RenderFrameProfile,
    request: &BaseFramePresentRequest<'_>,
) -> gpu_surface::GpuSurfaceRenderStats {
    profile.composited_base_cache_hit = true;
    if request.collect_upload_plan {
        gpu_surface::GpuSurfaceRenderStats::with_upload_plan(upload_plan_context(request))
    } else {
        gpu_surface::GpuSurfaceRenderStats::default()
    }
}

fn upload_plan_context(
    request: &BaseFramePresentRequest<'_>,
) -> Option<gpu_surface::GpuSurfaceRenderCanvasUploadPlanContext> {
    request.upload_plan_context
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;

    use super::*;
    use crate::gui_runtime::native_vello::generic_runtime::FrameWork;
    use crate::gui_runtime::native_vello::generic_runtime::adapter::NativeAdapterGeneration;
    use crate::gui_runtime::native_vello::generic_runtime::closing::NativeLifecycle;
    use crate::gui_runtime::native_vello::generic_runtime::native_encode_present::{
        NativeEncodePresentPath, NativeEncodePresentPlanContext,
    };
    use crate::gui_runtime::native_vello::generic_runtime::native_visual_packet::{
        NativeVisualRequestAdapter, NativeVisualRequestBegin, NativeVisualRequestMailbox,
    };
    use crate::gui_runtime::native_vello::generic_runtime::runner_state::NativeTargetGeneration;
    use winit::window::WindowId;

    #[test]
    fn composited_base_refreshes_when_dirty_recreated_or_updated() {
        assert!(composited_base_needs_refresh(true, false, false));
        assert!(composited_base_needs_refresh(false, true, false));
        assert!(composited_base_needs_refresh(true, true, false));
        assert!(composited_base_needs_refresh(false, false, true));
        assert!(!composited_base_needs_refresh(false, false, false));
    }

    #[test]
    fn present_base_frame_uses_live_path_without_transient_overlays() {
        assert!(!should_use_composited_base(&[]));
        assert!(should_use_composited_base(&[PaintPrimitive::FillRect(
            crate::runtime::PaintFillRect {
                widget_id: 1,
                rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
                color: Rgba8 {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                },
            }
        )]));
    }

    #[test]
    fn upload_plan_context_and_cache_hit_observation_follow_collection_mode() {
        let mut mailbox = NativeVisualRequestMailbox::new();
        let window_id = WindowId::dummy();
        assert!(mailbox.bind_window(window_id));
        let _ = mailbox.enqueue_for_test(FrameWork::None);
        let packet = match NativeVisualRequestAdapter::begin(&mut mailbox, window_id, true) {
            NativeVisualRequestBegin::Requested(packet) => packet.identity(),
            other => panic!("unexpected packet begin: {other:?}"),
        };
        let context = gpu_surface::GpuSurfaceRenderCanvasUploadPlanContext::new(
            NativeEncodePresentPlanContext {
                packet,
                adapter_generation: NativeAdapterGeneration::from_test_serial(1),
                target_generation: NativeTargetGeneration::from_test_serial(1),
                lifecycle: NativeLifecycle::default(),
                path: NativeEncodePresentPath::Composited,
                snapshot_revision: NonZeroU64::MIN,
            },
            NativeAdapterGeneration::from_test_serial(1),
            gpu_surface::GpuSurfaceRenderCanvasUploadTarget::new(
                1,
                wgpu::TextureFormat::Rgba8Unorm,
                64,
                32,
            ),
        )
        .expect("valid upload-plan context");
        let theme = crate::theme::ThemeTokens::default();
        let paint_plan = SurfacePaintPlan::empty(&theme);
        let occlusion_plan = SurfaceOcclusionPlan::default();
        for collect_upload_plan in [false, true] {
            let mut profile = RenderFrameProfile::default();
            let request = BaseFramePresentRequest {
                paint_plan: &paint_plan,
                occlusion_plan: &occlusion_plan,
                transient_overlay_primitives: &[],
                presentation_updates: &[],
                collect_upload_plan,
                upload_plan_context: Some(context),
            };

            assert_eq!(upload_plan_context(&request), Some(context));
            let stats = composited_base_cache_hit_stats(&mut profile, &request);
            assert!(profile.composited_base_cache_hit);
            assert_eq!(
                stats.render_canvas_upload_plan,
                collect_upload_plan
                    .then_some(gpu_surface::GpuSurfaceRenderCanvasUploadPlanObservation::NoWork)
            );
        }
    }
}
