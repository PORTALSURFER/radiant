use super::*;

#[test]
fn scrollbar_list_item_and_canvas_helpers_build_common_leaf_nodes() {
    let mut surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::column(
        6,
        4.0,
        vec![
            SurfaceChild::fill(SurfaceNode::scrollbar(
                60,
                ScrollbarAxis::Vertical,
                WidgetSizing::fixed(Vector2::new(12.0, 120.0)),
                |offset| DemoMessage::Rename(format!("offset:{offset:.2}")),
            )),
            SurfaceChild::fill(SurfaceNode::scrollbar_mapped(
                61,
                ScrollbarAxis::Horizontal,
                WidgetSizing::fixed(Vector2::new(120.0, 12.0)),
                |message| match message {
                    ScrollbarMessage::OffsetChanged { offset_fraction } => {
                        DemoMessage::Rename(format!("raw:{offset_fraction:.1}"))
                    }
                },
            )),
            SurfaceChild::fill(SurfaceNode::list_item(
                62,
                "Row",
                WidgetSizing::fixed(Vector2::new(120.0, 24.0)),
            )),
            SurfaceChild::fill(SurfaceNode::list_item_mapped(
                64,
                "Mapped row",
                WidgetSizing::fixed(Vector2::new(120.0, 24.0)),
                |_| DemoMessage::Rename(String::from("row")),
            )),
            SurfaceChild::fill(SurfaceNode::selectable(
                65,
                "Choice",
                false,
                WidgetSizing::fixed(Vector2::new(120.0, 24.0)),
                DemoMessage::SetActive,
            )),
            SurfaceChild::fill(SurfaceNode::canvas(
                63,
                WidgetSizing::fixed(Vector2::new(120.0, 80.0)),
            )),
            SurfaceChild::fill(SurfaceNode::canvas_mapped(
                66,
                WidgetSizing::fixed(Vector2::new(160.0, 90.0)),
                |message| match message {
                    CanvasMessage::Input { input } => DemoMessage::CanvasInput(input),
                },
            )),
        ],
    ));

    assert!(
        widget_ref::<ScrollbarWidget, _>(&surface, 60, "scrollbar")
            .common()
            .id
            == 60
    );
    assert!(
        widget_ref::<ListItemWidget, _>(&surface, 62, "list item")
            .common()
            .id
            == 62
    );
    assert!(
        widget_ref::<ListItemWidget, _>(&surface, 64, "list item")
            .common()
            .id
            == 64
    );
    assert!(
        widget_ref::<SelectableWidget, _>(&surface, 65, "selectable")
            .common()
            .id
            == 65
    );
    assert!(
        widget_ref::<CanvasWidget, _>(&surface, 63, "canvas")
            .common()
            .id
            == 63
    );
    assert!(
        widget_ref::<CanvasWidget, _>(&surface, 66, "canvas")
            .common()
            .id
            == 66
    );
    assert_eq!(
        surface.dispatch_widget_output(
            60,
            radiant::widgets::WidgetOutput::typed(ScrollbarMessage::OffsetChanged {
                offset_fraction: 0.25,
            })
        ),
        Some(DemoMessage::Rename(String::from("offset:0.25")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            61,
            radiant::widgets::WidgetOutput::typed(ScrollbarMessage::OffsetChanged {
                offset_fraction: 0.5,
            })
        ),
        Some(DemoMessage::Rename(String::from("raw:0.5")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            64,
            radiant::widgets::WidgetOutput::typed(ListItemMessage::Invoked)
        ),
        Some(DemoMessage::Rename(String::from("row")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            65,
            radiant::widgets::WidgetOutput::typed(SelectableMessage::SelectionChanged {
                selected: true,
            })
        ),
        Some(DemoMessage::SetActive(true))
    );

    let canvas_input = WidgetInput::PointerPress {
        position: Point::new(12.0, 8.0),
        button: PointerButton::Primary,
        modifiers: Default::default(),
    };
    let canvas_output = surface
        .dispatch_widget_input(
            66,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 90.0)),
            canvas_input.clone(),
        )
        .expect("canvas should forward routed input");
    assert_eq!(
        canvas_output.typed_ref::<CanvasMessage>(),
        Some(&CanvasMessage::Input {
            input: canvas_input.clone()
        })
    );
    assert_eq!(
        surface.dispatch_widget_output(66, canvas_output),
        Some(DemoMessage::CanvasInput(canvas_input))
    );
}
