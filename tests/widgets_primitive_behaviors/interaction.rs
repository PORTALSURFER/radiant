use super::*;

#[test]
fn list_item_invocation_is_public_and_deterministic() {
    let mut item = ListItemWidget::new(
        9,
        "Document",
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 28.0));

    assert_eq!(
        item.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(12.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        None
    );
    assert_eq!(
        item.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(12.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(ListItemMessage::Invoked)
    );

    let _ = item.handle_input(bounds, WidgetInput::FocusChanged(true));
    assert_eq!(
        item.handle_input(bounds, WidgetInput::KeyPress(WidgetKey::Enter)),
        Some(ListItemMessage::Invoked)
    );
}

#[test]
fn drag_handle_emits_captured_drag_lifecycle() {
    let mut handle = DragHandleWidget::new(12, WidgetSizing::fixed(Vector2::new(24.0, 24.0)));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(24.0, 24.0));

    assert_eq!(
        handle.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(12.0, 12.0),
            },
        ),
        None
    );
    assert!(handle.common.state.hovered);
    assert_eq!(
        handle.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(12.0, 12.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(DragHandleMessage::Started {
            position: Point::new(12.0, 12.0),
        })
    );
    assert!(handle.common.state.pressed);
    assert!(handle.common.state.active);
    assert_eq!(
        handle.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(12.0, 70.0),
            },
        ),
        Some(DragHandleMessage::Moved {
            position: Point::new(12.0, 70.0),
        })
    );
    assert_eq!(
        handle.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(12.0, 70.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(DragHandleMessage::Ended {
            position: Point::new(12.0, 70.0),
        })
    );
    assert!(!handle.common.state.pressed);
    assert!(!handle.common.state.active);
}

#[test]
fn selectable_toggles_selected_state_with_pointer_and_keyboard() {
    let mut selectable = SelectableWidget::new(
        11,
        "Choice",
        false,
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 28.0));

    assert!(!selectable.common.state.selected);
    assert_eq!(
        selectable.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(12.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        None
    );
    assert_eq!(
        selectable.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(12.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(SelectableMessage::SelectionChanged { selected: true })
    );
    assert!(selectable.common.state.selected);

    let _ = selectable.handle_input(bounds, WidgetInput::FocusChanged(true));
    assert_eq!(
        selectable.handle_input(bounds, WidgetInput::KeyPress(WidgetKey::Space)),
        Some(SelectableMessage::SelectionChanged { selected: false })
    );
    assert!(!selectable.common.state.selected);
}
