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
fn tracked_drop_candidate_hover_emits_clear_for_non_candidate_when_target_active() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut row = InteractiveRowWidget::new(15, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_tracked_drop_candidate(true, false, false, true);
    let position = Point::new(8.0, 6.0);

    assert_eq!(
        row.handle_input(bounds, WidgetInput::pointer_move(position)),
        Some(InteractiveRowMessage::ClearDropTarget { position })
    );
}

#[test]
fn tracked_drop_candidate_hover_emits_target_for_candidate() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut row = InteractiveRowWidget::new(16, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_tracked_drop_candidate(true, false, true, true);
    let position = Point::new(8.0, 6.0);

    assert_eq!(
        row.handle_input(bounds, WidgetInput::pointer_move(position)),
        Some(InteractiveRowMessage::HoverDropTarget { position })
    );
}

#[test]
fn normal_hover_emits_hover_position() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut quiet_row =
        InteractiveRowWidget::new(17, WidgetSizing::fixed(Vector2::new(120.0, 22.0)));
    let mut row = InteractiveRowWidget::new(18, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_hover_messages(true);
    let position = Point::new(8.0, 6.0);

    assert_eq!(
        quiet_row.handle_input(bounds, WidgetInput::pointer_move(position)),
        None
    );
    assert_eq!(
        row.handle_input(bounds, WidgetInput::pointer_move(position)),
        Some(InteractiveRowMessage::Hover { position })
    );
}

#[test]
fn ordinary_hover_does_not_keep_stable_pointer_motion_routed_by_default() {
    let mut row = InteractiveRowWidget::new(19, WidgetSizing::fixed(Vector2::new(120.0, 22.0)));

    assert!(!row.accepts_pointer_move());
    assert_eq!(
        row.handle_input(
            Rect::from_size(120.0, 22.0),
            WidgetInput::pointer_move(Point::new(8.0, 6.0)),
        ),
        None
    );
    assert!(row.common.state.hovered);
}

#[test]
fn explicit_hover_messages_keep_stable_pointer_motion_routed() {
    let row = InteractiveRowWidget::new(20, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_hover_messages(true);

    assert!(row.accepts_pointer_move());
}

#[test]
fn inert_drag_active_rows_do_not_accept_stable_pointer_motion() {
    let drag_active = InteractiveRowWidget::new(21, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_drag_active(true);
    let drag_source = InteractiveRowWidget::new(22, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_drag_source(true);
    let drop_only = InteractiveRowWidget::new(23, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_drop_target_mode(true, false);
    let non_candidate =
        InteractiveRowWidget::new(24, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
            .with_tracked_drop_candidate(true, false, false, false);

    assert!(!drag_active.accepts_pointer_move());
    assert!(!drag_source.accepts_pointer_move());
    assert!(!drop_only.accepts_pointer_move());
    assert!(!non_candidate.accepts_pointer_move());
}

#[test]
fn active_drag_work_keeps_stable_pointer_motion_routed() {
    let source_motion =
        InteractiveRowWidget::new(25, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
            .with_drag_source(true)
            .with_drag_source_motion(true);
    let drop_hover = InteractiveRowWidget::new(26, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_tracked_drop_candidate(true, false, true, false);
    let drop_clear = InteractiveRowWidget::new(27, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
        .with_tracked_drop_candidate(true, false, false, true);

    assert!(source_motion.accepts_pointer_move());
    assert!(drop_hover.accepts_pointer_move());
    assert!(drop_clear.accepts_pointer_move());
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
fn coalesced_row_drag_preserves_press_origin_and_current_pointer() {
    let bounds = Rect::from_size(120.0, 22.0);
    let mut row =
        InteractiveRowWidget::new(13, WidgetSizing::fixed(Vector2::new(120.0, 22.0))).with_drag();
    let origin = Point::new(8.0, 6.0);
    let destination = Point::new(90.0, 16.0);

    assert_eq!(
        row.handle_input(bounds, WidgetInput::primary_press(origin)),
        None
    );
    assert_eq!(
        row.handle_input(bounds, WidgetInput::pointer_move(destination)),
        Some(InteractiveRowMessage::Drag(
            DragHandleMessage::started_from(origin, destination)
        ))
    );
    assert_eq!(
        row.handle_input(bounds, WidgetInput::primary_release(destination)),
        Some(InteractiveRowMessage::Drag(DragHandleMessage::ended(
            destination
        )))
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
            origin: start,
            position: moved,
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

#[test]
fn double_click_press_keeps_visual_pressed_until_release_without_single_activation() {
    let bounds = Rect::from_size(120.0, 22.0);
    let position = Point::new(8.0, 6.0);
    let mut row =
        InteractiveRowWidget::new(15, WidgetSizing::fixed(Vector2::new(120.0, 22.0))).with_drag();

    assert_eq!(
        row.handle_input(bounds, WidgetInput::primary_double_click(position)),
        Some(InteractiveRowMessage::DoubleActivate)
    );
    assert!(row.common.state.pressed);
    assert!(row.double_activated);
    assert_eq!(
        row.handle_input(bounds, WidgetInput::pointer_move(Point::new(30.0, 6.0))),
        None,
        "the second click must not accidentally enter drag mode"
    );
    assert_eq!(
        row.handle_input(bounds, WidgetInput::primary_release(position)),
        None,
        "double activation must not be followed by a single activation"
    );
    assert!(!row.common.state.pressed);
    assert!(!row.double_activated);
}

#[test]
fn synchronize_from_previous_clears_retained_drag_when_host_drag_becomes_idle() {
    let start = Point::new(8.0, 6.0);
    let mut previous =
        InteractiveRowWidget::new(13, WidgetSizing::fixed(Vector2::new(120.0, 22.0)))
            .with_drag()
            .with_drag_active(true)
            .with_drag_source(true);
    previous.common.state.pressed = true;
    previous.pressed_position = Some(start);
    previous.dragged = true;
    assert!(previous.common.state.pressed);
    assert!(previous.dragged);

    let mut current =
        InteractiveRowWidget::new(13, WidgetSizing::fixed(Vector2::new(120.0, 22.0))).with_drag();
    crate::widgets::Widget::synchronize_from_previous(&mut current, &previous);

    assert!(!current.common.state.pressed);
    assert!(current.pressed_position.is_none());
    assert!(!current.dragged);
}

#[test]
fn synchronize_from_previous_preserves_ordinary_pressed_row_state() {
    let bounds = Rect::from_size(120.0, 22.0);
    let start = Point::new(8.0, 6.0);
    let mut previous =
        InteractiveRowWidget::new(14, WidgetSizing::fixed(Vector2::new(120.0, 22.0))).with_drag();
    let _ = previous.handle_input(bounds, WidgetInput::primary_press(start));
    assert!(previous.common.state.pressed);
    assert_eq!(previous.pressed_position, Some(start));
    assert!(!previous.dragged);

    let mut current =
        InteractiveRowWidget::new(14, WidgetSizing::fixed(Vector2::new(120.0, 22.0))).with_drag();
    crate::widgets::Widget::synchronize_from_previous(&mut current, &previous);

    assert!(current.common.state.pressed);
    assert_eq!(current.pressed_position, Some(start));
    assert!(!current.dragged);
}
