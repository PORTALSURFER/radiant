//! Native devtools overlay example.

use radiant::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
enum DemoMessage {
    Increment,
}

#[derive(Default)]
struct DemoState {
    count: usize,
}

fn main() -> radiant::Result {
    radiant::app(DemoState::default())
        .title("Radiant Devtools Overlay")
        .size(420, 160)
        .options(NativeRunOptions::default().devtools_overlay_enabled(true))
        .view(|state| {
            column([
                text(format!("Count: {}", state.count)),
                button("Increment")
                    .primary()
                    .message(DemoMessage::Increment),
            ])
            .padding(16.0)
            .spacing(8.0)
        })
        .update(|state, message| match message {
            DemoMessage::Increment => state.count += 1,
        })
        .run()
}
