//! Small stateful counter app built with Radiant application builders.

use radiant::prelude::*;

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
                    .on_click(|state: &mut CounterState| state.count += 1),
            ])
            .padding(16.0)
            .spacing(8.0)
        })
        .run()
}
