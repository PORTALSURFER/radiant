use super::super::*;
use crate::gui::types::ImageRgba;
use std::sync::Arc;

pub(super) fn hover_capabilities(line: GpuSurfaceLineStyle) -> GpuSurfaceCapabilities {
    GpuSurfaceCapabilities {
        fast_pointer_move: true,
        coalesce_vertical_wheel: true,
        runtime_overlays: GpuSurfaceRuntimeOverlays::pointer_vertical_line(line),
    }
}

pub(super) fn white_hover_capabilities() -> GpuSurfaceCapabilities {
    hover_capabilities(GpuSurfaceLineStyle {
        color: Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        width: 1.0,
    })
}

pub(super) fn rgba_content(size: Vector2) -> GpuSurfaceContent {
    let width = size.x as usize;
    let height = size.y as usize;
    GpuSurfaceContent::RgbaAtlas {
        source_rect: Rect::from_min_size(Point::new(0.0, 0.0), size),
        atlas: Arc::new(
            ImageRgba::new(width, height, vec![255; width * height * 4]).expect("valid image"),
        ),
    }
}
