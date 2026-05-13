use super::*;
use radiant::runtime::{
    Command, SurfaceRuntime, declarative_command_runtime_bridge,
    declarative_owned_command_runtime_bridge,
};

#[test]
fn declarative_command_bridge_supports_command_update_flow() {
    let bridge = declarative_command_runtime_bridge(
        DemoState::default(),
        project_command_surface,
        |state: &mut DemoState, message| match message {
            CommandDemoMessage::Start => Command::batch([
                Command::message(CommandDemoMessage::Rename(String::from("Closure"))),
                Command::message(CommandDemoMessage::Increment),
                Command::request_repaint(),
            ]),
            CommandDemoMessage::Increment => {
                state.count += 1;
                Command::none()
            }
            CommandDemoMessage::Rename(name) => {
                state.name = name;
                Command::none()
            }
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let outcome = runtime.dispatch_message(CommandDemoMessage::Start);

    assert_eq!(outcome.messages_dispatched, 3);
    assert!(outcome.repaint_requested);
    assert_eq!(runtime.bridge().state().count, 1);
    assert_eq!(runtime.bridge().state().name, "Closure");
}

#[test]
fn owned_command_bridge_supports_runtime_command_flow() {
    let bridge = declarative_owned_command_runtime_bridge(
        DemoState::default(),
        project_owned_command_surface,
        |state: &mut DemoState, message| match message {
            CommandDemoMessage::Start => Command::batch([
                Command::message(CommandDemoMessage::Rename(String::from("Owned"))),
                Command::message(CommandDemoMessage::Increment),
                Command::request_repaint(),
            ]),
            CommandDemoMessage::Increment => {
                state.count += 1;
                Command::none()
            }
            CommandDemoMessage::Rename(name) => {
                state.name = name;
                Command::none()
            }
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let outcome = runtime.dispatch_message(CommandDemoMessage::Start);

    assert_eq!(outcome.messages_dispatched, 3);
    assert!(outcome.repaint_requested);
    assert_eq!(runtime.bridge().state().count, 1);
    assert_eq!(runtime.bridge().state().name, "Owned");
}

enum CommandDemoMessage {
    Start,
    Increment,
    Rename(String),
}

fn project_command_surface(state: &mut DemoState) -> Arc<UiSurface<CommandDemoMessage>> {
    let title = TextWidget::new(
        10,
        format!("{} ({})", display_name(state), state.count),
        WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
    );
    let button = ButtonWidget::new(11, "Run", WidgetSizing::fixed(Vector2::new(96.0, 28.0)));
    let input = TextInputWidget::new(
        12,
        state.name.clone(),
        WidgetSizing::new(Vector2::new(120.0, 28.0), Vector2::new(180.0, 28.0)),
    );

    Arc::new(UiSurface::new(SurfaceNode::row(
        1,
        8.0,
        vec![
            SurfaceChild::fill(SurfaceNode::static_widget(title)),
            SurfaceChild::fill(SurfaceNode::widget(
                button,
                WidgetMessageMapper::button(|_| CommandDemoMessage::Start),
            )),
            SurfaceChild::fill(SurfaceNode::widget(input, WidgetMessageMapper::none())),
        ],
    )))
}

fn project_owned_command_surface(state: &mut DemoState) -> UiSurface<CommandDemoMessage> {
    Arc::unwrap_or_clone(project_command_surface(state))
}
