use super::*;

#[test]
fn gpu_surface_widget_projects_generic_retained_gpu_primitive() {
    let atlas = Arc::new(ImageRgba::new(2, 1, vec![0, 0, 0, 255, 255, 255, 255, 255]).unwrap());
    let content = GpuSurfaceContent::RgbaAtlas {
        source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(2.0, 1.0)),
        atlas: Arc::clone(&atlas),
    };
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::static_widget(
        GpuSurfaceWidget::from_parts(GpuSurfaceParts {
            id: 41,
            sizing: WidgetSizing::fixed(Vector2::new(80.0, 20.0)),
            key: 9001,
            revision: 7,
            content,
        })
        .with_capabilities(GpuSurfaceCapabilities {
            fast_pointer_move: true,
            coalesce_vertical_wheel: true,
            runtime_overlays: GpuSurfaceRuntimeOverlays::pointer_vertical_line(
                GpuSurfaceLineStyle {
                    color: Rgba8 {
                        r: 255,
                        g: 255,
                        b: 255,
                        a: 255,
                    },
                    width: 1.0,
                },
            ),
        })
        .with_overlays(vec![GpuSurfaceOverlay::VerticalCursor {
            ratio: 0.5,
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            width: 1.0,
        }]),
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
    assert!(gpu.capabilities.fast_pointer_move);
    assert!(gpu.capabilities.coalesce_vertical_wheel);
    assert!(
        gpu.capabilities
            .runtime_overlays
            .pointer_vertical_line
            .is_some()
    );
    assert_eq!(gpu.overlays.len(), 1);
    let GpuSurfaceContent::RgbaAtlas { atlas, .. } = &gpu.content else {
        panic!("expected rgba atlas gpu content");
    };
    assert_eq!(atlas.width, 2);
}

#[test]
fn invalid_gpu_surface_payloads_do_not_enter_paint_plan() {
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::static_widget(
        GpuSurfaceWidget::from_parts(GpuSurfaceParts {
            id: 41,
            sizing: WidgetSizing::fixed(Vector2::new(80.0, 20.0)),
            key: 9001,
            revision: 7,
            content: GpuSurfaceContent::SignalBands {
                frames: 2,
                band_count: 0,
                frame_range: [0.0, 2.0],
                samples: [0.0, 1.0].into(),
            },
        }),
    ));
    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0)),
    );

    let plan = surface.paint_plan(&output, &ThemeTokens::default());

    assert!(
        plan.primitives
            .iter()
            .all(|primitive| !matches!(primitive, PaintPrimitive::GpuSurface(_)))
    );
}

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

#[test]
fn paint_plan_stats_count_backend_neutral_frame_shape() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::stack(
        1,
        vec![
            SurfaceChild::fill(SurfaceNode::retained_canvas_mapped(
                40,
                WidgetSizing::fixed(Vector2::new(100.0, 40.0)),
                RetainedSurfaceDescriptor {
                    key: 40,
                    revision: 1,
                    dirty_mask: 0,
                    volatile: false,
                },
                |message| match message {
                    CanvasMessage::Input { input } => DemoMessage::CanvasInput(input),
                },
            )),
            SurfaceChild::fill(SurfaceNode::static_widget(GpuSurfaceWidget::new(
                41,
                WidgetSizing::fixed(Vector2::new(80.0, 20.0)),
                9001,
                7,
                GpuSurfaceContent::RgbaAtlas {
                    source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
                    atlas: Arc::new(ImageRgba::new(1, 1, vec![255, 255, 255, 255]).unwrap()),
                },
            ))),
            SurfaceChild::fill(SurfaceNode::text(
                42,
                "Stats",
                WidgetSizing::fixed(Vector2::new(80.0, 20.0)),
            )),
        ],
    ));
    let frame = surface.frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 80.0)),
        &ThemeTokens::default(),
    );

    let stats = frame.paint_plan.stats();

    assert_eq!(stats.total, frame.paint_plan.primitives.len());
    assert_eq!(stats.custom_surfaces, 1);
    assert_eq!(stats.gpu_surfaces, 1);
    assert_eq!(stats.text, 1);
}
