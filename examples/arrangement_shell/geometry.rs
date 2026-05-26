use radiant::gui::visualization::{TimelineAxis, TimelineItemLayout, TimelineLaneLayout};
use radiant::prelude::*;

use super::{TOTAL_BEATS, TRACK_COUNT};

const CLIP_VERTICAL_INSET: f32 = 8.0;

pub(crate) fn x_for_beat(timeline: Rect, beat: f32) -> f32 {
    beat_axis(timeline).x_for_value(beat)
}

pub(crate) fn beat_for_x(timeline: Rect, x: f32) -> f32 {
    beat_axis(timeline).value_for_x(x)
}

pub(crate) fn track_label_rect(timeline: Rect, track: usize) -> Rect {
    let label_bounds = Rect::from_min_max(
        Point::new(timeline.min.x - 64.0, timeline.min.y),
        Point::new(timeline.min.x - 8.0, timeline.max.y),
    );
    track_layout(timeline).lane_label_rect(label_bounds, track)
}

pub(crate) fn beat_range_item_rect(timeline: Rect, track: usize, start: f32, end: f32) -> Rect {
    clip_item_layout(timeline).item_rect(track, start, end)
}

pub(crate) fn track_layout(timeline: Rect) -> TimelineLaneLayout {
    TimelineLaneLayout::even(timeline, TRACK_COUNT)
}

fn clip_item_layout(timeline: Rect) -> TimelineItemLayout {
    TimelineItemLayout::fill_lanes(beat_axis(timeline), track_layout(timeline))
        .with_vertical_inset(CLIP_VERTICAL_INSET)
}

fn beat_axis(timeline: Rect) -> TimelineAxis {
    TimelineAxis::new(timeline, 0.0, TOTAL_BEATS)
}
