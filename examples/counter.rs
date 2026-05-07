//! Small stateful counter app built on the beginner-facing Radiant API.

use radiant::prelude::*;

#[derive(Clone, Debug)]
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
                button("Increment", CounterMessage::Increment),
            ])
            .spacing(8.0)
        })
        .update(|state, message| match message {
            CounterMessage::Increment => state.count += 1,
        })
        .run()
}
