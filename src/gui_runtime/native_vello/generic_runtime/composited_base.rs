//! Cached composed frame used by paint-only transient overlay presentations.

use super::runtime_helpers::SurfaceOcclusionPlan;
use super::{GpuSurfaceRenderer, RenderFrameProfile, RenderSurfacePixelSize, gpu_surface};
#[cfg(test)]
use crate::gui::types::{Point, Rect as UiRect, Rgba8, Vector2};
use crate::runtime::{PaintPrimitive, SurfacePaintPlan};
use vello::{util::RenderSurface, wgpu};

mod frame;
pub(super) use frame::CompositedBaseFrame;

pub(super) struct BaseFramePresentTarget<'a> {
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) encoder: &'a mut wgpu::CommandEncoder,
    pub(super) surface_view: &'a wgpu::TextureView,
    pub(super) dpi_scale: crate::theme::DpiScale,
}

pub(super) struct BaseFramePresentState<'a> {
    pub(super) base_frame: &'a mut Option<CompositedBaseFrame>,
    pub(super) base_dirty: &'a mut bool,
    pub(super) gpu_surface_renderer: &'a mut GpuSurfaceRenderer,
    pub(super) profile: &'a mut RenderFrameProfile,
}

struct BaseFrameRefreshState<'a> {
    base_dirty: &'a mut bool,
    gpu_surface_renderer: &'a mut GpuSurfaceRenderer,
    profile: &'a mut RenderFrameProfile,
}

pub(super) fn present_base_frame(
    state: &mut BaseFramePresentState<'_>,
    surface: &RenderSurface<'_>,
    target: &mut BaseFramePresentTarget<'_>,
    paint_plan: &SurfacePaintPlan,
    occlusion_plan: &SurfaceOcclusionPlan,
    transient_overlay_primitives: &[PaintPrimitive],
    has_gpu_surfaces: bool,
) -> gpu_surface::GpuSurfaceRenderStats {
    if !should_use_composited_base(transient_overlay_primitives) {
        return present_live_base(
            state.gpu_surface_renderer,
            surface,
            target,
            paint_plan,
            occlusion_plan,
            has_gpu_surfaces,
        );
    }

    let (frame, frame_recreated) = CompositedBaseFrame::ensure(
        state.base_frame,
        target.device,
        surface.config.width,
        surface.config.height,
        surface.config.format,
    );
    let needs_refresh = composited_base_needs_refresh(*state.base_dirty, frame_recreated);
    let stats = if needs_refresh {
        let refresh_state = BaseFrameRefreshState {
            base_dirty: state.base_dirty,
            gpu_surface_renderer: state.gpu_surface_renderer,
            profile: state.profile,
        };
        refresh_composited_base_frame(
            frame,
            refresh_state,
            surface,
            target,
            paint_plan,
            occlusion_plan,
            has_gpu_surfaces,
        )
    } else {
        state.profile.composited_base_cache_hit = true;
        gpu_surface::GpuSurfaceRenderStats::default()
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
    paint_plan: &SurfacePaintPlan,
    occlusion_plan: &SurfaceOcclusionPlan,
    has_gpu_surfaces: bool,
) -> gpu_surface::GpuSurfaceRenderStats {
    surface.blitter.copy(
        target.device,
        target.encoder,
        &surface.target_view,
        target.surface_view,
    );
    if !should_render_gpu_surfaces(has_gpu_surfaces) {
        return gpu_surface::GpuSurfaceRenderStats::default();
    }
    let surface_size = RenderSurfacePixelSize::from_surface(surface);
    gpu_surface_renderer.render(
        &mut gpu_surface::GpuSurfaceRenderTarget {
            device: target.device,
            queue: target.queue,
            encoder: target.encoder,
            target_view: target.surface_view,
            format: surface.config.format,
            size: surface_size.physical_size(),
            dpi_scale: target.dpi_scale,
        },
        &paint_plan.primitives,
        occlusion_plan,
    )
}

fn refresh_composited_base_frame(
    frame: &CompositedBaseFrame,
    state: BaseFrameRefreshState<'_>,
    surface: &RenderSurface<'_>,
    target: &mut BaseFramePresentTarget<'_>,
    paint_plan: &SurfacePaintPlan,
    occlusion_plan: &SurfaceOcclusionPlan,
    has_gpu_surfaces: bool,
) -> gpu_surface::GpuSurfaceRenderStats {
    let (stats, elapsed) = state.profile.measure(|| {
        surface.blitter.copy(
            target.device,
            target.encoder,
            &surface.target_view,
            &frame.view,
        );
        if should_render_gpu_surfaces(has_gpu_surfaces) {
            let surface_size = RenderSurfacePixelSize::from_surface(surface);
            state.gpu_surface_renderer.render(
                &mut gpu_surface::GpuSurfaceRenderTarget {
                    device: target.device,
                    queue: target.queue,
                    encoder: target.encoder,
                    target_view: &frame.view,
                    format: surface.config.format,
                    size: surface_size.physical_size(),
                    dpi_scale: target.dpi_scale,
                },
                &paint_plan.primitives,
                occlusion_plan,
            )
        } else {
            gpu_surface::GpuSurfaceRenderStats::default()
        }
    });
    *state.base_dirty = false;
    state.profile.composited_base_refresh = elapsed;
    stats
}

fn composited_base_needs_refresh(base_dirty: bool, frame_recreated: bool) -> bool {
    base_dirty || frame_recreated
}

fn should_use_composited_base(transient_overlay_primitives: &[PaintPrimitive]) -> bool {
    !transient_overlay_primitives.is_empty()
}

fn should_render_gpu_surfaces(has_gpu_surfaces: bool) -> bool {
    has_gpu_surfaces
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composited_base_refreshes_when_dirty_or_recreated() {
        assert!(composited_base_needs_refresh(true, false));
        assert!(composited_base_needs_refresh(false, true));
        assert!(composited_base_needs_refresh(true, true));
        assert!(!composited_base_needs_refresh(false, false));
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
    fn gpu_surface_composition_is_needed_only_when_scene_contains_gpu_surfaces() {
        assert!(!should_render_gpu_surfaces(false));
        assert!(should_render_gpu_surfaces(true));
    }
}
