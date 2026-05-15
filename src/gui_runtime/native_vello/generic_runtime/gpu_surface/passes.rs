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

pub(super) fn set_surface_scissor(pass: &mut wgpu::RenderPass<'_>, rect: UiRect) -> bool {
    let Some((x, y, width, height)) = surface_scissor_rect(rect) else {
        return false;
    };
    pass.set_scissor_rect(x, y, width, height);
    true
}

pub(super) fn surface_rect_has_finite_positive_size(rect: UiRect) -> bool {
    rect.min.x.is_finite()
        && rect.min.y.is_finite()
        && rect.max.x.is_finite()
        && rect.max.y.is_finite()
        && rect.width() > 0.0
        && rect.height() > 0.0
}

fn surface_scissor_rect(rect: UiRect) -> Option<(u32, u32, u32, u32)> {
    if !surface_rect_has_finite_positive_size(rect) {
        return None;
    }
    let x = rect.min.x.max(0.0).floor() as u32;
    let y = rect.min.y.max(0.0).floor() as u32;
    let width = rect.width().ceil() as u32;
    let height = rect.height().ceil() as u32;
    (width > 0 && height > 0).then_some((x, y, width, height))
}

pub(super) type OverlayVec4Slots = [[f32; 4]; GPU_SURFACE_OVERLAY_VEC4_SLOTS];
pub(super) type OverlayColorSlots = [[f32; 4]; MAX_GPU_SURFACE_OVERLAYS];
pub(super) type VerticalOverlayUniforms = (OverlayVec4Slots, OverlayVec4Slots, OverlayColorSlots);

pub(super) fn vertical_overlays(overlays: &[GpuSurfaceOverlay]) -> VerticalOverlayUniforms {
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
            let Some((start, end)) = normalized_range(start, end) else {
                return hidden_overlay(color);
            };
            (start, color, -end)
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
        } => (
            normalized_ratio(ratio).unwrap_or(-1.0),
            color,
            normalized_line_width(width),
        ),
    }
}

fn normalized_range(start: f32, end: f32) -> Option<(f32, f32)> {
    let start = normalized_ratio(start)?;
    let end = normalized_ratio(end)?;
    Some((start.min(end), start.max(end)))
}

fn normalized_ratio(ratio: f32) -> Option<f32> {
    ratio.is_finite().then_some(ratio.clamp(0.0, 1.0))
}

fn normalized_line_width(width: f32) -> f32 {
    if width.is_finite() && width > 0.0 {
        width
    } else {
        1.0
    }
}

fn hidden_overlay(color: Rgba8) -> (f32, Rgba8, f32) {
    (-1.0, color, 1.0)
}

pub(super) fn rgba_to_float(color: Rgba8) -> [f32; 4] {
    [
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
        color.a as f32 / 255.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: Rgba8 = Rgba8 {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    #[test]
    fn vertical_overlay_uniforms_sanitize_invalid_cursor_inputs() {
        let (ratios, widths, colors) = vertical_overlays(&[
            GpuSurfaceOverlay::VerticalCursor {
                ratio: f32::NAN,
                color: WHITE,
                width: f32::INFINITY,
            },
            GpuSurfaceOverlay::RuntimeVerticalLine {
                ratio: 1.5,
                color: WHITE,
                width: -2.0,
            },
        ]);

        assert_eq!(ratios[0][0], -1.0);
        assert_eq!(widths[0][0], 1.0);
        assert_eq!(ratios[0][1], 1.0);
        assert_eq!(widths[0][1], 1.0);
        assert_eq!(colors[0], rgba_to_float(WHITE));
    }

    #[test]
    fn vertical_overlay_uniforms_order_and_sanitize_ranges() {
        let (ratios, widths, _) = vertical_overlays(&[
            GpuSurfaceOverlay::HorizontalRange {
                start: 0.8,
                end: 0.2,
                color: WHITE,
            },
            GpuSurfaceOverlay::HorizontalRange {
                start: f32::NAN,
                end: 0.5,
                color: WHITE,
            },
        ]);

        assert_eq!(ratios[0][0], 0.2);
        assert_eq!(widths[0][0], -0.8);
        assert_eq!(ratios[0][1], -1.0);
        assert_eq!(widths[0][1], 1.0);
    }

    #[test]
    fn surface_scissor_rect_rejects_invalid_geometry() {
        assert_eq!(
            surface_scissor_rect(UiRect::from_min_size(
                Point::new(f32::NEG_INFINITY, 0.0),
                Vector2::new(10.0, 10.0),
            )),
            None
        );
        assert_eq!(
            surface_scissor_rect(UiRect::from_min_size(
                Point::new(0.0, 0.0),
                Vector2::new(0.0, 10.0),
            )),
            None
        );
    }

    #[test]
    fn surface_scissor_rect_uses_finite_positive_pixel_bounds() {
        assert_eq!(
            surface_scissor_rect(UiRect::from_min_size(
                Point::new(-2.4, 3.2),
                Vector2::new(10.2, 6.1),
            )),
            Some((0, 3, 11, 7))
        );
    }
}
