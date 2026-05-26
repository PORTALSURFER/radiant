use radiant::gui::visualization::{TimelineAxis, TimelineLaneLayout};
use radiant::prelude::*;

use super::{TOTAL_BEATS, TRACK_COUNT};

pub(crate) fn x_for_beat(timeline: Rect, beat: f32) -> f32 {
    beat_axis(timeline).x_for_value(beat)
}

pub(crate) fn beat_for_x(timeline: Rect, x: f32) -> f32 {
    beat_axis(timeline).value_for_x(x)
}

pub(crate) fn track_label_rect(timeline: Rect, track: usize) -> Rect {
    let track_rect = track_layout(timeline).lane_rect(track);
    Rect::from_min_max(
        Point::new(timeline.min.x - 64.0, track_rect.min.y),
        Point::new(timeline.min.x - 8.0, track_rect.max.y),
    )
}

pub(crate) fn track_layout(timeline: Rect) -> TimelineLaneLayout {
    TimelineLaneLayout::even(timeline, TRACK_COUNT)
}

fn beat_axis(timeline: Rect) -> TimelineAxis {
    TimelineAxis::new(timeline, 0.0, TOTAL_BEATS)
}
