use super::*;

#[test]
fn interactive_row_actions_routes_single_or_double_activation_to_same_action() {
    let actions = InteractiveRowActions::new()
        .primary(|| "activate")
        .double(|| "activate");

    assert_eq!(
        actions.route(InteractiveRowMessage::Activate),
        Some("activate")
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::DoubleActivate),
        Some("activate")
    );
}

#[test]
fn interactive_row_actions_routes_single_or_double_activation_with_key() {
    let actions = InteractiveRowActions::new()
        .primary_key("folder", |key| (key, "activate"))
        .double_key("folder", |key| (key, "activate"));

    assert_eq!(
        actions.route(InteractiveRowMessage::Activate),
        Some(("folder", "activate"))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::DoubleActivate),
        Some(("folder", "activate"))
    );
}

#[test]
fn interactive_row_actions_routes_single_modifiers_or_double_to_same_action() {
    let actions = InteractiveRowActions::new()
        .primary_with_modifiers(|modifiers| modifiers)
        .double(PointerModifiers::default);
    let modifiers = PointerModifiers {
        shift: true,
        command: true,
        ..PointerModifiers::default()
    };

    assert_eq!(
        actions.route(InteractiveRowMessage::ActivateWithModifiers { modifiers }),
        Some(modifiers)
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::DoubleActivate),
        Some(PointerModifiers::default())
    );
}

#[test]
fn interactive_row_actions_routes_keyed_modifier_activation_secondary_and_drag() {
    let actions = InteractiveRowActions::new()
        .primary_with_modifiers_key("file", |key, modifiers| {
            (key, "activate", modifiers, Point::new(0.0, 0.0))
        })
        .double_key("file", |key| {
            (
                key,
                "activate",
                PointerModifiers::default(),
                Point::new(0.0, 0.0),
            )
        })
        .secondary_key("file", |key, position| {
            (key, "secondary", PointerModifiers::default(), position)
        })
        .drag_key("file", |key, drag| {
            (key, "drag", PointerModifiers::default(), drag.position())
        });
    let modifiers = PointerModifiers {
        shift: true,
        command: true,
        ..PointerModifiers::default()
    };
    let position = Point::new(12.0, 24.0);

    assert_eq!(
        actions.route(InteractiveRowMessage::ActivateWithModifiers { modifiers }),
        Some(("file", "activate", modifiers, Point::new(0.0, 0.0)))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::DoubleActivate),
        Some((
            "file",
            "activate",
            PointerModifiers::default(),
            Point::new(0.0, 0.0)
        ))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::SecondaryActivate { position }),
        Some(("file", "secondary", PointerModifiers::default(), position))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::Drag(DragHandleMessage::started(
            position
        ))),
        Some(("file", "drag", PointerModifiers::default(), position))
    );
}

#[test]
fn interactive_row_actions_routes_activation_and_secondary_with_one_key() {
    let actions = InteractiveRowActions::new().primary_secondary_key(
        "source",
        |key| (key, "activate", Point::new(0.0, 0.0)),
        |key, position| (key, "secondary", position),
    );
    let position = Point::new(12.0, 24.0);

    assert_eq!(
        actions.route(InteractiveRowMessage::Activate),
        Some(("source", "activate", Point::new(0.0, 0.0)))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::SecondaryActivate { position }),
        Some(("source", "secondary", position))
    );
}

#[test]
fn interactive_row_actions_routes_keyed_tree_drop_row_actions() {
    let actions = InteractiveRowActions::new()
        .primary_key("folder", |key| (key, "activate", Point::new(0.0, 0.0)))
        .double_key("folder", |key| (key, "activate", Point::new(0.0, 0.0)))
        .secondary_key("folder", |key, position| (key, "secondary", position))
        .drag_key("folder", |key, drag| (key, "drag", drag.position()))
        .drop_target_key(
            "folder",
            |key| (key, "drop", Point::new(0.0, 0.0)),
            |key, position| (key, "hover_drop", position),
        );
    let position = Point::new(12.0, 24.0);

    assert_eq!(
        actions.route(InteractiveRowMessage::Activate),
        Some(("folder", "activate", Point::new(0.0, 0.0)))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::DoubleActivate),
        Some(("folder", "activate", Point::new(0.0, 0.0)))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::SecondaryActivate { position }),
        Some(("folder", "secondary", position))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::Drag(DragHandleMessage::started(
            position
        ))),
        Some(("folder", "drag", position))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::Drop),
        Some(("folder", "drop", Point::new(0.0, 0.0)))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::HoverDropTarget { position }),
        Some(("folder", "hover_drop", position))
    );
}

#[test]
fn interactive_row_actions_routes_tracked_drop_candidate_clear() {
    let actions = InteractiveRowActions::new().tracked_drop_candidate_key(
        "folder",
        |key| (key, "drop", Point::new(0.0, 0.0)),
        |key, position| (key, "hover_drop", position),
        |key, position| (key, "clear_drop", position),
    );
    let position = Point::new(12.0, 24.0);

    assert_eq!(
        actions.route(InteractiveRowMessage::Drop),
        Some(("folder", "drop", Point::new(0.0, 0.0)))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::HoverDropTarget { position }),
        Some(("folder", "hover_drop", position))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::ClearDropTarget { position }),
        Some(("folder", "clear_drop", position))
    );
}
