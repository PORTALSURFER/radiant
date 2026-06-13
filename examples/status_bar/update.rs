use super::model::{StatusBarState, StatusMessage};
use radiant::prelude::UpdateContext;
use std::{thread, time::Duration};

pub(super) fn update(
    state: &mut StatusBarState,
    message: StatusMessage,
    context: &mut UpdateContext<StatusMessage>,
) {
    match message {
        StatusMessage::ActionPressed => {
            state.record_action();
            context.request_repaint();
        }
        StatusMessage::AutosaveChanged(enabled) => {
            state.set_autosave(enabled);
            context.request_repaint();
        }
        StatusMessage::StartWorker => {
            let id = state.start_worker(format!("preview {}", state.next_worker_id));
            spawn_worker(context, id, Duration::from_millis(850));
            context.request_repaint();
        }
        StatusMessage::StartBatch => {
            for delay in [
                Duration::from_millis(650),
                Duration::from_millis(1_000),
                Duration::from_millis(1_350),
            ] {
                let id = state.start_worker(format!("batch item {}", state.next_worker_id));
                spawn_worker(context, id, delay);
            }
            context.request_repaint();
        }
        StatusMessage::WorkerFinished(id) => {
            state.finish_worker(id);
            context.request_repaint();
        }
        StatusMessage::Frame => {
            state.tick();
            if state.animation_running && state.frame.is_multiple_of(60) {
                state
                    .log
                    .publish("animation", format!("frame {}", state.frame));
            }
            context.request_repaint();
        }
        StatusMessage::ToggleAnimation => {
            state.toggle_animation();
            context.request_repaint();
        }
        StatusMessage::Reset => {
            state.reset();
            context.request_repaint();
        }
    }
}

fn spawn_worker(context: &mut UpdateContext<StatusMessage>, id: u64, duration: Duration) {
    context.business().background("status-bar-worker").run(
        move |_| {
            thread::sleep(duration);
            id
        },
        StatusMessage::WorkerFinished,
    );
}
