use crate::{gui::types::Rect as UiRect, runtime::PaintGpuSurface};
use vello::wgpu;

pub(super) fn surface_dest(
    surface: &PaintGpuSurface,
    dpi_scale: crate::theme::DpiScale,
) -> [f32; 4] {
    [
        dpi_scale.logical_to_physical(surface.rect.min.x),
        dpi_scale.logical_to_physical(surface.rect.min.y),
        dpi_scale.logical_to_physical(surface.rect.width()),
        dpi_scale.logical_to_physical(surface.rect.height()),
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

pub(super) fn set_surface_scissor(
    pass: &mut wgpu::RenderPass<'_>,
    rect: UiRect,
    dpi_scale: crate::theme::DpiScale,
) -> bool {
    let Some((x, y, extent)) = surface_scissor_rect(rect, dpi_scale) else {
        return false;
    };
    pass.set_scissor_rect(x, y, extent.width, extent.height);
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct SurfacePixelExtent {
    pub(super) width: u32,
    pub(super) height: u32,
}

pub(super) fn surface_pixel_extent(
    rect: UiRect,
    dpi_scale: crate::theme::DpiScale,
) -> Option<SurfacePixelExtent> {
    if !rect.has_finite_positive_area() {
        return None;
    }
    let width = dpi_scale.logical_to_physical(rect.width()).ceil() as u32;
    let height = dpi_scale.logical_to_physical(rect.height()).ceil() as u32;
    (width > 0 && height > 0).then_some(SurfacePixelExtent { width, height })
}

fn surface_scissor_rect(
    rect: UiRect,
    dpi_scale: crate::theme::DpiScale,
) -> Option<(u32, u32, SurfacePixelExtent)> {
    let x = dpi_scale.logical_to_physical(rect.min.x.max(0.0)).floor() as u32;
    let y = dpi_scale.logical_to_physical(rect.min.y.max(0.0)).floor() as u32;
    Some((x, y, surface_pixel_extent(rect, dpi_scale)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::{Point, Vector2};

    #[test]
    fn surface_scissor_rect_rejects_invalid_geometry() {
        assert_eq!(
            surface_scissor_rect(
                UiRect::from_min_size(Point::new(f32::NEG_INFINITY, 0.0), Vector2::new(10.0, 10.0),),
                crate::theme::DpiScale::ONE
            ),
            None
        );
        assert_eq!(
            surface_scissor_rect(
                UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(0.0, 10.0),),
                crate::theme::DpiScale::ONE
            ),
            None
        );
    }

    #[test]
    fn surface_scissor_rect_uses_finite_positive_pixel_bounds() {
        assert_eq!(
            surface_scissor_rect(
                UiRect::from_min_size(Point::new(-2.4, 3.2), Vector2::new(10.2, 6.1),),
                crate::theme::DpiScale::ONE
            ),
            Some((
                0,
                3,
                SurfacePixelExtent {
                    width: 11,
                    height: 7,
                },
            ))
        );
    }

    #[test]
    fn surface_pixel_extent_rejects_invalid_geometry() {
        assert_eq!(
            surface_pixel_extent(
                UiRect::from_min_size(Point::new(f32::NAN, 0.0), Vector2::new(10.0, 10.0),),
                crate::theme::DpiScale::ONE
            ),
            None
        );
        assert_eq!(
            surface_pixel_extent(
                UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(10.0, -1.0),),
                crate::theme::DpiScale::ONE
            ),
            None
        );
    }

    #[test]
    fn surface_pixel_extent_rounds_positive_layout_size_up() {
        assert_eq!(
            surface_pixel_extent(
                UiRect::from_min_size(Point::new(2.0, 3.0), Vector2::new(10.2, 6.1),),
                crate::theme::DpiScale::ONE
            ),
            Some(SurfacePixelExtent {
                width: 11,
                height: 7,
            })
        );
    }
}
