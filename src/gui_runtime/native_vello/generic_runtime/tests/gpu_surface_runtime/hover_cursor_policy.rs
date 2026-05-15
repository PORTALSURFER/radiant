use super::super::super::gpu_surface_cursor::{
    clear_surface_cursor_overlay, topmost_native_hover_surface_index, update_surface_cursor_overlay,
};
use super::*;
use crate::gui::types::ImageRgba;
use std::sync::Arc;

fn hover_capabilities(line: GpuSurfaceLineStyle) -> GpuSurfaceCapabilities {
    GpuSurfaceCapabilities {
        fast_pointer_move: true,
        coalesce_vertical_wheel: true,
        runtime_overlays: GpuSurfaceRuntimeOverlays::pointer_vertical_line(line),
    }
}

fn rgba_content(size: Vector2) -> GpuSurfaceContent {
    let width = size.x as usize;
    let height = size.y as usize;
    GpuSurfaceContent::RgbaAtlas {
        source_rect: Rect::from_min_size(Point::new(0.0, 0.0), size),
        atlas: Arc::new(
            ImageRgba::new(width, height, vec![255; width * height * 4]).expect("valid image"),
        ),
    }
}

#[test]
fn gpu_surface_lookup_skips_unrenderable_surface_content() {
    let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 20.0));
    let capabilities = hover_capabilities(GpuSurfaceLineStyle {
        color: Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        width: 1.0,
    });
    let primitives = vec![
        PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: 1,
            key: 1,
            revision: 1,
            rect,
            content: GpuSurfaceContent::SignalBands {
                frames: 1,
                band_count: 0,
                frame_range: [0.0, 1.0],
                samples: Arc::<[f32]>::from([0.0]),
            },
            capabilities,
            overlays: Vec::new(),
        }),
        PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: 2,
            key: 2,
            revision: 1,
            rect,
            content: rgba_content(Vector2::new(20.0, 20.0)),
            capabilities,
            overlays: Vec::new(),
        }),
    ];

    let surface_index = topmost_native_hover_surface_index(&primitives, Point::new(10.0, 10.0))
        .expect("valid surface");

    assert_eq!(surface_index, 1);
}

#[test]
fn gpu_surface_lookup_skips_empty_surface_rects() {
    let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(0.0, 20.0));
    let primitives = vec![PaintPrimitive::GpuSurface(PaintGpuSurface {
        widget_id: 1,
        key: 1,
        revision: 1,
        rect,
        content: rgba_content(Vector2::new(1.0, 1.0)),
        capabilities: hover_capabilities(GpuSurfaceLineStyle {
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            width: 1.0,
        }),
        overlays: Vec::new(),
    })];

    assert!(topmost_native_hover_surface_index(&primitives, Point::new(0.0, 10.0)).is_none());
}

#[test]
fn gpu_surface_lookup_skips_nonfinite_surface_rects_and_positions() {
    let rect = Rect::from_min_max(Point::new(f32::NEG_INFINITY, 0.0), Point::new(40.0, 20.0));
    let primitives = vec![PaintPrimitive::GpuSurface(PaintGpuSurface {
        widget_id: 1,
        key: 1,
        revision: 1,
        rect,
        content: rgba_content(Vector2::new(1.0, 1.0)),
        capabilities: hover_capabilities(GpuSurfaceLineStyle {
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            width: 1.0,
        }),
        overlays: Vec::new(),
    })];

    assert!(topmost_native_hover_surface_index(&primitives, Point::new(0.0, 10.0)).is_none());

    let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 20.0));
    let primitives = vec![PaintPrimitive::GpuSurface(PaintGpuSurface {
        widget_id: 1,
        key: 1,
        revision: 1,
        rect,
        content: rgba_content(Vector2::new(1.0, 1.0)),
        capabilities: hover_capabilities(GpuSurfaceLineStyle {
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            width: 1.0,
        }),
        overlays: Vec::new(),
    })];

    assert!(topmost_native_hover_surface_index(&primitives, Point::new(f32::NAN, 10.0)).is_none());
}

#[test]
fn native_hover_cursor_clears_stale_overlay_for_invalid_geometry() {
    let mut surface = PaintGpuSurface {
        widget_id: 1,
        key: 1,
        revision: 1,
        rect: Rect::from_min_max(Point::new(f32::NEG_INFINITY, 0.0), Point::new(40.0, 20.0)),
        content: rgba_content(Vector2::new(1.0, 1.0)),
        capabilities: hover_capabilities(GpuSurfaceLineStyle {
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            width: 1.0,
        }),
        overlays: vec![GpuSurfaceOverlay::RuntimeVerticalLine {
            ratio: 0.5,
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            width: 1.0,
        }],
    };

    assert!(update_surface_cursor_overlay(
        &mut surface,
        Point::new(0.0, 10.0)
    ));
    assert!(surface.overlays.is_empty());
}

#[test]
fn native_hover_cursor_updates_topmost_surface_and_clears_stale_cursors() {
    let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 20.0));
    let capabilities = hover_capabilities(GpuSurfaceLineStyle {
        color: Rgba8 {
            r: 255,
            g: 160,
            b: 0,
            a: 255,
        },
        width: 2.0,
    });
    let content = rgba_content(Vector2::new(1.0, 1.0));
    let mut primitives = vec![
        PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: 1,
            key: 1,
            revision: 1,
            rect,
            content: content.clone(),
            capabilities,
            overlays: vec![GpuSurfaceOverlay::RuntimeVerticalLine {
                ratio: 0.1,
                color: capabilities
                    .runtime_overlays
                    .pointer_vertical_line
                    .unwrap()
                    .color,
                width: 2.0,
            }],
        }),
        PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: 2,
            key: 2,
            revision: 1,
            rect,
            content,
            capabilities,
            overlays: Vec::new(),
        }),
    ];

    let target = topmost_native_hover_surface_index(&primitives, Point::new(30.0, 10.0));

    assert_eq!(target, Some(1));
    for (index, primitive) in primitives.iter_mut().enumerate() {
        let PaintPrimitive::GpuSurface(surface) = primitive else {
            continue;
        };
        if Some(index) == target {
            assert!(update_surface_cursor_overlay(
                surface,
                Point::new(30.0, 10.0)
            ));
        } else {
            assert!(clear_surface_cursor_overlay(surface));
        }
    }
    let [
        PaintPrimitive::GpuSurface(bottom),
        PaintPrimitive::GpuSurface(top),
    ] = primitives.as_slice()
    else {
        panic!("expected GPU surfaces");
    };
    assert!(bottom.overlays.is_empty());
    assert!(matches!(
        top.overlays.as_slice(),
        [GpuSurfaceOverlay::RuntimeVerticalLine { ratio, .. }] if *ratio == 0.75
    ));
}
