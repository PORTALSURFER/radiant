//! Runtime command flattening scenario.

use radiant::runtime::Command;
use std::hint::black_box;

pub(super) fn command_flattening_512() -> impl FnMut() {
    bench_command_flattening_512
}

fn bench_command_flattening_512() {
    let command = Command::batch((0..512).map(|index| {
        if index % 8 == 0 {
            Command::batch([
                Command::message(index),
                Command::request_repaint(),
                Command::message(index + 10_000),
            ])
        } else if index % 5 == 0 {
            Command::request_paint_only()
        } else {
            Command::message(index)
        }
    }));
    let messages = command.into_messages();
    assert_eq!(messages.len(), 486);
    black_box(messages);
}
