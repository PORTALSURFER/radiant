//! Bottom status bar with actions, toggles, and background work.

use radiant::prelude::*;
use std::{thread, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StatusMessage {
    ActionPressed,
    AutosaveChanged(bool),
    StartWorker,
    WorkerFinished(u64),
    Reset,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StatusBarState {
    autosave: bool,
    action_count: u32,
    worker_running: bool,
    completed_workers: u32,
    next_worker_id: u64,
    status: String,
}

impl Default for StatusBarState {
    fn default() -> Self {
        Self {
            autosave: true,
            action_count: 0,
            worker_running: false,
            completed_workers: 0,
            next_worker_id: 1,
            status: String::from("Ready"),
        }
    }
}

impl StatusBarState {
    fn record_action(&mut self) {
        self.action_count += 1;
        self.status = format!("Action button pressed {} time(s)", self.action_count);
    }

    fn set_autosave(&mut self, enabled: bool) {
        self.autosave = enabled;
        self.status = if enabled {
            String::from("Autosave enabled")
        } else {
            String::from("Autosave paused")
        };
    }

    fn start_worker(&mut self) -> Option<u64> {
        if self.worker_running {
            self.status = String::from("Background worker already running");
            return None;
        }
        let id = self.next_worker_id;
        self.next_worker_id += 1;
        self.worker_running = true;
        self.status = format!("Background worker {id} running");
        Some(id)
    }

    fn finish_worker(&mut self, id: u64) {
        self.worker_running = false;
        self.completed_workers += 1;
        self.status = format!("Background worker {id} finished");
    }

    fn reset(&mut self) {
        *self = Self::default();
        self.status = String::from("Status reset");
    }

    fn status_segments(&self) -> StatusSegments {
        StatusSegments::primary(self.status.clone())
            .with_center(format!(
                "Autosave {} | actions {} | workers {}",
                if self.autosave { "on" } else { "off" },
                self.action_count,
                self.completed_workers
            ))
            .with_right(if self.worker_running { "Busy" } else { "Idle" })
    }
}

fn main() -> radiant::Result {
    radiant::app(StatusBarState::default())
        .title("Radiant Status Bar")
        .size(560, 220)
        .min_size(440, 170)
        .view(project_surface)
        .update_with(update)
        .run()
}

fn project_surface(state: &mut StatusBarState) -> View<StatusMessage> {
    column([workspace_panel(state).fill_height(), status_bar(state)])
        .fill()
        .spacing(0.0)
}

fn workspace_panel(state: &StatusBarState) -> View<StatusMessage> {
    column([
        text("Status Bar Example").height(30.0).fill_width(),
        text("Use the controls below to update the persistent status strip.")
            .wrap()
            .height(42.0)
            .fill_width(),
        row([
            button("Trigger action")
                .primary()
                .message(StatusMessage::ActionPressed)
                .id(10)
                .width(132.0)
                .height(34.0),
            toggle("Autosave", state.autosave)
                .message(StatusMessage::AutosaveChanged)
                .id(11)
                .width(118.0)
                .height(34.0),
            button(if state.worker_running {
                "Working..."
            } else {
                "Start worker"
            })
            .subtle()
            .message(StatusMessage::StartWorker)
            .id(12)
            .width(118.0)
            .height(34.0),
            button("Reset")
                .subtle()
                .message(StatusMessage::Reset)
                .id(13)
                .width(84.0)
                .height(34.0),
        ])
        .spacing(10.0)
        .fill_width(),
        text(worker_hint(state)).height(28.0).fill_width(),
    ])
    .padding(16.0)
    .spacing(12.0)
    .fill_width()
}

fn status_bar(state: &StatusBarState) -> View<StatusMessage> {
    let segments = state.status_segments();
    row([
        text(segments.left).truncate().fill_width(),
        text(segments.center)
            .align_text(TextAlign::Center)
            .width(230.0),
        text(segments.right)
            .align_text(TextAlign::Right)
            .width(64.0),
    ])
    .subtle()
    .padding_x(12.0)
    .padding_y(4.0)
    .height(30.0)
    .fill_width()
}

fn worker_hint(state: &StatusBarState) -> String {
    if state.worker_running {
        "A background worker is running; completion will update the status bar.".to_string()
    } else {
        "Start a background worker to route async completion back into app state.".to_string()
    }
}

fn update(
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
            if let Some(id) = state.start_worker() {
                context.spawn(
                    "status-bar-worker",
                    move || {
                        thread::sleep(Duration::from_millis(700));
                        id
                    },
                    StatusMessage::WorkerFinished,
                );
            }
            context.request_repaint();
        }
        StatusMessage::WorkerFinished(id) => {
            state.finish_worker(id);
            context.request_repaint();
        }
        StatusMessage::Reset => {
            state.reset();
            context.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::prelude::IntoView;

    #[test]
    fn status_bar_state_tracks_actions_toggles_and_workers() {
        let mut state = StatusBarState::default();

        state.record_action();
        state.set_autosave(false);
        let worker = state.start_worker().expect("worker should start");
        state.finish_worker(worker);

        assert_eq!(state.action_count, 1);
        assert!(!state.autosave);
        assert!(!state.worker_running);
        assert_eq!(state.completed_workers, 1);
        assert_eq!(
            state.status_segments().center,
            "Autosave off | actions 1 | workers 1"
        );
    }

    #[test]
    fn status_bar_rejects_second_running_worker() {
        let mut state = StatusBarState::default();
        assert_eq!(state.start_worker(), Some(1));

        assert_eq!(state.start_worker(), None);

        assert!(state.worker_running);
        assert!(state.status.contains("already running"));
    }

    #[test]
    fn status_bar_projects_controls_and_status_strip() {
        let mut state = StatusBarState::default();
        let surface = project_surface(&mut state).into_surface();

        assert!(surface.find_widget(10).is_some());
        assert!(surface.find_widget(11).is_some());
        assert!(surface.find_widget(12).is_some());
        assert!(surface.find_widget(13).is_some());
        assert!(surface.keyboard_focus_order().len() >= 4);
    }
}
