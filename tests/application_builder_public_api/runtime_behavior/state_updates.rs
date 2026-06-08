use super::*;

#[test]
fn stateful_app_builder_projects_updates_and_preserves_commands() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .title("Counter")
        .size(320, 120)
        .view(|state| {
            ui::column([
                ui::text(format!("Count: {}", state.count)),
                ui::button("Increment").message(DemoMessage::Increment),
            ])
        })
        .update_command(|state, message| match message {
            DemoMessage::Increment => {
                state.count += 1;
                Command::request_repaint()
            }
        })
        .into_bridge();

    let before = bridge.project_surface();
    let increment = before
        .dispatch_widget_output(
            3,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("generated button should route through the same surface mapper");

    let command = bridge.update(increment);

    assert!(command.requests_repaint());
    let after = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, 2, "text").text,
        "Count: 1"
    );
}

#[test]
fn handle_message_exposes_update_context_with_clear_app_api_name() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| ui::text(format!("Count: {}", state.count)))
        .handle_message(|state, message, context| match message {
            DemoMessage::Increment => {
                state.count += 1;
                context.request_repaint();
            }
        })
        .into_bridge();

    let command = bridge.update(DemoMessage::Increment);

    assert!(command.requests_repaint());
    let after = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, 1, "text").text,
        "Count: 1"
    );
}

#[test]
fn reducer_remains_compatibility_alias_for_context_aware_handlers() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| ui::text(format!("Count: {}", state.count)))
        .reducer(|state, message, context| match message {
            DemoMessage::Increment => {
                state.count += 1;
                context.request_repaint();
            }
        })
        .into_bridge();

    let command = bridge.update(DemoMessage::Increment);

    assert!(command.requests_repaint());
    let after = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, 1, "text").text,
        "Count: 1"
    );
}

#[test]
fn ordinary_handler_without_repaint_command_requests_surface_repaint_by_default() {
    use radiant::prelude as ui;
    use radiant::runtime::RepaintScope;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| ui::text(format!("Count: {}", state.count)))
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .into_bridge();

    let command = bridge.update(DemoMessage::Increment);

    assert_eq!(command.repaint_scope(), Some(RepaintScope::Surface));
}

#[test]
fn ordinary_handler_explicit_paint_only_is_not_upgraded_to_surface_repaint() {
    use radiant::prelude as ui;
    use radiant::runtime::RepaintScope;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| ui::text(format!("Count: {}", state.count)))
        .handle_message(|state, message, context| match message {
            DemoMessage::Increment => {
                state.count += 1;
                context.request_paint_only();
            }
        })
        .into_bridge();

    let command = bridge.update(DemoMessage::Increment);

    assert_eq!(command.repaint_scope(), Some(RepaintScope::PaintOnly));
}

#[test]
fn ordinary_handler_explicit_surface_repaint_is_preserved() {
    use radiant::prelude as ui;
    use radiant::runtime::RepaintScope;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| ui::text(format!("Count: {}", state.count)))
        .handle_message(|state, message, context| match message {
            DemoMessage::Increment => {
                state.count += 1;
                context.request_repaint();
            }
        })
        .into_bridge();

    let command = bridge.update(DemoMessage::Increment);

    assert_eq!(command.repaint_scope(), Some(RepaintScope::Surface));
}

#[test]
fn repaint_policy_none_disables_ordinary_message_automatic_repaint() {
    use radiant::prelude as ui;

    let mut bridge = ui::app(DemoState::default())
        .view(|state| ui::text(format!("Count: {}", state.count)))
        .handle_message(|state, message, _context| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .repaint_policy(ui::RepaintPolicy::none())
        .into_bridge();

    let command = bridge.update(DemoMessage::Increment);

    assert!(!command.requests_repaint());
}

#[test]
fn repaint_policy_can_skip_frame_messages() {
    use radiant::prelude as ui;

    #[derive(Clone, Debug, PartialEq)]
    enum Message {
        Frame,
        User,
    }

    let mut bridge = ui::app(DemoState::default())
        .view(|state| ui::text(format!("Count: {}", state.count)))
        .handle_message(|state, message, _context| match message {
            Message::Frame => state.count += 1,
            Message::User => state.count += 10,
        })
        .repaint_policy(ui::RepaintPolicy::after_messages_except_value(
            Message::Frame,
        ))
        .into_bridge();

    let frame_command = bridge.update(Message::Frame);
    let user_command = bridge.update(Message::User);

    assert!(!frame_command.requests_repaint());
    assert!(user_command.requests_repaint());
    let after = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, 1, "text").text,
        "Count: 11"
    );
}
