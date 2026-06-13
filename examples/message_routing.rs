//! Message routing with UpdateContext follow-up.

use radiant::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Message {
    IncrementRequested,
    ApplyIncrement,
}

#[derive(Default)]
struct State {
    count: usize,
}

fn main() -> radiant::Result {
    radiant::app(State::default())
        .title("Radiant Message Routing")
        .size(360, 140)
        .min_size(280, 120)
        .view(|state| {
            row([
                text(format!("Count: {}", state.count))
                    .fill_width()
                    .height(32.0),
                button("Increment")
                    .primary()
                    .message(Message::IncrementRequested),
            ])
            .padding(16.0)
            .spacing(12.0)
        })
        .handle_message(|state, message, context| match message {
            Message::IncrementRequested => {
                context.emit(Message::ApplyIncrement);
                context.request_repaint();
            }
            Message::ApplyIncrement => {
                state.count += 1;
            }
        })
        .run()
}
