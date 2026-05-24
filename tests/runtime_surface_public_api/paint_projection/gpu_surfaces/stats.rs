use super::*;

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
