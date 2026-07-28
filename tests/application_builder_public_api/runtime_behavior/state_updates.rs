use super::*;

#[test]
fn stateful_app_builder_projects_updates_and_preserves_context_requests() {
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
        .handle_message(|state, message, context| match message {
            DemoMessage::Increment => {
                state.count += 1;
                context.request_repaint();
            }
        })
        .into_bridge();

    let before = bridge.project_surface();
    let radiant::layout::LayoutNode::Container(container) = before.layout_node() else {
        panic!("counter view should lower to a container");
    };
    let text_id = container.children[0].child.id();
    let button_id = container.children[1].child.id();
    let increment = before
        .dispatch_widget_output(
            button_id,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("generated button should route through the same surface mapper");

    let command = bridge.update(increment);

    assert!(command.requests_repaint());
    let after = bridge.project_surface();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, text_id, "text").text,
        "Count: 1"
    );
}

#[test]
fn stateful_runtime_keeps_ordinary_messages_and_worker_mappers_ui_local() {
    use radiant::prelude as ui;
    use std::{cell::RefCell, rc::Rc};

    #[derive(Clone)]
    enum UiOnlyMessage {
        Load(Rc<RefCell<Vec<&'static str>>>),
        Loaded(Rc<RefCell<Vec<&'static str>>>),
    }

    let events = Rc::new(RefCell::new(Vec::new()));
    let view_events = Rc::clone(&events);
    let bridge = ui::app(DemoState::default())
        .view(move |state| {
            ui::column([
                ui::text(format!("Count: {}", state.count)).id(10),
                ui::button("Load")
                    .message(UiOnlyMessage::Load(Rc::clone(&view_events)))
                    .id(11),
            ])
        })
        .handle_message(|state, message, context| match message {
            UiOnlyMessage::Load(events) => {
                events.borrow_mut().push("load");
                context
                    .business()
                    .background("ui-only-message")
                    .run(|_| (), move |()| UiOnlyMessage::Loaded(events));
            }
            UiOnlyMessage::Loaded(events) => {
                events.borrow_mut().push("loaded");
                state.count += 1;
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 64.0));
    let message = runtime
        .surface()
        .dispatch_widget_output(
            11,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        )
        .expect("button should emit its UI-only message");

    runtime.dispatch_message(message);
    let finished = wait_for_runtime_message(&mut runtime);

    assert_eq!(finished.messages_dispatched, 1);
    assert_eq!(&*events.borrow(), &["load", "loaded"]);
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Count: 1"
    );
}

#[test]
fn handle_message_exposes_ui_update_context_with_clear_app_api_name() {
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
    let text_id = after.root().id();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, text_id, "text").text,
        "Count: 1"
    );
}

#[test]
fn handle_message_is_the_only_context_aware_app_handler_name() {
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
    let text_id = after.root().id();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, text_id, "text").text,
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
    let text_id = after.root().id();
    assert_eq!(
        widget_ref::<TextWidget, _>(&after, text_id, "text").text,
        "Count: 11"
    );
}
