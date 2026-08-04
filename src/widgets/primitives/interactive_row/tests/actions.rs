use super::*;
use std::{cell::RefCell, rc::Rc};

fn assert_send_sync<T: Send + Sync>() {}

fn double_activation_message() -> InteractiveRowMessage {
    InteractiveRowMessage::DoubleActivate {
        provenance: InteractionProvenance::Pointer {
            modifiers: PointerModifiers {
                command: true,
                ..PointerModifiers::default()
            },
            timestamp: None,
            sequence_range: None,
        },
    }
}

fn activation_message() -> InteractiveRowMessage {
    InteractiveRowMessage::Activate {
        provenance: InteractionProvenance::Programmatic,
    }
}

fn modifier_activation_message(modifiers: PointerModifiers) -> InteractiveRowMessage {
    InteractiveRowMessage::ActivateWithModifiers {
        provenance: InteractionProvenance::Pointer {
            modifiers,
            timestamp: None,
            sequence_range: None,
        },
    }
}

#[test]
fn legacy_interactive_row_actions_remain_send_sync() {
    assert_send_sync::<InteractiveRowActions<()>>();
}

#[test]
fn interactive_row_actions_routes_single_or_double_activation_to_same_action() {
    let actions = InteractiveRowActions::new()
        .primary(|| "activate")
        .double(|| "activate");

    assert_eq!(actions.route(activation_message()), Some("activate"));
    assert_eq!(actions.route(double_activation_message()), Some("activate"));
}

#[test]
fn interactive_row_actions_routes_single_or_double_activation_with_key() {
    let actions = InteractiveRowActions::new()
        .primary_key("folder", |key| (key, "activate"))
        .double_key("folder", |key| (key, "activate"));

    assert_eq!(
        actions.route(activation_message()),
        Some(("folder", "activate"))
    );
    assert_eq!(
        actions.route(double_activation_message()),
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
        actions.route(modifier_activation_message(modifiers)),
        Some(modifiers)
    );
    assert_eq!(
        actions.route(double_activation_message()),
        Some(PointerModifiers::default())
    );
}

#[test]
fn interactive_row_actions_routes_modifier_primary_and_double_with_one_key() {
    let actions = InteractiveRowActions::new().primary_with_modifiers_and_double_key(
        "file",
        |key, modifiers| (key, "activate", modifiers),
        |key| (key, "double", PointerModifiers::default()),
    );
    let modifiers = PointerModifiers {
        shift: true,
        command: true,
        ..PointerModifiers::default()
    };

    assert_eq!(
        actions.route(modifier_activation_message(modifiers)),
        Some(("file", "activate", modifiers))
    );
    assert_eq!(
        actions.route(double_activation_message()),
        Some(("file", "double", PointerModifiers::default()))
    );
}

#[test]
fn interactive_row_actions_routes_keyed_modifier_activation_secondary_and_drag() {
    let actions = InteractiveRowActions::new()
        .hover_key("file", |key, position| {
            (key, "hover", PointerModifiers::default(), position)
        })
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
        actions.route(InteractiveRowMessage::Hover {
            position,
            metadata: Default::default(),
        }),
        Some(("file", "hover", PointerModifiers::default(), position))
    );
    assert_eq!(
        actions.route(modifier_activation_message(modifiers)),
        Some(("file", "activate", modifiers, Point::new(0.0, 0.0)))
    );
    assert_eq!(
        actions.route(double_activation_message()),
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
        actions.route(activation_message()),
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
        actions.route(activation_message()),
        Some(("folder", "activate", Point::new(0.0, 0.0)))
    );
    assert_eq!(
        actions.route(double_activation_message()),
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
        actions.route(InteractiveRowMessage::HoverDropTarget {
            position,
            metadata: Default::default(),
        }),
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
        actions.route(InteractiveRowMessage::HoverDropTarget {
            position,
            metadata: Default::default(),
        }),
        Some(("folder", "hover_drop", position))
    );
    assert_eq!(
        actions.route(InteractiveRowMessage::ClearDropTarget {
            position,
            metadata: Default::default(),
        }),
        Some(("folder", "clear_drop", position))
    );
}

