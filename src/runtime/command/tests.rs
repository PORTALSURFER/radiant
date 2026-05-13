use super::Command;

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
fn batch_collapses_single_command_groups() {
    let command = Command::batch([Command::none(), Command::batch([Command::message("only")])]);

    assert!(matches!(command, Command::Message("only")));
}

#[test]
fn repaint_requests_are_detected_through_nested_batches() {
    let command = Command::<()>::batch([
        Command::none(),
        Command::batch([Command::request_repaint()]),
    ]);

    assert!(command.requests_repaint());
}
