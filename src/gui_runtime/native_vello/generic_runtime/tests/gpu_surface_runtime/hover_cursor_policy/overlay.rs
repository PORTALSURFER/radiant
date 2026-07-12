use super::super::super::super::gpu_surface_cursor::{
    clear_surface_cursor_overlay, update_surface_cursor_overlay,
};
use super::super::*;
use super::fixtures::*;

#[test]
fn native_hover_cursor_clears_stale_overlay_for_invalid_geometry() {
    let mut surface = PaintGpuSurface {
        widget_id: 1,
        key: 1,
        revision: 1,
        rect: Rect::from_min_max(Point::new(f32::NEG_INFINITY, 0.0), Point::new(40.0, 20.0)),
        content: rgba_content(Vector2::new(1.0, 1.0)),
        capabilities: white_hover_capabilities(),
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

    let target = visible_hover_surface_index(&primitives, Point::new(30.0, 10.0));

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

#[test]
fn native_hover_cursor_updates_single_runtime_overlay_in_place() {
    let mut surface = PaintGpuSurface {
        widget_id: 1,
        key: 1,
        revision: 1,
        rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 20.0)),
        content: rgba_content(Vector2::new(1.0, 1.0)),
        capabilities: white_hover_capabilities(),
        overlays: vec![
            GpuSurfaceOverlay::HorizontalRange {
                start: 0.25,
                end: 0.75,
                color: Rgba8 {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 80,
                },
            },
            GpuSurfaceOverlay::RuntimeVerticalLine {
                ratio: 0.1,
                color: Rgba8 {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
                width: 1.0,
            },
        ],
    };

    assert!(update_surface_cursor_overlay(
        &mut surface,
        Point::new(30.0, 10.0)
    ));

    assert!(matches!(
        surface.overlays.as_slice(),
        [
            GpuSurfaceOverlay::HorizontalRange { start, end, .. },
            GpuSurfaceOverlay::RuntimeVerticalLine { ratio, .. },
        ] if *start == 0.25 && *end == 0.75 && *ratio == 0.75
    ));
}
