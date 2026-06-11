//! Small stateful counter app using Radiant's message-first application model.

use radiant::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
enum CounterMessage {
    Increment,
}

#[derive(Default)]
struct CounterState {
    count: usize,
}

fn main() -> radiant::Result {
    radiant::app(CounterState::default())
        .title("Radiant Counter")
        .size(320, 120)
        .min_size(240, 96)
        .view(|state| {
            column([
                text(format!("Count: {}", state.count)),
                button("Increment")
                    .primary()
                    .message(CounterMessage::Increment),
            ])
            .padding(16.0)
            .spacing(8.0)
        })
        .update(|state, message| match message {
            CounterMessage::Increment => state.count += 1,
        })
        .run()
}
