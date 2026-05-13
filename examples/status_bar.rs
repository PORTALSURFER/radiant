//! Bottom status bar with one-line updates and progress-aware background work.

use radiant::prelude::*;
use radiant::{
    gui::{
        feedback::{horizontal_progress_activity_rect, horizontal_progress_fill_rect},
        paint::{BorderSides, FillRect, PaintFrame, Primitive, border_fill_rects},
        types::Rgba8,
    },
    layout::{Point, Rect},
    theme::ThemeTokens,
};
use std::{thread, time::Duration};

const STATUS_PROGRESS_KEY: u64 = 70;
const LOG_LIMIT: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StatusMessage {
    ActionPressed,
    AutosaveChanged(bool),
    StartWorker,
    StartBatch,
    WorkerFinished(u64),
    Frame,
    ToggleAnimation,
    Reset,
}

#[derive(Clone, Debug, PartialEq)]
struct StatusBarState {
    autosave: bool,
    action_count: u32,
    jobs: Vec<WorkItem>,
    completed_workers: u32,
    next_worker_id: u64,
    frame: u64,
    animation_running: bool,
    log: StatusLineLog,
}

#[derive(Clone, Debug, PartialEq)]
struct WorkItem {
    id: u64,
    label: String,
    progress: f32,
    done: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StatusLineLog {
    entries: Vec<StatusLineEntry>,
    limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StatusLineEntry {
    source: String,
    message: String,
}

impl StatusLineLog {
    fn new(limit: usize) -> Self {
        Self {
            entries: vec![StatusLineEntry::new("system", "Ready")],
            limit,
        }
    }

    fn publish(&mut self, source: impl Into<String>, message: impl Into<String>) {
        self.entries
            .push(StatusLineEntry::new(source.into(), message.into()));
        let overflow = self.entries.len().saturating_sub(self.limit);
        if overflow > 0 {
            self.entries.drain(0..overflow);
        }
    }

    fn latest(&self) -> String {
        self.entries
            .last()
            .map(StatusLineEntry::line)
            .unwrap_or_else(|| "system: Ready".to_string())
    }

    fn recent_lines(&self) -> Vec<String> {
        self.entries
            .iter()
            .rev()
            .map(StatusLineEntry::line)
            .collect()
    }
}

impl StatusLineEntry {
    fn new(source: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            message: message.into(),
        }
    }

    fn line(&self) -> String {
        format!("{}: {}", self.source, self.message)
    }
}

impl Default for StatusBarState {
    fn default() -> Self {
        Self {
            autosave: true,
            action_count: 0,
            jobs: Vec::new(),
            completed_workers: 0,
            next_worker_id: 1,
            frame: 0,
            animation_running: false,
            log: StatusLineLog::new(LOG_LIMIT),
        }
    }
}

impl StatusBarState {
    fn record_action(&mut self) {
        self.action_count += 1;
        self.log.publish(
            "action",
            format!("button pressed {} time(s)", self.action_count),
        );
    }

    fn set_autosave(&mut self, enabled: bool) {
        self.autosave = enabled;
        let message = if enabled {
            "autosave enabled"
        } else {
            "autosave paused"
        };
        self.log.publish("autosave", message);
    }

    fn start_worker(&mut self, label: impl Into<String>) -> u64 {
        let id = self.next_worker_id;
        self.next_worker_id += 1;
        let label = label.into();
        self.jobs.push(WorkItem {
            id,
            label: label.clone(),
            progress: 0.0,
            done: false,
        });
        self.log.publish("worker", format!("{label} started"));
        id
    }

    fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        for job in self.jobs.iter_mut().filter(|job| !job.done) {
            job.progress = (job.progress + 0.018).min(0.92);
        }
    }

    fn finish_worker(&mut self, id: u64) {
        if let Some(job) = self.jobs.iter_mut().find(|job| job.id == id) {
            job.progress = 1.0;
            job.done = true;
            self.completed_workers += 1;
            self.log
                .publish("worker", format!("{} finished", job.label));
        }
    }

    fn active_count(&self) -> usize {
        self.jobs.iter().filter(|job| !job.done).count()
    }

