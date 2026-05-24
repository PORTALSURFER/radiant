use radiant::prelude::*;

use super::{TOTAL_BEATS, TRACK_COUNT};

pub(crate) fn track_height(timeline: Rect) -> f32 {
    timeline.height() / TRACK_COUNT as f32
}

pub(crate) fn x_for_beat(timeline: Rect, beat: f32) -> f32 {
    timeline.min.x + timeline.width() * (beat / TOTAL_BEATS).clamp(0.0, 1.0)
}

pub(crate) fn beat_for_x(timeline: Rect, x: f32) -> f32 {
    ((x - timeline.min.x) / timeline.width().max(1.0) * TOTAL_BEATS).clamp(0.0, TOTAL_BEATS)
}

pub(crate) fn track_label_rect(timeline: Rect, track: usize) -> Rect {
    let y = timeline.min.y + track as f32 * track_height(timeline);
    Rect::from_min_max(
        Point::new(timeline.min.x - 64.0, y),
        Point::new(timeline.min.x - 8.0, y + track_height(timeline)),
    )
}
