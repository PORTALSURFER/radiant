use super::{
    model::{StatusBarState, StatusMessage},
    paint::STATUS_PROGRESS_KEY,
};
use radiant::prelude::*;

pub(super) fn project_surface(state: &mut StatusBarState) -> View<StatusMessage> {
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
