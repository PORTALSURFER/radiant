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
fn interactive_row_emits_secondary_activation() {
    let mut row = InteractiveRowWidget::new(31, WidgetSizing::fixed(Vector2::new(120.0, 18.0)));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));

    assert_eq!(
        row.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(16.0, 8.0),
                button: PointerButton::Secondary,
                modifiers: Default::default(),
            },
        ),
        Some(InteractiveRowMessage::SecondaryActivate {
            position: Point::new(16.0, 8.0)
        })
    );
    assert!(row.common.state.hovered);
    assert!(!row.common.state.pressed);
}

#[test]
fn interactive_row_clears_stale_hover_when_requested() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));
    let mut previous =
        InteractiveRowWidget::new(32, WidgetSizing::fixed(Vector2::new(120.0, 18.0)));
    assert_eq!(
        previous.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(16.0, 8.0),
            },
        ),
        None
    );
    assert!(previous.common.state.hovered);

    let mut row = InteractiveRowWidget::new(32, WidgetSizing::fixed(Vector2::new(120.0, 18.0)))
        .clear_hover_on_sync();
    row.synchronize_from_previous(&previous);

    assert!(!row.common.state.hovered);
}

#[test]
fn interactive_row_active_drag_source_releases_after_refresh() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));
    let mut row = InteractiveRowWidget::new(33, WidgetSizing::fixed(Vector2::new(120.0, 18.0)))
        .with_drag()
        .with_drag_source(true);

    assert_eq!(
        row.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(34.0, 8.0),
            },
        ),
        None,
        "runtime drag previews should own movement while the row is only the retained source"
    );
    assert_eq!(
        row.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(220.0, 90.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(InteractiveRowMessage::Drag(DragHandleMessage::Ended {
            position: Point::new(220.0, 90.0),
        }))
    );
}

#[test]
fn interactive_row_suppresses_hover_during_external_drag() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));
    let mut row = InteractiveRowWidget::new(34, WidgetSizing::fixed(Vector2::new(120.0, 18.0)))
        .with_drop_target(true);
    row.common.state.hovered = true;

    assert_eq!(
        row.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(16.0, 8.0),
            },
        ),
        None
    );
    assert!(!row.common.state.hovered);
}

#[test]
fn pointer_shield_blocks_configured_pointer_events_inside_bounds() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));
    let shield = PointerShieldWidget::pointer_move_only(true).with_pointer_release(true);

    assert_eq!(
        shield.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(16.0, 8.0),
            },
        ),
        Some(PointerShieldMessage::PointerMove {
            position: Point::new(16.0, 8.0),
        })
    );
    assert_eq!(
        shield.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(18.0, 8.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(PointerShieldMessage::PointerRelease {
            position: Point::new(18.0, 8.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        })
    );
    assert_eq!(
        shield.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(18.0, 8.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        None
    );
}

#[test]
fn pointer_shield_stays_quiet_when_inactive_or_outside_bounds() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 18.0));
    let inactive = PointerShieldWidget::fill(false);
    let active = PointerShieldWidget::fill(true);

    assert!(!inactive.accepts_pointer_move());
    assert!(inactive.common.state.disabled);
    assert_eq!(
        inactive.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(16.0, 8.0),
            },
        ),
        None
    );
    assert_eq!(
        active.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(160.0, 8.0),
            },
        ),
        None
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
