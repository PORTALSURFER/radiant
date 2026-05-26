use radiant::gui::visualization::TimelineAxis;
use radiant::prelude::*;

use super::{TOTAL_BEATS, model::PianoRollViewport};

pub(crate) fn row_height_for(rect: Rect, viewport: PianoRollViewport) -> f32 {
    rect.height() / viewport.row_count() as f32
}

pub(crate) fn x_for_beat_view(grid: Rect, viewport: PianoRollViewport, beat: f32) -> f32 {
    beat_axis(grid, viewport).x_for_value_unclamped(beat)
}

pub(crate) fn beat_for_x_view(grid: Rect, viewport: PianoRollViewport, x: f32) -> f32 {
    beat_axis(grid, viewport)
        .value_for_x(x)
        .clamp(0.0, TOTAL_BEATS)
}

pub(crate) fn y_for_pitch_view(grid: Rect, viewport: PianoRollViewport, pitch: i32) -> f32 {
    let row = viewport.pitch_end() - pitch;
    grid.min.y + row as f32 * row_height_for(grid, viewport)
}

pub(crate) fn pitch_for_y_view(grid: Rect, viewport: PianoRollViewport, y: f32) -> i32 {
    let row = ((y - grid.min.y) / row_height_for(grid, viewport).max(1.0)).floor() as i32;
    (viewport.pitch_end() - row).clamp(viewport.pitch_start, viewport.pitch_end())
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

fn beat_axis(grid: Rect, viewport: PianoRollViewport) -> TimelineAxis {
    TimelineAxis::new(
        grid,
        viewport.beat_start,
        viewport.beat_start + viewport.visible_beats,
    )
}
