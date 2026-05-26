use radiant::gui::visualization::{
    TimelineAxis, TimelinePitchItemLayout, TimelinePitchLayout, TimelineValueMarkerLayout,
};
use radiant::prelude::*;

use super::{TOTAL_BEATS, model::PianoRollViewport};

pub(crate) fn row_height_for(rect: Rect, viewport: PianoRollViewport) -> f32 {
    pitch_layout(rect, viewport).row_height()
}

pub(crate) fn x_for_beat_view(grid: Rect, viewport: PianoRollViewport, beat: f32) -> f32 {
    beat_axis(grid, viewport).x_for_value_unclamped(beat)
}

pub(crate) fn beat_for_x_view(grid: Rect, viewport: PianoRollViewport, x: f32) -> f32 {
    beat_axis(grid, viewport)
        .value_for_x(x)
        .clamp(0.0, TOTAL_BEATS)
}

pub(crate) fn beat_range_rect_view(
    grid: Rect,
    viewport: PianoRollViewport,
    start_beat: f32,
    end_beat: f32,
) -> Rect {
    beat_axis(grid, viewport).range_rect_unclamped(start_beat, end_beat)
}

pub(crate) fn pitch_for_y_view(grid: Rect, viewport: PianoRollViewport, y: f32) -> i32 {
    pitch_layout(grid, viewport)
        .pitch_at(Point::new(grid.min.x, y.clamp(grid.min.y, grid.max.y)))
        .unwrap_or(viewport.pitch_start)
}

pub(crate) fn quantize_beat(beat: f32) -> f32 {
    (beat * 4.0).round() / 4.0
}

pub(crate) fn synthetic_velocity(id: u32) -> f32 {
    0.55 + (id % 5) as f32 * 0.08
}

pub(crate) fn is_black_key(pitch: i32) -> bool {
    matches!(pitch.rem_euclid(12), 1 | 3 | 6 | 8 | 10)
}

pub(crate) fn pitch_label(pitch: i32) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!(
        "{}{}",
        NAMES[pitch.rem_euclid(12) as usize],
        pitch.div_euclid(12) - 1
    )
}

pub(crate) fn beat_axis(grid: Rect, viewport: PianoRollViewport) -> TimelineAxis {
    TimelineAxis::new(
        grid,
        viewport.beat_start,
        viewport.beat_start + viewport.visible_beats,
    )
}

pub(crate) fn pitch_layout(grid: Rect, viewport: PianoRollViewport) -> TimelinePitchLayout {
    TimelinePitchLayout::new(grid, viewport.pitch_start, viewport.row_count())
}

pub(crate) fn pitch_item_layout(
    grid: Rect,
    viewport: PianoRollViewport,
) -> TimelinePitchItemLayout {
    TimelinePitchItemLayout::new(beat_axis(grid, viewport), pitch_layout(grid, viewport))
}

pub(crate) fn timeline_value_marker_layout(
    lane: Rect,
    viewport: PianoRollViewport,
    stem_width: f32,
    handle_size: f32,
) -> TimelineValueMarkerLayout {
    TimelineValueMarkerLayout::new(beat_axis(lane, viewport), stem_width, handle_size)
}
