use super::model::StatusBarState;
use radiant::{
    gui::{
        feedback::{horizontal_progress_activity_rect, horizontal_progress_fill_rect},
        paint::{BorderSides, PaintFrame},
        types::Rgba8,
    },
    layout::Rect,
    theme::ThemeTokens,
};

pub(super) const STATUS_PROGRESS_KEY: u64 = 70;

pub(super) fn progress_frame(
    state: &StatusBarState,
    bounds: Rect,
    theme: &ThemeTokens,
) -> PaintFrame {
    let mut frame = PaintFrame::default();
    let track = bounds.inset(2.0, 4.0, 2.0, 4.0);
    frame.push_rect(track, theme.bg_tertiary);
    if let Some(fill) = horizontal_progress_fill_rect(track, state.aggregate_progress()) {
        frame.push_rect(fill, theme.accent_copper);
    }
    if state.active_count() > 0 || state.animation_running {
        let position = ((state.frame % 120) as f32) / 119.0;
        if let Some(activity) = horizontal_progress_activity_rect(track, position, 0.26, 24.0) {
            frame.push_rect(activity, rgba(255, 184, 132, 188));
        }
    }
    frame.push_border_rects(track, theme.border_emphasis, 1.0, BorderSides::ALL);
    frame
}

const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}
