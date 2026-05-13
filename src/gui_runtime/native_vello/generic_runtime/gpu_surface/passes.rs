use super::*;

pub(super) fn surface_dest(surface: &PaintGpuSurface) -> [f32; 4] {
    [
        surface.rect.min.x,
        surface.rect.min.y,
        surface.rect.width(),
        surface.rect.height(),
    ]
}

pub(super) fn gpu_surface_render_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    target_view: &'a wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("radiant_gpu_surface_pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    })
}

pub(super) fn signal_body_render_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    target_view: &'a wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("radiant_gpu_signal_body_pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    })
}

pub(super) fn set_surface_scissor(pass: &mut wgpu::RenderPass<'_>, rect: UiRect) {
    let x = rect.min.x.max(0.0).floor() as u32;
    let y = rect.min.y.max(0.0).floor() as u32;
    let width = rect.width().max(1.0).ceil() as u32;
    let height = rect.height().max(1.0).ceil() as u32;
    pass.set_scissor_rect(x, y, width, height);
}

pub(super) fn vertical_overlays(
    overlays: &[GpuSurfaceOverlay],
) -> (
    [[f32; 4]; GPU_SURFACE_OVERLAY_VEC4_SLOTS],
    [[f32; 4]; GPU_SURFACE_OVERLAY_VEC4_SLOTS],
    [[f32; 4]; MAX_GPU_SURFACE_OVERLAYS],
) {
    let mut ratios = [[-1.0; 4]; GPU_SURFACE_OVERLAY_VEC4_SLOTS];
    let mut widths = [[1.0; 4]; GPU_SURFACE_OVERLAY_VEC4_SLOTS];
    let mut colors = [[1.0, 1.0, 1.0, 0.0]; MAX_GPU_SURFACE_OVERLAYS];
    for (index, overlay) in overlays
        .iter()
        .filter(|overlay| {
            matches!(
                overlay,
                GpuSurfaceOverlay::HorizontalRange { .. }
                    | GpuSurfaceOverlay::VerticalCursor { .. }
            )
        })
        .chain(
            overlays
                .iter()
                .filter(|overlay| matches!(overlay, GpuSurfaceOverlay::RuntimeVerticalLine { .. })),
        )
        .take(MAX_GPU_SURFACE_OVERLAYS)
        .enumerate()
    {
        let (ratio, color, width) = vertical_overlay_parts(*overlay);
        ratios[index / 4][index % 4] = ratio;
        widths[index / 4][index % 4] = width;
        colors[index] = rgba_to_float(color);
    }
    (ratios, widths, colors)
}

fn vertical_overlay_parts(overlay: GpuSurfaceOverlay) -> (f32, Rgba8, f32) {
    match overlay {
        GpuSurfaceOverlay::HorizontalRange { start, end, color } => {
            (start.clamp(0.0, 1.0), color, -end.clamp(0.0, 1.0))
        }
        GpuSurfaceOverlay::VerticalCursor {
            ratio,
            color,
            width,
        }
        | GpuSurfaceOverlay::RuntimeVerticalLine {
            ratio,
            color,
            width,
        } => (ratio, color, width),
    }
}

pub(super) fn rgba_to_float(color: Rgba8) -> [f32; 4] {
    [
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
        color.a as f32 / 255.0,
    ]
}
