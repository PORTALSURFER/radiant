use super::{Command, RepaintScope, flatten::additional_reserve_for_target_capacity};

#[test]
fn batch_drops_empty_commands_and_preserves_message_order() {
    let command = Command::batch([
        Command::none(),
        Command::message("first"),
        Command::batch([Command::message("second")]),
    ]);

    assert_eq!(command.into_messages(), vec!["first", "second"]);
}

#[test]
fn batch_flattens_nested_command_groups() {
    let command = Command::batch([
        Command::batch([Command::message("first")]),
        Command::batch([
            Command::none(),
            Command::batch([Command::message("second")]),
        ]),
    ]);

    let Command::Batch(commands) = &command else {
        panic!("non-empty nested batch should stay a batch");
    };

    assert_eq!(commands.len(), 2);
    assert!(matches!(commands[0], Command::Message("first")));
    assert!(matches!(commands[1], Command::Message("second")));
    assert_eq!(command.into_messages(), vec!["first", "second"]);
}

#[test]
fn message_flattening_preserves_nested_message_order() {
    let command = Command::Batch(vec![
        Command::Batch(vec![
            Command::message("first"),
            Command::request_repaint(),
            Command::Batch(vec![
                Command::message("second"),
                Command::none(),
                Command::Batch(vec![Command::message("third")]),
            ]),
        ]),
        Command::Batch(vec![
            Command::request_paint_only(),
            Command::message("second"),
        ]),
    ]);

    let messages = command.into_messages();

    assert_eq!(messages, vec!["first", "second", "third", "second"]);
    assert!(messages.capacity() >= messages.len());
}

#[test]
fn message_flattening_reserves_only_recursive_immediate_messages() {
    let command = Command::Batch(vec![
        Command::request_repaint(),
        Command::Batch(vec![
            Command::message("first"),
            Command::after(std::time::Duration::from_millis(1), "delayed"),
            Command::Batch(vec![Command::message("second"), Command::none()]),
        ]),
        Command::request_paint_only(),
    ]);

    let messages = command.into_messages();

    assert_eq!(messages, vec!["first", "second"]);
    assert_eq!(messages.capacity(), messages.len());
}

#[test]
fn message_flattening_into_reuses_output_storage() {
    let command = Command::batch([
        Command::message("first"),
        Command::batch([Command::message("second")]),
    ]);
    let mut messages = Vec::with_capacity(8);
    messages.push("stale");
    let capacity = messages.capacity();

    command.into_messages_into(&mut messages);

    assert_eq!(messages, ["first", "second"]);
    assert_eq!(messages.capacity(), capacity);

    Command::<&str>::none().into_messages_into(&mut messages);

    assert!(messages.is_empty());
    assert_eq!(messages.capacity(), capacity);
}

#[test]
fn message_flattening_reserve_helper_treats_hint_as_target_capacity() {
    assert_eq!(additional_reserve_for_target_capacity(0, 8, 12), 12);
    assert_eq!(additional_reserve_for_target_capacity(3, 8, 12), 9);
    assert_eq!(additional_reserve_for_target_capacity(0, 12, 8), 0);
    assert_eq!(additional_reserve_for_target_capacity(0, 12, 12), 0);
}

#[test]
fn batch_collapses_single_command_groups() {
    let command = Command::batch([Command::none(), Command::batch([Command::message("only")])]);

    assert!(matches!(command, Command::Message("only")));
}

#[test]
fn batch_flattens_exact_single_nested_command_group() {
    let command = Command::batch([Command::batch([
        Command::none(),
        Command::message("first"),
        Command::batch([Command::message("second")]),
    ])]);

    let Command::Batch(commands) = &command else {
        panic!("single nested batch with multiple commands should flatten");
    };

    assert_eq!(commands.len(), 2);
    assert!(matches!(commands[0], Command::Message("first")));
    assert!(matches!(commands[1], Command::Message("second")));
    assert_eq!(command.into_messages(), vec!["first", "second"]);
}

