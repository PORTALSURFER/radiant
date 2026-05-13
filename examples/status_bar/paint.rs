use super::model::StatusBarState;
use radiant::{
    gui::{
        feedback::{horizontal_progress_activity_rect, horizontal_progress_fill_rect},
        paint::{BorderSides, FillRect, PaintFrame, Primitive, border_fill_rects},
        types::Rgba8,
    },
    layout::{Point, Rect},
    theme::ThemeTokens,
};

pub(super) const STATUS_PROGRESS_KEY: u64 = 70;

pub(super) fn progress_frame(
    state: &StatusBarState,
    bounds: Rect,
    theme: &ThemeTokens,
) -> PaintFrame {
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
