use super::*;

#[test]
fn public_widgets_dispatch_messages_for_reusable_controls() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(96.0, 28.0));
    let mut button = ButtonWidget::new(10, "Import", WidgetSizing::fixed(Vector2::new(96.0, 28.0)));
    let mut toggle =
        ToggleWidget::new(11, "Enabled", WidgetSizing::fixed(Vector2::new(84.0, 28.0)));
    let mut input = TextInputWidget::new(
        12,
        "ab",
        WidgetSizing::new(Vector2::new(96.0, 28.0), Vector2::new(160.0, 28.0)),
    );
    let mut badge = BadgeWidget::new(13, "Ready", WidgetSizing::fixed(Vector2::new(64.0, 24.0)));
    let mut drag = DragHandleWidget::new(17, WidgetSizing::fixed(Vector2::new(24.0, 24.0)));
    let mut item = ListItemWidget::new(
        14,
        "Document",
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    );
    let mut selectable = SelectableWidget::new(
        16,
        "Selected",
        false,
        WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
    );

    assert_eq!(
        Widget::handle_input(&mut button, bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_typed_widget_output(
        Widget::handle_input(&mut button, bounds, WidgetInput::KeyPress(WidgetKey::Enter)),
        radiant::widgets::ButtonMessage::Activate,
    );

    assert_eq!(
        Widget::handle_input(&mut toggle, bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_typed_widget_output(
        Widget::handle_input(&mut toggle, bounds, WidgetInput::KeyPress(WidgetKey::Space)),
        radiant::widgets::ToggleMessage::ValueChanged { checked: true },
    );

    assert_eq!(
        Widget::handle_input(&mut input, bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_typed_widget_output(
        Widget::handle_input(&mut input, bounds, WidgetInput::Character('z')),
        radiant::widgets::TextInputMessage::Changed {
            value: String::from("abz"),
        },
    );

    assert_eq!(
        Widget::handle_input(&mut badge, bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_typed_widget_output(
        Widget::handle_input(&mut badge, bounds, WidgetInput::KeyPress(WidgetKey::Enter)),
        radiant::widgets::BadgeMessage::Activate,
    );

    assert_typed_widget_output(
        Widget::handle_input(
            &mut drag,
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(10.0, 10.0),
                button: radiant::widgets::PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        radiant::widgets::DragHandleMessage::Started {
            position: Point::new(10.0, 10.0),
        },
    );
    assert_typed_widget_output(
        Widget::handle_input(
            &mut drag,
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(10.0, 38.0),
            },
        ),
        radiant::widgets::DragHandleMessage::Moved {
            position: Point::new(10.0, 38.0),
        },
    );

    assert_eq!(
        Widget::handle_input(&mut item, bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_typed_widget_output(
        Widget::handle_input(&mut item, bounds, WidgetInput::KeyPress(WidgetKey::Enter)),
        radiant::widgets::ListItemMessage::Invoked,
    );

    assert_eq!(
        Widget::handle_input(&mut selectable, bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_typed_widget_output(
        Widget::handle_input(
            &mut selectable,
            bounds,
            WidgetInput::KeyPress(WidgetKey::Space),
        ),
        radiant::widgets::SelectableMessage::SelectionChanged { selected: true },
    );
}
