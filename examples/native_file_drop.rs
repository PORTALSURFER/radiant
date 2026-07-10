//! Declarative native file-drop target routing.

use radiant::prelude::*;

#[derive(Clone, Debug)]
enum Message {
    NativeFileDrop(NativeFileDrop),
}

#[derive(Default)]
struct State {
    status: String,
}

fn main() -> radiant::Result {
    radiant::app(State::default())
        .title("Radiant Native File Drop")
        .size(420, 180)
        .min_size(320, 140)
        .view(|state| {
            column([
                text("Drop a file onto this panel").height(28.0),
                text(if state.status.is_empty() {
                    "Waiting for native file drop..."
                } else {
                    state.status.as_str()
                })
                .size(360.0, 80.0)
                .padding(16.0)
                .accepts_native_file_drop()
                .on_native_file_drop(Message::NativeFileDrop),
            ])
            .padding(16.0)
            .spacing(12.0)
        })
        .update(|state, message| match message {
            Message::NativeFileDrop(drop) => {
                state.status = match drop.phase {
                    NativeFileDropPhase::Hover => drop
                        .path
                        .map(|path| format!("Hovering {}", path.display()))
                        .unwrap_or_else(|| String::from("Hovering a native file")),
                    NativeFileDropPhase::Cancel => String::from("Native file drop canceled"),
                    NativeFileDropPhase::Drop => drop
                        .path
                        .map(|path| format!("Dropped {}", path.display()))
                        .unwrap_or_else(|| String::from("Dropped a native file")),
                };
            }
        })
        .run()
}