#[test]
fn local_actions_route_full_matrix_with_non_send_key_and_release_callbacks() {
    #[derive(Clone)]
    struct LocalKey(Rc<RefCell<usize>>);

    let state = Rc::new(RefCell::new(0usize));
    let key = LocalKey(Rc::clone(&state));
    let captured = Rc::clone(&state);
    let actions = InteractiveRowLocalActions::new()
        .hover_key(key.clone(), move |key, _| {
            *key.0.borrow_mut() += 1;
            "hover"
        })
        .primary_with_modifiers_key(key.clone(), move |key, _| {
            *key.0.borrow_mut() += 1;
            "activate"
        })
        .double_activate_key(key.clone(), move |key| {
            *key.0.borrow_mut() += 1;
            "double"
        })
        .secondary_key(key.clone(), move |key, _| {
            *key.0.borrow_mut() += 1;
            "secondary"
        })
        .drag_key(key.clone(), move |key, _| {
            *key.0.borrow_mut() += 1;
            "drag"
        })
        .tracked_drop_candidate_key(
            key,
            move |key| {
                *key.0.borrow_mut() += 1;
                "drop"
            },
            move |key, _| {
                *key.0.borrow_mut() += 1;
                "hover_drop"
            },
            move |key, _| {
                *key.0.borrow_mut() += 1;
                "clear_drop"
            },
        );

    let position = Point::new(12.0, 24.0);
    let messages = [
        (
            InteractiveRowMessage::Hover {
                position,
                metadata: Default::default(),
            },
            "hover",
        ),
        (
            modifier_activation_message(PointerModifiers {
                shift: true,
                ..PointerModifiers::default()
            }),
            "activate",
        ),
        (double_activation_message(), "double"),
        (
            InteractiveRowMessage::SecondaryActivate { position },
            "secondary",
        ),
        (
            InteractiveRowMessage::Drag(DragHandleMessage::moved(position)),
            "drag",
        ),
        (InteractiveRowMessage::Drop, "drop"),
        (
            InteractiveRowMessage::HoverDropTarget {
                position,
                metadata: Default::default(),
            },
            "hover_drop",
        ),
        (
            InteractiveRowMessage::ClearDropTarget {
                position,
                metadata: Default::default(),
            },
            "clear_drop",
        ),
    ];
    for (message, expected) in messages {
        assert_eq!(actions.route(message), Some(expected));
    }
    assert_eq!(*captured.borrow(), messages.len());
    assert!(Rc::strong_count(&captured) > 1);
    drop(state);
    drop(actions);
    assert_eq!(Rc::strong_count(&captured), 1);
}

#[test]
fn shared_and_local_action_routers_produce_the_same_representative_messages() {
    let shared = InteractiveRowActions::new()
        .primary(|| "activate")
        .secondary(|_| "secondary")
        .drag(|_| "drag")
        .tracked_drop_candidate_key("row", |_| "drop", |_, _| "hover_drop", |_, _| "clear_drop");
    let local = InteractiveRowLocalActions::new()
        .primary(|| "activate")
        .secondary(|_| "secondary")
        .drag(|_| "drag")
        .tracked_drop_candidate_key(
            Rc::new(()),
            |_| "drop",
            |_, _| "hover_drop",
            |_, _| "clear_drop",
        );
    let position = Point::new(12.0, 24.0);
    for message in [
        activation_message(),
        InteractiveRowMessage::SecondaryActivate { position },
        InteractiveRowMessage::Drag(DragHandleMessage::started(position)),
        InteractiveRowMessage::Drop,
        InteractiveRowMessage::HoverDropTarget {
            position,
            metadata: Default::default(),
        },
        InteractiveRowMessage::ClearDropTarget {
            position,
            metadata: Default::default(),
        },
    ] {
        assert_eq!(shared.route(message), local.route(message));
    }
}
