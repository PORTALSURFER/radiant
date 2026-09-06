//! Stream aligned appends, then small edits, through persistent shader storage.
//! Run explicitly with `cargo run --example persistent_shader_storage`.

use radiant::prelude::*;
use radiant::runtime::{
    GpuPersistentStorageError, GpuPersistentStoragePatch, GpuPersistentStorageSnapshot,
    GpuPersistentStorageStatus, GpuPersistentStorageTarget, GpuPersistentStorageUpdate,
    RenderCanvasContent, RenderCanvasShaderSurfaceDescriptor, render_canvas,
};
use std::{sync::Arc, time::Duration};

type Admission = std::result::Result<Option<GpuPersistentStorageStatus>, GpuPersistentStorageError>;

struct State {
    descriptor: Arc<RenderCanvasShaderSurfaceDescriptor>,
    revision: u64,
    logical_elements: usize,
    tick: u64,
    error: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            descriptor: Arc::new(
                RenderCanvasShaderSurfaceDescriptor::new("persistent/bars")
                    .wgsl_source(include_str!("persistent_shader_storage/bars.wgsl"))
                    .entry_point("vertex_main")
                    .fragment_entry_point("fragment_main")
                    .storage_identity(101)
                    .storage_revision(1)
                    .storage_bytes([0; 256])
                    .vertex_count(6),
            ),
            revision: 0,
            logical_elements: 1,
            tick: 0,
            error: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Message {
    Tick,
    Admitted(Admission),
}

fn target() -> GpuPersistentStorageTarget {
    GpuPersistentStorageTarget::new(21, 91, 101, 1)
}

fn view(state: &State) -> View<Message> {
    column([
        text("Streaming bars").size(400.0, 28.0),
        render_canvas(
            91,
            1,
            RenderCanvasContent::CustomShader {
                descriptor: Arc::clone(&state.descriptor),
            },
        )
        .id(21)
        .size(640.0, 240.0),
        text(
            state
                .error
                .as_deref()
                .unwrap_or("Appending values, then updating one bar at a time.")
                .to_owned(),
        )
        .wrap()
        .size(640.0, 48.0),
    ])
    .padding(24.0)
    .spacing(12.0)
}

fn handle(state: &mut State, message: Message, context: &mut UiUpdateContext<Message>) {
    match message {
        Message::Admitted(Ok(Some(GpuPersistentStorageStatus::Ready { revision }))) => {
            state.revision = revision;
            context.request_paint_only();
            context.after(Duration::from_millis(32), Message::Tick);
        }
        Message::Admitted(result) => {
            state.error = Some(format!("Storage admission stopped: {result:?}"));
            context.request_repaint();
        }
        Message::Tick => {
            let Some(next) = state.revision.checked_add(1) else {
                return;
            };
            let value = (0.5 + 0.45 * (state.tick as f32 * 0.19).sin()).to_le_bytes();
            let patch = if state.logical_elements < 64 {
                state.logical_elements += 1;
                GpuPersistentStoragePatch::append(target(), state.revision, next, value)
            } else {
                GpuPersistentStoragePatch::replace(
                    target(),
                    state.revision,
                    next,
                    (state.tick as usize % 64) * 4,
                    value,
                )
            };
            state.tick += 1;
            match patch {
                Ok(patch) => context.update_gpu_persistent_storage(
                    GpuPersistentStorageUpdate::Patch(patch),
                    Message::Admitted,
                ),
                Err(error) => context.emit(Message::Admitted(Err(error))),
            }
            context.request_paint_only();
        }
    }
}

fn main() -> radiant::Result {
    radiant::app(State::default())
        .title("Radiant Persistent Storage")
        .size(700, 400)
        .view(view)
        .on_startup(|_, context| {
            let snapshot = GpuPersistentStorageSnapshot::new(target(), 4, 256, 4, 0, [0; 4])
                .expect("fixed valid storage layout");
            context.update_gpu_persistent_storage(
                GpuPersistentStorageUpdate::Snapshot(snapshot),
                Message::Admitted,
            );
        })
        .handle_message(handle)
        .run()
}
