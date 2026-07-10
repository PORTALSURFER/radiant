//! Declarative native file-drop target routing.

use radiant::prelude::*;
use std::sync::Arc;

#[derive(Clone, Debug)]
enum Message {
    NativeFileDrop(NativeFileDrop),
}

#[derive(Default)]
struct State {
    status: Arc<str>,
}

fn main() -> radiant::Result {
    radiant::app(State::default())
        .title("Radiant Native File Drop")
        .size(420, 180)
        .min_size(320, 140)
        .view(|state| {
            let status = if state.status.is_empty() {
                TextContent::from("Waiting for native file drop...")
            } else {
                TextContent::from(Arc::clone(&state.status))
            };
            column([
                text("Drop a file onto this panel").height(28.0),
                text(status)
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
                state.status = Arc::from(match drop.phase {
                    NativeFileDropPhase::Hover => drop
                        .path
                        .map(|path| format!("Hovering {}", path.display()))
                        .unwrap_or_else(|| String::from("Hovering a native file")),
                    NativeFileDropPhase::Cancel => String::from("Native file drop canceled"),
                    NativeFileDropPhase::Drop => drop
                        .path
                        .map(|path| format!("Dropped {}", path.display()))
                        .unwrap_or_else(|| String::from("Dropped a native file")),
                });
            }
        })
        .run()
}
