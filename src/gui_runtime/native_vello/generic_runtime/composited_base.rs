//! Cached composed frame used by paint-only transient overlay presentations.

use super::*;
use crate::runtime::{PaintPrimitive, SurfacePaintPlan};

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
            size: Vector2::new(surface.config.width as f32, surface.config.height as f32),
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
            size: Vector2::new(surface.config.width as f32, surface.config.height as f32),
        },
        &paint_plan.primitives,
    );
    *base_dirty = false;
    profile.composited_base_refresh = started.elapsed();
    stats
}

pub(super) struct CompositedBaseFrame {
    _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
}

impl CompositedBaseFrame {
    pub(super) fn ensure<'a>(
        frame: &'a mut Option<Self>,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> (&'a mut Self, bool) {
        if frame
            .as_ref()
            .is_some_and(|frame| frame.matches(width, height, format))
            && let Some(existing) = frame
        {
            return (existing, false);
        }
        (frame.insert(Self::new(device, width, height, format)), true)
    }

    fn new(device: &wgpu::Device, width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("radiant_composited_base_frame"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            _texture: texture,
            view,
            width: width.max(1),
            height: height.max(1),
            format,
        }
    }

    fn matches(&self, width: u32, height: u32, format: wgpu::TextureFormat) -> bool {
        composited_base_frame_matches_descriptor(
            self.width,
            self.height,
            self.format,
            width,
            height,
            format,
        )
    }
}

fn composited_base_frame_matches_descriptor(
    stored_width: u32,
    stored_height: u32,
    stored_format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> bool {
    stored_width == width.max(1) && stored_height == height.max(1) && stored_format == format
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
    fn composited_base_frame_matches_surface_descriptor() {
        assert!(composited_base_frame_matches_descriptor(
            640,
            360,
            wgpu::TextureFormat::Bgra8Unorm,
            640,
            360,
            wgpu::TextureFormat::Bgra8Unorm
        ));
        assert!(!composited_base_frame_matches_descriptor(
            640,
            360,
            wgpu::TextureFormat::Bgra8Unorm,
            641,
            360,
            wgpu::TextureFormat::Bgra8Unorm
        ));
        assert!(!composited_base_frame_matches_descriptor(
            640,
            360,
            wgpu::TextureFormat::Bgra8Unorm,
            640,
            360,
            wgpu::TextureFormat::Rgba8Unorm
        ));
    }

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
