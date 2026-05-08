//! Explicit message routing with a reducer and command.

use radiant::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Message {
    Clicked,
    Increment,
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
                button("Increment").primary().message(Message::Clicked),
            ])
            .padding(16.0)
            .spacing(12.0)
        })
        .update_command(|state, message| match message {
            Message::Clicked => Command::batch([
                Command::message(Message::Increment),
                Command::request_repaint(),
            ]),
            Message::Increment => {
                state.count += 1;
                Command::none()
            }
        })
        .run()
}
