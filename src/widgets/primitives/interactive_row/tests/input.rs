use super::*;

#[test]
fn accessors_expose_identity_and_common_contract_for_custom_row_wrappers() {
    let mut row = InteractiveRowWidget::new(13, WidgetSizing::fixed(Vector2::new(120.0, 22.0)));

    assert_eq!(row.id(), 13);
    assert_eq!(row.common().id, 13);

    row.common_mut().state.hovered = true;
    assert!(row.common().state.hovered);
}

#[test]
fn drop_target_mode_configures_hover_and_drop_only_states() {
    let inactive = InteractiveRowWidget::new(7, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_drop_target(true)
        .with_drop_target_mode(false, true);
    assert!(!inactive.props.droppable);
    assert!(!inactive.props.drag_active);
    assert!(!inactive.props.drop_hover);

    let hover = InteractiveRowWidget::new(8, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_drop_target_mode(true, true);
    assert!(hover.props.droppable);
    assert!(hover.props.drag_active);
    assert!(hover.props.drop_hover);

    let drop_only = InteractiveRowWidget::new(9, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_drop_target_mode(true, false);
    assert!(drop_only.props.droppable);
    assert!(drop_only.props.drag_active);
    assert!(!drop_only.props.drop_hover);
}

#[test]
fn handle_input_mapped_routes_custom_row_output() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut row = InteractiveRowWidget::new(10, WidgetSizing::fixed(Vector2::new(120.0, 22.0)));
    let pointer = Point::new(4.0, 4.0);

    let press = row.handle_input_mapped(
        bounds,
        WidgetInput::PointerPress {
            position: pointer,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
        |_| Some("activated"),
    );
    assert!(press.is_none());

    let release = row
        .handle_input_mapped(
            bounds,
            WidgetInput::PointerRelease {
                position: pointer,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
            |message| message.is_single_activation().then_some("activated"),
        )
        .expect("release maps to typed widget output");
    assert_eq!(release.typed_ref::<&'static str>(), Some(&"activated"));
}

#[test]
fn small_pressed_motion_does_not_start_row_drag() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut row =
        InteractiveRowWidget::new(11, WidgetSizing::fixed(Vector2::new(120.0, 22.0))).with_drag();
    let start = Point::new(8.0, 6.0);
    let tiny_move = Point::new(11.0, 10.0);

    assert_eq!(
        row.handle_input(bounds, WidgetInput::primary_press(start)),
        None
    );
    assert_eq!(
        row.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: tiny_move
            }
        ),
        None
    );
    assert!(!row.dragged);
    assert_eq!(
        row.handle_input(bounds, WidgetInput::primary_release(tiny_move)),
        Some(InteractiveRowMessage::Activate)
    );
}

#[test]
fn focus_loss_preserves_started_row_drag() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut row =
        InteractiveRowWidget::new(11, WidgetSizing::fixed(Vector2::new(120.0, 22.0))).with_drag();
    let start = Point::new(8.0, 6.0);
    let moved = Point::new(16.0, 12.0);
    let release = Point::new(24.0, 12.0);

    assert_eq!(
        row.handle_input(bounds, WidgetInput::primary_press(start)),
        None
    );
    assert_eq!(
        row.handle_input(bounds, WidgetInput::PointerMove { position: moved }),
        Some(InteractiveRowMessage::Drag(DragHandleMessage::Started {
            position: moved
        }))
    );
    assert_eq!(
        row.handle_input(bounds, WidgetInput::FocusChanged(false)),
        None
    );
    assert!(row.common.state.pressed);
    assert!(row.dragged);
    assert_eq!(
        row.handle_input(bounds, WidgetInput::primary_release(release)),
        Some(InteractiveRowMessage::Drag(DragHandleMessage::Ended {
            position: release
        }))
    );
    assert!(!row.common.state.pressed);
    assert!(!row.dragged);
}

#[test]
fn focus_loss_clears_pressed_row_before_drag_starts() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut row =
        InteractiveRowWidget::new(12, WidgetSizing::fixed(Vector2::new(120.0, 22.0))).with_drag();

    assert_eq!(
        row.handle_input(bounds, WidgetInput::primary_press(Point::new(8.0, 6.0))),
        None
    );
    assert!(row.common.state.pressed);
    assert_eq!(
        row.handle_input(bounds, WidgetInput::FocusChanged(false)),
        None
    );
    assert!(!row.common.state.pressed);
    assert!(!row.dragged);
}
