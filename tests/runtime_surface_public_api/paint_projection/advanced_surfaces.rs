use super::*;

#[test]
fn retained_canvas_metadata_reaches_backend_neutral_paint_plan() {
    let retained = RetainedSurfaceDescriptor {
        key: 42,
        revision: 7,
        dirty_mask: 0b101,
        volatile: false,
    };
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::retained_canvas_mapped(
        90,
        WidgetSizing::fixed(Vector2::new(240.0, 120.0)),
        retained,
        |message| match message {
            CanvasMessage::Input { input } => DemoMessage::CanvasInput(input),
        },
    ));
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 120.0)),
    );

    let plan = surface.paint_plan(&layout, &ThemeTokens::default());

    let custom = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::CustomSurface(custom) => Some(custom),
            _ => None,
        })
        .expect("retained canvas should emit one custom surface primitive");
    assert_eq!(custom.widget_id, 90);
    assert_eq!(custom.retained, Some(retained));
}

#[test]
fn retained_canvas_builder_projects_metadata_and_input_mapping() {
    let surface = radiant::runtime::retained_canvas(44)
        .revision(7)
        .dirty_mask(3)
        .volatile(true)
        .on_input(|message| match message {
            CanvasMessage::Input { input } => DemoMessage::CanvasInput(input),
        })
        .id(44)
        .size(120.0, 40.0)
        .into_surface();
    let plan = surface.paint_plan(
        &layout_tree(
            &surface.layout_node(),
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 40.0)),
        ),
        &ThemeTokens::default(),
    );
    let custom = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::CustomSurface(custom) => Some(custom),
            _ => None,
        })
        .expect("retained canvas should project one custom surface primitive");
    assert_eq!(
        custom.retained,
        Some(RetainedSurfaceDescriptor {
            key: 44,
            revision: 7,
            dirty_mask: 3,
            volatile: true,
        })
    );
}
