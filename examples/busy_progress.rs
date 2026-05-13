//! Busy progress bar backed by slow background work.

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

const PROGRESS_CANVAS_KEY: u64 = 70;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BusyMessage {
    StartOne,
    StartBatch,
    Frame,
    Finished(u64),
    Reset,
}

#[derive(Clone, Debug)]
struct WorkItem {
    id: u64,
    label: String,
    progress: f32,
    done: bool,
}

#[derive(Clone, Debug)]
struct BusyState {
    jobs: Vec<WorkItem>,
    next_id: u64,
    frame: u64,
}

impl Default for BusyState {
    fn default() -> Self {
        Self {
            jobs: Vec::new(),
            next_id: 1,
            frame: 0,
        }
    }
}

impl BusyState {
    fn start_job(&mut self, label: impl Into<String>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.jobs.push(WorkItem {
            id,
            label: label.into(),
            progress: 0.0,
            done: false,
        });
        id
    }

    fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        for job in self.jobs.iter_mut().filter(|job| !job.done) {
            job.progress = (job.progress + 0.018).min(0.92);
        }
    }

    fn finish(&mut self, id: u64) {
        if let Some(job) = self.jobs.iter_mut().find(|job| job.id == id) {
            job.progress = 1.0;
            job.done = true;
        }
    }

    fn reset(&mut self) {
        self.jobs.clear();
    }

    fn active_count(&self) -> usize {
        self.jobs.iter().filter(|job| !job.done).count()
    }

    fn completed_count(&self) -> usize {
        self.jobs.iter().filter(|job| job.done).count()
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
}

fn main() -> radiant::Result {
    radiant::app(BusyState::default())
        .title("Radiant Busy Progress")
        .size(520, 240)
        .min_size(400, 190)
        .view(project_surface)
        .animation(|state| state.active_count() > 0)
        .on_frame(|| BusyMessage::Frame)
        .retained_painter(
            PROGRESS_CANVAS_KEY,
            |state, _descriptor, rect, _viewport| {
                Some(progress_frame(state, rect, &ThemeTokens::default()))
            },
        )
        .update_with(update)
        .run()
}

fn project_surface(state: &mut BusyState) -> View<BusyMessage> {
    column([
        text("Busy Progress").height(28.0).fill_width(),
        progress_canvas(state),
        text(status_text(state)).height(28.0).fill_width(),
        row([
            button("Start job")
                .primary()
                .message(BusyMessage::StartOne)
                .width(112.0)
                .height(32.0),
            button("Start batch")
                .subtle()
                .message(BusyMessage::StartBatch)
                .width(112.0)
                .height(32.0),
            button("Reset")
                .subtle()
                .message(BusyMessage::Reset)
                .width(88.0)
                .height(32.0),
        ])
        .spacing(10.0)
        .fill_width(),
        job_list(state).fill_height(),
    ])
    .padding(16.0)
    .spacing(10.0)
    .fill()
}

fn progress_canvas(state: &BusyState) -> View<BusyMessage> {
    retained_canvas(PROGRESS_CANVAS_KEY)
        .revision(state.visual_revision())
        .dirty_mask(u64::from(state.active_count() > 0))
        .volatile(state.active_count() > 0)
        .view()
        .height(36.0)
        .fill_width()
}

fn job_list(state: &BusyState) -> View<BusyMessage> {
    if state.jobs.is_empty() {
        text("No active work. Start a job or batch to queue background tasks.")
            .wrap()
            .fill_width()
            .height(48.0)
    } else {
        list(state.jobs.iter().cloned(), |job| {
            list_row_id(
                10_000 + job.id,
                [
                    text(job.label).fill_width(),
                    text(if job.done { "Done" } else { "Working" })
                        .height(28.0)
                        .width(88.0),
                    text(format!("{:>3}%", (job.progress * 100.0).round() as u32))
                        .align_text(TextAlign::Right)
                        .height(28.0)
                        .width(58.0),
                ],
            )
        })
        .fill_height()
    }
}

fn update(state: &mut BusyState, message: BusyMessage, context: &mut UpdateContext<BusyMessage>) {
    match message {
        BusyMessage::StartOne => {
            let id = state.start_job(format!("Render preview {}", state.next_id));
            spawn_slow_job(context, id, Duration::from_millis(850));
            context.request_repaint();
        }
        BusyMessage::StartBatch => {
            for delay in [
                Duration::from_millis(650),
                Duration::from_millis(1_000),
                Duration::from_millis(1_350),
            ] {
                let id = state.start_job(format!("Analyze source {}", state.next_id));
                spawn_slow_job(context, id, delay);
            }
            context.request_repaint();
        }
        BusyMessage::Frame => {
            state.tick();
            context.request_repaint();
        }
        BusyMessage::Finished(id) => {
            state.finish(id);
            context.request_repaint();
        }
        BusyMessage::Reset => {
            state.reset();
            context.request_repaint();
        }
    }
}

fn spawn_slow_job(context: &mut UpdateContext<BusyMessage>, id: u64, duration: Duration) {
    context.spawn(
        "busy-progress-job",
        move || {
            thread::sleep(duration);
            id
        },
        BusyMessage::Finished,
    );
}

fn status_text(state: &BusyState) -> String {
    if state.jobs.is_empty() {
        return "Idle".to_string();
    }
    format!(
        "{} of {} finished, {} running",
        state.completed_count(),
        state.jobs.len(),
        state.active_count()
    )
}

fn progress_frame(state: &BusyState, bounds: Rect, theme: &ThemeTokens) -> PaintFrame {
    let mut frame = PaintFrame::default();
    let track = Rect::from_min_max(
        Point::new(bounds.min.x + 4.0, bounds.min.y + 10.0),
        Point::new(bounds.max.x - 4.0, bounds.max.y - 10.0),
    );
    push_rect(&mut frame, track, theme.bg_tertiary);
    if let Some(fill) = horizontal_progress_fill_rect(track, state.aggregate_progress()) {
        push_rect(&mut frame, fill, theme.accent_copper);
    }
    if state.active_count() > 0 {
        let position = ((state.frame % 120) as f32) / 119.0;
        if let Some(activity) = horizontal_progress_activity_rect(track, position, 0.22, 32.0) {
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
    fn busy_progress_tracks_aggregate_work() {
        let mut state = BusyState::default();
        let first = state.start_job("First");
        let second = state.start_job("Second");

        state.tick();
        assert!(state.aggregate_progress() > 0.0);
        assert_eq!(state.active_count(), 2);

        state.finish(first);
        assert_eq!(state.completed_count(), 1);
        assert_eq!(state.active_count(), 1);

        state.finish(second);
        assert_eq!(state.aggregate_progress(), 1.0);
        assert_eq!(state.active_count(), 0);
    }

    #[test]
    fn busy_progress_paints_track_fill_and_activity() {
        let mut state = BusyState::default();
        state.start_job("Preview");
        state.tick();

        let frame = progress_frame(
            &state,
            Rect::from_min_size(
                Point::new(0.0, 0.0),
                radiant::layout::Vector2::new(240.0, 36.0),
            ),
            &ThemeTokens::default(),
        );

        assert!(frame.primitives.len() >= 4);
    }

    #[test]
    fn busy_progress_projects_controls_and_rows() {
        let mut state = BusyState::default();
        state.start_job("Preview");
        let surface = project_surface(&mut state).into_surface();

        assert!(surface.keyboard_focus_order().len() >= 3);
    }
}