    fn aggregate_progress(&self) -> f32 {
        if self.jobs.is_empty() {
            return 0.0;
        }
        self.jobs.iter().map(|job| job.progress).sum::<f32>() / self.jobs.len() as f32
    }

    fn visual_revision(&self) -> u64 {
        let progress = (self.aggregate_progress() * 10_000.0).round() as u64;
        (self.frame << 16) ^ progress ^ (self.jobs.len() as u64)
    }

    fn toggle_animation(&mut self) {
        self.animation_running = !self.animation_running;
        self.log.publish(
            "animation",
            if self.animation_running {
                "started"
            } else {
                "stopped"
            },
        );
    }

    fn reset(&mut self) {
        *self = Self::default();
        self.log.publish("system", "status reset");
    }

    fn status_segments(&self) -> StatusSegments {
        StatusSegments::primary(self.log.latest())
            .with_center(format!(
                "Autosave {} | actions {} | workers {}",
                if self.autosave { "on" } else { "off" },
                self.action_count,
                self.completed_workers
            ))
            .with_right(if self.active_count() > 0 {
                "Busy"
            } else if self.animation_running {
                "Animating"
            } else {
                "Idle"
            })
    }
}

fn main() -> radiant::Result {
    radiant::app(StatusBarState::default())
        .title("Radiant Status Bar")
        .size(660, 340)
        .min_size(520, 260)
        .view(project_surface)
        .animation(|state| state.active_count() > 0 || state.animation_running)
        .on_frame(|| StatusMessage::Frame)
        .retained_painter(
            STATUS_PROGRESS_KEY,
            |state, _descriptor, rect, _viewport| {
                Some(progress_frame(state, rect, &ThemeTokens::default()))
            },
        )
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
        text("Use the controls below to send one-line updates into the persistent status strip.")
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
            button("Start job")
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
            button("Start batch")
                .subtle()
                .message(StatusMessage::StartBatch)
                .id(14)
                .width(112.0)
                .height(34.0),
            button(if state.animation_running {
                "Stop animation"
            } else {
                "Start animation"
            })
            .subtle()
            .message(StatusMessage::ToggleAnimation)
            .id(15)
            .width(132.0)
            .height(34.0),
        ])
        .spacing(10.0)
        .fill_width(),
        text(worker_hint(state)).height(28.0).fill_width(),
        log_panel(state).fill_height(),
    ])
    .padding(16.0)
    .spacing(12.0)
    .fill_width()
}

fn status_bar(state: &StatusBarState) -> View<StatusMessage> {
    let segments = state.status_segments();
    row([
        text(segments.left).truncate().fill_width(),
        progress_canvas(state).width(136.0),
        text(segments.center)
            .align_text(TextAlign::Center)
            .width(260.0),
        text(segments.right)
            .align_text(TextAlign::Right)
            .width(76.0),
    ])
    .subtle()
    .padding_x(12.0)
    .padding_y(6.0)
    .height(34.0)
    .fill_width()
}

fn worker_hint(state: &StatusBarState) -> String {
    if state.active_count() > 0 {
        format!(
            "{} background job(s) running; progress and completion update the status line.",
            state.active_count()
        )
    } else {
        "Start jobs, toggle autosave, or run animation to publish one-line status updates."
            .to_string()
    }
}

fn progress_canvas(state: &StatusBarState) -> View<StatusMessage> {
    retained_canvas(STATUS_PROGRESS_KEY)
        .revision(state.visual_revision())
        .dirty_mask(u64::from(
            state.active_count() > 0 || state.animation_running,
        ))
        .volatile(state.active_count() > 0 || state.animation_running)
        .view()
        .height(18.0)
}

fn log_panel(state: &StatusBarState) -> View<StatusMessage> {
    column(
        state
            .log
            .recent_lines()
            .into_iter()
            .map(|line| text(line).truncate().height(22.0).fill_width()),
    )
    .spacing(2.0)
    .fill_width()
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
    context.spawn(
        "status-bar-worker",
        move || {
            thread::sleep(duration);
            id
        },
        StatusMessage::WorkerFinished,
    );
}

