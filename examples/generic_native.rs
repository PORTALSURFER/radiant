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
        .size(460, 128)
        .min_size(400, 112)
        .view(project_surface)
        .handle_message(|state: &mut DemoState, message, context| match message {
            DemoMessage::ButtonPressed => {
                context.emit(DemoMessage::Increment);
                context.request_repaint();
            }
            DemoMessage::Increment => {
                state.count += 1;
            }
        })
        .run()
}

fn project_surface(state: &mut DemoState) -> View<DemoMessage> {
    row([
        text(format!("Generic Radiant count: {}", state.count))
            .id(10)
            .min_size(220.0, 32.0)
            .preferred_size(260.0, 32.0)
            .fill_width(),
        button("Increment")
            .primary()
            .message(DemoMessage::ButtonPressed)
            .id(11)
            .size(112.0, 40.0),
    ])
    .id(1)
    .padding(20.0)
    .spacing(16.0)
}
