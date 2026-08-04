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
        Widget::handle_input(
            &mut button,
            bounds,
            WidgetInput::key_press(WidgetKey::Enter),
        ),
        radiant::widgets::ButtonMessage::Activate,
    );

    assert_eq!(
        Widget::handle_input(&mut toggle, bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_typed_widget_output(
        Widget::handle_input(
            &mut toggle,
            bounds,
            WidgetInput::key_press(WidgetKey::Space),
        ),
        radiant::widgets::ToggleMessage::ValueChanged {
            checked: true,
            provenance: radiant::widgets::InteractionProvenance::Keyboard { timestamp: None },
        },
    );

    assert_eq!(
        Widget::handle_input(&mut input, bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_typed_widget_output(
        Widget::handle_input(&mut input, bounds, WidgetInput::character('z')),
        radiant::widgets::TextInputMessage::Changed {
            value: String::from("abz"),
        },
    );

    assert_eq!(
        Widget::handle_input(&mut badge, bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_typed_widget_output(
        Widget::handle_input(&mut badge, bounds, WidgetInput::key_press(WidgetKey::Enter)),
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
                timestamp: None,
            },
        ),
        radiant::widgets::DragHandleMessage::started(Point::new(10.0, 10.0)),
    );
    assert_typed_widget_output(
        Widget::handle_input(
            &mut drag,
            bounds,
            WidgetInput::pointer_move(Point::new(10.0, 38.0)),
        ),
        radiant::widgets::DragHandleMessage::Moved {
            position: Point::new(10.0, 38.0),
            metadata: radiant::widgets::DragHandleMetadata::empty(),
        },
    );

    assert_eq!(
        Widget::handle_input(&mut item, bounds, WidgetInput::FocusChanged(true)),
        None
    );
    assert_typed_widget_output(
        Widget::handle_input(&mut item, bounds, WidgetInput::key_press(WidgetKey::Enter)),
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
            WidgetInput::key_press(WidgetKey::Space),
        ),
        radiant::widgets::SelectableMessage::SelectionChanged { selected: true },
    );
}

#[test]
fn toggle_message_exposes_explicit_programmatic_provenance_and_hashes_it() {
    use std::collections::{HashSet, hash_map::DefaultHasher};
    use std::hash::{Hash, Hasher};

    fn assert_toggle_message_traits<T: Clone + Copy + Debug + PartialEq + Eq + Hash>() {}

    assert_toggle_message_traits::<radiant::widgets::ToggleMessage>();

    let programmatic = radiant::widgets::ToggleMessage::ValueChanged {
        checked: true,
        provenance: radiant::widgets::InteractionProvenance::Programmatic,
    };
    let keyboard = radiant::widgets::ToggleMessage::ValueChanged {
        checked: true,
        provenance: radiant::widgets::InteractionProvenance::Keyboard { timestamp: None },
    };
    assert_ne!(programmatic, keyboard);
    assert_eq!(
        programmatic,
        radiant::widgets::ToggleMessage::ValueChanged {
            checked: true,
            provenance: radiant::widgets::InteractionProvenance::Programmatic,
        }
    );

    let mut programmatic_hasher = DefaultHasher::new();
    programmatic.hash(&mut programmatic_hasher);
    let mut keyboard_hasher = DefaultHasher::new();
    keyboard.hash(&mut keyboard_hasher);
    assert_ne!(programmatic_hasher.finish(), keyboard_hasher.finish());

    let messages = HashSet::from([programmatic, keyboard]);
    assert_eq!(messages.len(), 2);
}
