use super::*;

#[test]
fn gpu_surface_compatibility_constructor_delegates_to_named_parts() {
    let content = GpuSurfaceContent::SignalBands {
        frames: 2,
        band_count: 1,
        frame_range: [0.0, 2.0],
        samples: [0.0, 1.0].into(),
    };
    let from_parts = GpuSurfaceWidget::from_parts(GpuSurfaceParts {
        id: 41,
        sizing: WidgetSizing::fixed(Vector2::new(80.0, 20.0)),
        key: 9001,
        revision: 7,
        content: content.clone(),
    });
    let positional = GpuSurfaceWidget::new(
        41,
        WidgetSizing::fixed(Vector2::new(80.0, 20.0)),
        9001,
        7,
        content,
    );

    assert_eq!(from_parts, positional);
}

#[test]
fn surface_node_gpu_surface_helper_preserves_named_resource_identity() {
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::gpu_surface(
        41,
        WidgetSizing::fixed(Vector2::new(80.0, 20.0)),
        9001,
        7,
        GpuSurfaceContent::SignalBands {
            frames: 2,
            band_count: 1,
            frame_range: [0.0, 2.0],
            samples: [0.0, 1.0].into(),
        },
    ));
    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0)),
    );

    let plan = surface.paint_plan(&output, &ThemeTokens::default());

    let Some(PaintPrimitive::GpuSurface(gpu)) = plan.primitives.first() else {
        panic!("expected gpu surface primitive");
    };
    assert_eq!(gpu.widget_id, 41);
    assert_eq!(gpu.key, 9001);
    assert_eq!(gpu.revision, 7);
}
