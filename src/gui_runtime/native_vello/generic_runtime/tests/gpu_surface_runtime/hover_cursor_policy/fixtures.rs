use super::super::*;
use crate::gui::types::ImageRgba;
use std::sync::Arc;

pub(super) fn visible_hover_surface_index(
    primitives: &[PaintPrimitive],
    position: Point,
) -> Option<usize> {
    use super::super::super::super::{
        gpu_surface_cursor::topmost_native_hover_surface_index,
        runtime_helpers::{
            GpuSurfaceInteractionScratch, collect_gpu_surface_interaction_regions_with_scratch,
        },
    };

    let mut regions = Vec::new();
    collect_gpu_surface_interaction_regions_with_scratch(
        primitives,
        &mut regions,
        &mut GpuSurfaceInteractionScratch::default(),
    );
    topmost_native_hover_surface_index(&regions, position)
}

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