fn progress_frame(state: &StatusBarState, bounds: Rect, theme: &ThemeTokens) -> PaintFrame {
    let mut frame = PaintFrame::default();
    let track = Rect::from_min_max(
        Point::new(bounds.min.x + 2.0, bounds.min.y + 4.0),
        Point::new(bounds.max.x - 2.0, bounds.max.y - 4.0),
    );
    push_rect(&mut frame, track, theme.bg_tertiary);
    if let Some(fill) = horizontal_progress_fill_rect(track, state.aggregate_progress()) {
        push_rect(&mut frame, fill, theme.accent_copper);
    }
    if state.active_count() > 0 || state.animation_running {
        let position = ((state.frame % 120) as f32) / 119.0;
        if let Some(activity) = horizontal_progress_activity_rect(track, position, 0.26, 24.0) {
            push_rect(&mut frame, activity, rgba(255, 184, 132, 188));
        }
    }
    frame.primitives.extend(
        border_fill_rects(track, theme.border_emphasis, 1.0, BorderSides::ALL)
            .into_iter()
            .map(Primitive::Rect),
    );
    frame
}

fn push_rect(frame: &mut PaintFrame, rect: Rect, color: Rgba8) {
    frame
        .primitives
        .push(Primitive::Rect(FillRect { rect, color }));
}

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::{prelude::IntoView, theme::ThemeTokens};

    #[test]
    fn status_line_log_keeps_latest_bounded_message() {
        let mut log = StatusLineLog::new(3);

        log.publish("button", "pressed");
        log.publish("worker", "started");
        log.publish("animation", "stopped");

        assert_eq!(log.entries.len(), 3);
        assert_eq!(log.latest(), "animation: stopped");
        assert_eq!(
            log.recent_lines(),
            vec![
                "animation: stopped".to_string(),
                "worker: started".to_string(),
                "button: pressed".to_string()
            ]
        );
    }

    #[test]
    fn status_bar_state_tracks_actions_toggles_workers_and_progress() {
        let mut state = StatusBarState::default();

        state.record_action();
        state.set_autosave(false);
        let worker = state.start_worker("preview");
        state.tick();
        assert!(state.aggregate_progress() > 0.0);
        state.finish_worker(worker);
        state.toggle_animation();

        assert_eq!(state.action_count, 1);
        assert!(!state.autosave);
        assert_eq!(state.active_count(), 0);
        assert_eq!(state.completed_workers, 1);
        assert!(state.animation_running);
        assert_eq!(state.log.latest(), "animation: started");
        assert_eq!(
            state.status_segments().center,
            "Autosave off | actions 1 | workers 1"
        );
    }

    #[test]
    fn status_bar_allows_concurrent_worker_progress() {
        let mut state = StatusBarState::default();
        let first = state.start_worker("first");
        let second = state.start_worker("second");

        state.tick();
        state.finish_worker(first);

        assert_eq!(state.active_count(), 1);
        assert_eq!(state.completed_workers, 1);
        assert!(state.aggregate_progress() < 1.0);

        state.finish_worker(second);
        assert_eq!(state.active_count(), 0);
        assert_eq!(state.aggregate_progress(), 1.0);
    }

    #[test]
    fn status_bar_projects_controls_and_status_strip() {
        let mut state = StatusBarState::default();
        let surface = project_surface(&mut state).into_surface();

        assert!(surface.find_widget(10).is_some());
        assert!(surface.find_widget(11).is_some());
        assert!(surface.find_widget(12).is_some());
        assert!(surface.find_widget(13).is_some());
        assert!(surface.find_widget(14).is_some());
        assert!(surface.find_widget(15).is_some());
        assert!(surface.keyboard_focus_order().len() >= 6);
    }

    #[test]
    fn status_bar_paints_progress_inside_status_strip() {
        let mut state = StatusBarState::default();
        state.start_worker("preview");
        state.tick();

        let frame = progress_frame(
            &state,
            Rect::from_min_size(
                Point::new(0.0, 0.0),
                radiant::layout::Vector2::new(160.0, 18.0),
            ),
            &ThemeTokens::default(),
        );

        assert!(frame.primitives.len() >= 4);
    }
}
