//! Cached composed frame used by paint-only transient overlay presentations.

use super::{GpuSurfaceRenderer, RenderFrameProfile, RenderSurfacePixelSize, gpu_surface};
#[cfg(test)]
use crate::gui::types::{Point, Rect as UiRect, Rgba8, Vector2};
use crate::runtime::{PaintPrimitive, SurfacePaintPlan};
use std::time::Instant;
use vello::{util::RenderSurface, wgpu};

mod frame;
pub(super) use frame::CompositedBaseFrame;

pub(super) struct BaseFramePresentTarget<'a> {
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) encoder: &'a mut wgpu::CommandEncoder,
    pub(super) surface_view: &'a wgpu::TextureView,
}

pub(super) struct BaseFramePresentState<'a> {
    pub(super) base_frame: &'a mut Option<CompositedBaseFrame>,
    pub(super) base_dirty: &'a mut bool,
    pub(super) gpu_surface_renderer: &'a mut GpuSurfaceRenderer,
    pub(super) profile: &'a mut RenderFrameProfile,
}

pub(super) fn present_base_frame(
    state: &mut BaseFramePresentState<'_>,
    surface: &RenderSurface<'_>,
    target: &mut BaseFramePresentTarget<'_>,
    paint_plan: &SurfacePaintPlan,
    transient_overlay_primitives: &[PaintPrimitive],
) -> gpu_surface::GpuSurfaceRenderStats {
    if !should_use_composited_base(transient_overlay_primitives) {
        return present_live_base(state.gpu_surface_renderer, surface, target, paint_plan);
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
        refresh_composited_base_frame(
            frame,
            state.base_dirty,
            state.gpu_surface_renderer,
            surface,
            target,
            paint_plan,
            state.profile,
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
) -> gpu_surface::GpuSurfaceRenderStats {
    let surface_size = RenderSurfacePixelSize::from_surface(surface);
    surface.blitter.copy(
        target.device,
        target.encoder,
        &surface.target_view,
        target.surface_view,
    );
    gpu_surface_renderer.render(
        &mut gpu_surface::GpuSurfaceRenderTarget {
            device: target.device,
            queue: target.queue,
            encoder: target.encoder,
            target_view: target.surface_view,
            format: surface.config.format,
            size: surface_size.logical_size(),
        },
        &paint_plan.primitives,
    )
}

fn refresh_composited_base_frame(
    frame: &CompositedBaseFrame,
    base_dirty: &mut bool,
    gpu_surface_renderer: &mut GpuSurfaceRenderer,
    surface: &RenderSurface<'_>,
    target: &mut BaseFramePresentTarget<'_>,
    paint_plan: &SurfacePaintPlan,
    profile: &mut RenderFrameProfile,
) -> gpu_surface::GpuSurfaceRenderStats {
    let surface_size = RenderSurfacePixelSize::from_surface(surface);
    let started = Instant::now();
    surface.blitter.copy(
        target.device,
        target.encoder,
        &surface.target_view,
        &frame.view,
    );
    let stats = gpu_surface_renderer.render(
        &mut gpu_surface::GpuSurfaceRenderTarget {
            device: target.device,
            queue: target.queue,
            encoder: target.encoder,
            target_view: &frame.view,
            format: surface.config.format,
            size: surface_size.logical_size(),
        },
        &paint_plan.primitives,
    );
    *base_dirty = false;
    profile.composited_base_refresh = started.elapsed();
    stats
}

fn composited_base_needs_refresh(base_dirty: bool, frame_recreated: bool) -> bool {
    base_dirty || frame_recreated
}

fn should_use_composited_base(transient_overlay_primitives: &[PaintPrimitive]) -> bool {
    !transient_overlay_primitives.is_empty()
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
}
