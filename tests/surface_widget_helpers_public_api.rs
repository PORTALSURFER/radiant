//! Public API coverage for surface widget helper construction and mapping.

use radiant::{
    layout::{Point, Rect, Vector2},
    runtime::{SurfaceChild, SurfaceNode, UiSurface},
    widgets::{
        BadgeMessage, ButtonMessage, CanvasMessage, CanvasWidget, ListItemMessage, ListItemWidget,
        PointerButton, ScrollbarAxis, ScrollbarMessage, ScrollbarWidget, SelectableMessage,
        SelectableWidget, TextInputMessage, TextWidget, ToggleMessage, Widget, WidgetInput,
        WidgetSizing,
    },
};

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Increment,
    Rename(String),
    SetActive(bool),
    CanvasInput(WidgetInput),
}

fn widget_ref<'a, T, Message>(surface: &'a UiSurface<Message>, id: u64, expected: &str) -> &'a T
where
    T: Widget + 'static,
{
    surface
        .find_widget(id)
        .unwrap_or_else(|| panic!("expected {expected} widget {id} to exist"))
        .widget()
        .as_any()
        .downcast_ref::<T>()
        .unwrap_or_else(|| panic!("expected widget {id} to be {expected}"))
}

#[test]
fn static_widget_helper_builds_non_emitting_leaf() {
    let title = TextWidget::new(
        30,
        "Status",
        WidgetSizing::fixed(Vector2::new(120.0, 20.0)).with_baseline(14.0),
    );
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::static_widget(title));

    assert!(surface.find_widget(30).is_some());
    assert_eq!(
        surface.dispatch_widget_output(
            30,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate)
        ),
        None
    );
}

#[test]
fn text_and_button_helpers_build_common_leaf_nodes() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::row(
        4,
        8.0,
        vec![
            SurfaceChild::fill(SurfaceNode::text(
                40,
                "Counter",
                WidgetSizing::fixed(Vector2::new(120.0, 20.0)).with_baseline(14.0),
            )),
            SurfaceChild::fill(SurfaceNode::button(
                41,
                "Increment",
                WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
                DemoMessage::Increment,
            )),
            SurfaceChild::fill(SurfaceNode::button_mapped(
                42,
                "Rename",
                WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
                |_| DemoMessage::Rename(String::from("Mapped")),
            )),
            SurfaceChild::fill(SurfaceNode::badge(
                43,
                "Active",
                WidgetSizing::fixed(Vector2::new(72.0, 24.0)),
                DemoMessage::SetActive(true),
            )),
            SurfaceChild::fill(SurfaceNode::badge_mapped(
                44,
                "Mapped badge",
                WidgetSizing::fixed(Vector2::new(112.0, 24.0)),
                |_| DemoMessage::Rename(String::from("Badge")),
            )),
        ],
    ));

    assert!(surface.find_widget(40).is_some());
    assert_eq!(
        surface.dispatch_widget_output(
            41,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate)
        ),
        Some(DemoMessage::Increment)
    );
    assert_eq!(
        surface.dispatch_widget_output(
            42,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate)
        ),
        Some(DemoMessage::Rename(String::from("Mapped")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            43,
            radiant::widgets::WidgetOutput::typed(BadgeMessage::Activate)
        ),
        Some(DemoMessage::SetActive(true))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            44,
            radiant::widgets::WidgetOutput::typed(BadgeMessage::Activate)
        ),
        Some(DemoMessage::Rename(String::from("Badge")))
    );
}

#[test]
fn text_input_and_toggle_helpers_map_value_messages() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::row(
        5,
        8.0,
        vec![
            SurfaceChild::fill(SurfaceNode::text_input(
                50,
                "Draft",
                WidgetSizing::fixed(Vector2::new(140.0, 28.0)),
                DemoMessage::Rename,
            )),
            SurfaceChild::fill(SurfaceNode::text_input_mapped(
                51,
                "Raw",
                WidgetSizing::fixed(Vector2::new(140.0, 28.0)),
                |message| match message {
                    TextInputMessage::Changed { value } | TextInputMessage::Submitted { value } => {
                        DemoMessage::Rename(format!("raw:{value}"))
                    }
                },
            )),
            SurfaceChild::fill(SurfaceNode::toggle(
                52,
                "Enabled",
                WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
                DemoMessage::SetActive,
            )),
            SurfaceChild::fill(SurfaceNode::toggle_mapped(
                53,
                "Raw toggle",
                WidgetSizing::fixed(Vector2::new(112.0, 28.0)),
                |message| match message {
                    ToggleMessage::ValueChanged { checked } => DemoMessage::SetActive(!checked),
                },
            )),
            SurfaceChild::fill(SurfaceNode::toggle_with_checked(
                54,
                "Done",
                true,
                WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
                DemoMessage::SetActive,
            )),
            SurfaceChild::fill(SurfaceNode::toggle_mapped_with_checked(
                55,
                "Raw done",
                true,
                WidgetSizing::fixed(Vector2::new(112.0, 28.0)),
                |message| match message {
                    ToggleMessage::ValueChanged { checked } => DemoMessage::SetActive(!checked),
                },
            )),
        ],
    ));

    assert_eq!(
        surface.dispatch_widget_output(
            50,
            radiant::widgets::WidgetOutput::typed(TextInputMessage::Changed {
                value: String::from("Edited"),
            })
        ),
        Some(DemoMessage::Rename(String::from("Edited")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            51,
            radiant::widgets::WidgetOutput::typed(TextInputMessage::Submitted {
                value: String::from("Submitted"),
            })
        ),
        Some(DemoMessage::Rename(String::from("raw:Submitted")))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            52,
            radiant::widgets::WidgetOutput::typed(ToggleMessage::ValueChanged { checked: true })
        ),
        Some(DemoMessage::SetActive(true))
    );
    assert_eq!(
        surface.dispatch_widget_output(
            53,
            radiant::widgets::WidgetOutput::typed(ToggleMessage::ValueChanged { checked: true })
        ),
        Some(DemoMessage::SetActive(false))
    );
    assert_eq!(
        surface
            .find_widget(54)
            .map(|widget| widget.widget().common().state.active),
        Some(true)
    );
    assert_eq!(
        surface
            .find_widget(55)
            .map(|widget| widget.widget().common().state.active),
        Some(true)
    );
}

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
