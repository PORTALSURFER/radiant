use super::*;

#[test]
fn dense_row_policy_drag_session_motion_clears_hover_for_input_only_rows() {
    let surface = interactive_row_underlay(text("Sample"))
        .dense_row_policy(DenseRowPolicy::new().drag_session_motion(true))
        .input_id(777)
        .mapped(|_| ())
        .size(140.0, 22.0)
        .into_surface();
    let _ = surface.frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 22.0)),
        &Default::default(),
    );

    let mut row = surface
        .find_widget(777)
        .and_then(|widget| {
            widget
                .widget()
                .as_any()
                .downcast_ref::<crate::widgets::InteractiveRowWidget>()
        })
        .expect("underlay should preserve the configured input row")
        .clone();

    assert!(row.props.drag_active);
    assert!(!row.props.drag_source);
    assert!(!row.props.droppable);
    assert!(row.props.pointer_motion_active);

    row.common_mut().state.hovered = true;
    assert_eq!(
        row.handle_input(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 22.0)),
            WidgetInput::pointer_move(Point::new(8.0, 6.0)),
        ),
        None
    );
    assert!(
        !row.common().state.hovered,
        "drag-session motion rows should clear stale hover instead of painting hover chrome"
    );
}

#[test]
fn interactive_row_actions_route_common_row_messages() {
    fn action_row() -> ViewNode<DemoMessage> {
        interactive_row_underlay(text("Collection"))
            .input_id(771)
            .actions(
                InteractiveRowActions::new()
                    .activate(|| DemoMessage::Activate)
                    .double_activate(|| DemoMessage::DoubleActivate)
                    .drop(|| DemoMessage::Drop)
                    .hover(DemoMessage::HoverDrop)
                    .hover_drop(DemoMessage::HoverDrop)
                    .secondary(DemoMessage::Secondary),
            )
            .size(140.0, 22.0)
    }

    let hover = Point::new(4.0, 9.0);
    let secondary = Point::new(10.0, 12.0);

    assert_eq!(
        action_row().view_dispatch_widget_output(
            771,
            WidgetOutput::typed(InteractiveRowMessage::Hover { position: hover }),
        ),
        Some(DemoMessage::HoverDrop(hover))
    );
    assert_eq!(
        action_row()
            .view_dispatch_widget_output(771, WidgetOutput::typed(InteractiveRowMessage::Drop),),
        Some(DemoMessage::Drop)
    );
    assert_eq!(
        action_row().view_dispatch_widget_output(
            771,
            WidgetOutput::typed(InteractiveRowMessage::HoverDropTarget { position: hover }),
        ),
        Some(DemoMessage::HoverDrop(hover))
    );
    assert_eq!(
        action_row().view_dispatch_widget_output(
            771,
            WidgetOutput::typed(InteractiveRowMessage::SecondaryActivate {
                position: secondary,
            }),
        ),
        Some(DemoMessage::Secondary(secondary))
    );
    assert_eq!(
        action_row().view_dispatch_widget_output(
            771,
            WidgetOutput::typed(InteractiveRowMessage::DoubleActivate),
        ),
        Some(DemoMessage::DoubleActivate)
    );
}

#[test]
fn interactive_row_actions_route_modifier_activation_for_embedded_rows() {
    let modifiers = crate::widgets::PointerModifiers {
        shift: true,
        command: true,
        ..crate::widgets::PointerModifiers::default()
    };
    let actions = InteractiveRowActions::new()
        .activate(|| DemoMessage::Activate)
        .activate_with_modifiers(DemoMessage::ActivateWithModifiers);

    assert_eq!(
        actions.route(InteractiveRowMessage::Activate),
        Some(DemoMessage::ActivateWithModifiers(
            crate::widgets::PointerModifiers::default()
        ))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::ActivateWithModifiers { modifiers }),
        Some(DemoMessage::ActivateWithModifiers(modifiers))
    );
}

#[test]
fn interactive_row_actions_route_keyed_activation_and_secondary_actions() {
    let actions = InteractiveRowActions::new()
        .activate_key(String::from("target-a"), DemoMessage::ActivateKey)
        .double_activate_key(String::from("target-b"), DemoMessage::DoubleActivateKey)
        .secondary_key(String::from("target-c"), DemoMessage::SecondaryKey);
    let secondary = Point::new(8.0, 14.0);

    assert_eq!(
        actions.route(InteractiveRowMessage::Activate),
        Some(DemoMessage::ActivateKey(String::from("target-a")))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::DoubleActivate),
        Some(DemoMessage::DoubleActivateKey(String::from("target-b")))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::SecondaryActivate {
            position: secondary
        }),
        Some(DemoMessage::SecondaryKey(
            String::from("target-c"),
            secondary
        ))
    );
}

#[test]
fn interactive_row_actions_route_keyed_primary_and_secondary_actions() {
    let actions = row_actions().primary_secondary_key(
        String::from("target-a"),
        DemoMessage::ActivateKey,
        DemoMessage::SecondaryKey,
    );
    let secondary = Point::new(8.0, 14.0);

    assert_eq!(
        actions.route(InteractiveRowMessage::Activate),
        Some(DemoMessage::ActivateKey(String::from("target-a")))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::SecondaryActivate {
            position: secondary
        }),
        Some(DemoMessage::SecondaryKey(
            String::from("target-a"),
            secondary
        ))
    );
}

#[test]
fn interactive_row_actions_route_keyed_modifier_activation_and_drag() {
    let modifiers = crate::widgets::PointerModifiers {
        alt: true,
        ..crate::widgets::PointerModifiers::default()
    };
    let drag = crate::widgets::DragHandleMessage::moved(Point::new(4.0, 6.0));
    let actions = InteractiveRowActions::new()
        .activate_with_modifiers_key(
            String::from("target-a"),
            DemoMessage::ActivateWithModifiersKey,
        )
        .drag_key(String::from("target-b"), DemoMessage::DragKey);

    assert_eq!(
        actions.route(InteractiveRowMessage::ActivateWithModifiers { modifiers }),
        Some(DemoMessage::ActivateWithModifiersKey(
            String::from("target-a"),
            modifiers
        ))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::Drag(drag)),
        Some(DemoMessage::DragKey(String::from("target-b"), drag))
    );
}
