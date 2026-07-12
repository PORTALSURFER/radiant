use super::super::*;
use super::fixtures::*;
use std::sync::Arc;

#[test]
fn gpu_surface_lookup_skips_unrenderable_surface_content() {
    let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 20.0));
    let capabilities = white_hover_capabilities();
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

    let surface_index =
        visible_hover_surface_index(&primitives, Point::new(10.0, 10.0)).expect("valid surface");

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
        capabilities: white_hover_capabilities(),
        overlays: Vec::new(),
    })];

    assert!(visible_hover_surface_index(&primitives, Point::new(0.0, 10.0)).is_none());
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
        capabilities: white_hover_capabilities(),
        overlays: Vec::new(),
    })];

    assert!(visible_hover_surface_index(&primitives, Point::new(0.0, 10.0)).is_none());

    let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 20.0));
    let primitives = vec![PaintPrimitive::GpuSurface(PaintGpuSurface {
        widget_id: 1,
        key: 1,
        revision: 1,
        rect,
        content: rgba_content(Vector2::new(1.0, 1.0)),
        capabilities: white_hover_capabilities(),
        overlays: Vec::new(),
    })];

    assert!(visible_hover_surface_index(&primitives, Point::new(f32::NAN, 10.0)).is_none());
}

#[test]
fn gpu_surface_lookup_selects_topmost_visible_hover_surface() {
    let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0));
    let capabilities = white_hover_capabilities();
    let content = rgba_content(Vector2::new(100.0, 40.0));
    let primitives = vec![
        PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: 1,
            key: 1,
            revision: 1,
            rect,
            content: content.clone(),
            capabilities,
            overlays: Vec::new(),
        }),
        PaintPrimitive::ClipStart(crate::runtime::PaintClipStart {
            node_id: 2,
            rect: Rect::from_min_size(Point::new(50.0, 0.0), Vector2::new(50.0, 40.0)),
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
        PaintPrimitive::ClipEnd(crate::runtime::PaintClipEnd { node_id: 2 }),
    ];

    assert_eq!(
        visible_hover_surface_index(&primitives, Point::new(25.0, 20.0)),
        Some(0),
        "the clipped-away later surface must not steal the visible surface's hover overlay"
    );
    assert_eq!(
        visible_hover_surface_index(&primitives, Point::new(75.0, 20.0)),
        Some(2),
        "the later surface remains topmost inside its visible clip"
    );
}