#[test]
fn repaint_requests_are_detected_through_nested_batches() {
    let command = Command::<()>::batch([
        Command::none(),
        Command::batch([Command::request_repaint()]),
    ]);

    assert!(command.requests_repaint());
}

#[test]
fn paint_only_requests_are_detected_through_nested_batches() {
    let command = Command::<()>::batch([Command::batch([
        Command::none(),
        Command::request_paint_only(),
    ])]);

    assert!(command.requests_repaint());
    assert!(command.requests_paint_only());
    assert!(!Command::<()>::request_repaint().requests_paint_only());
}

#[test]
fn repaint_scope_merges_nested_batches_with_surface_winning() {
    let command = Command::<()>::batch([
        Command::repaint(RepaintScope::PaintOnly),
        Command::batch([Command::none(), Command::repaint(RepaintScope::Surface)]),
    ]);

    assert_eq!(command.repaint_scope(), Some(RepaintScope::Surface));
    assert!(!command.requests_paint_only());
}

#[test]
fn dpi_scale_commands_request_surface_repaint_without_flattening_messages() {
    let command = Command::<&str>::set_dpi_scale(crate::theme::DpiScale::new(2.0));

    assert!(!command.is_empty());
    assert_eq!(command.repaint_scope(), Some(RepaintScope::Surface));
    assert!(command.requests_repaint());
    assert_eq!(command.into_messages(), Vec::<&str>::new());
}

#[test]
fn window_size_commands_request_surface_repaint_without_flattening_messages() {
    let command =
        Command::<&str>::set_window_logical_size(crate::layout::Vector2::new(760.0, 520.0));

    assert!(!command.is_empty());
    assert_eq!(command.repaint_scope(), Some(RepaintScope::Surface));
    assert!(command.requests_repaint());
    assert_eq!(command.into_messages(), Vec::<&str>::new());
}

#[test]
fn drag_with_external_batches_preview_before_external_payload() {
    let command = Command::begin_drag_with_external(
        crate::runtime::DragRequest::new(
            crate::runtime::DragPreview::sized("Sample", crate::layout::Vector2::new(120.0, 24.0)),
            crate::layout::Point::new(12.0, 16.0),
        ),
        crate::runtime::ExternalDragRequest::files(
            [std::path::PathBuf::from(r"C:\samples\kick.wav")],
            "Sample",
        ),
        |_| "done",
    );

    let Command::Batch(commands) = command else {
        panic!("combined drag command should batch preview and external payload");
    };

    assert_eq!(commands.len(), 2);
    assert!(matches!(commands[0], Command::BeginDrag { .. }));
    assert!(matches!(
        commands[1],
        Command::BeginExternalDrag {
            on_completed: Some(_),
            ..
        }
    ));
}

#[test]
fn drag_session_command_uses_available_drag_surfaces() {
    let drag = crate::runtime::DragRequest::new(
        crate::runtime::DragPreview::sized("Sample", crate::layout::Vector2::new(120.0, 24.0)),
        crate::layout::Point::new(12.0, 16.0),
    );
    let external = crate::runtime::ExternalDragRequest::files(
        [std::path::PathBuf::from(r"C:\samples\kick.wav")],
        "Sample",
    );

    let command =
        Command::begin_drag_session(Some(drag.clone()), Some(external.clone()), |_| "done");
    let Command::Batch(commands) = command else {
        panic!("combined drag session should batch preview and external payload");
    };
    assert_eq!(commands.len(), 2);
    assert!(matches!(commands[0], Command::BeginDrag { .. }));
    assert!(matches!(commands[1], Command::BeginExternalDrag { .. }));

    assert!(matches!(
        Command::<&str>::begin_drag_session(Some(drag), None, |_| "done"),
        Command::BeginDrag { .. }
    ));
    assert!(matches!(
        Command::begin_drag_session(None, Some(external), |_| "done"),
        Command::BeginExternalDrag { .. }
    ));
    assert!(Command::<&str>::begin_drag_session(None, None, |_| "done").is_empty());
}
