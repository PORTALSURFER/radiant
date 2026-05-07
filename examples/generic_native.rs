//! Standalone native Radiant example using application builders.

use radiant::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
enum DemoMessage {
    ButtonPressed,
    Increment,
}

#[derive(Default)]
struct DemoState {
    count: usize,
}

fn main() -> radiant::Result {
    radiant::app(DemoState::default())
        .title("Radiant Generic Native Example")
        .size(320, 96)
        .min_size(240, 80)
        .view(project_surface)
        .update_command(|state: &mut DemoState, message| match message {
            DemoMessage::ButtonPressed => Command::batch([
                Command::message(DemoMessage::Increment),
                Command::request_repaint(),
            ]),
            DemoMessage::Increment => {
                state.count += 1;
                Command::none()
            }
        })
        .run()
}

fn project_surface(state: &mut DemoState) -> ViewNode<DemoMessage> {
    row([
        text(format!("Generic Radiant count: {}", state.count))
            .id(10)
            .size(180.0, 24.0),
        button("Increment", DemoMessage::ButtonPressed)
            .id(11)
            .size(96.0, 32.0),
    ])
    .id(1)
    .spacing(12.0)
}
