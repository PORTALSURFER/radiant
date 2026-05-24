use radiant::prelude::*;

use super::{LOW_PITCH, PITCH_ROWS, TOTAL_BEATS};

pub(crate) fn row_height(rect: Rect) -> f32 {
    rect.height() / PITCH_ROWS as f32
}

pub(crate) fn x_for_beat(grid: Rect, beat: f32) -> f32 {
    grid.min.x + grid.width() * (beat / TOTAL_BEATS).clamp(0.0, 1.0)
}

pub(crate) fn beat_for_x(grid: Rect, x: f32) -> f32 {
    ((x - grid.min.x) / grid.width().max(1.0) * TOTAL_BEATS).clamp(0.0, TOTAL_BEATS)
}

pub(crate) fn y_for_pitch(grid: Rect, pitch: i32) -> f32 {
    let row = (LOW_PITCH + PITCH_ROWS as i32 - 1 - pitch).clamp(0, PITCH_ROWS as i32 - 1);
    grid.min.y + row as f32 * row_height(grid)
}

pub(crate) fn pitch_for_y(grid: Rect, y: f32) -> i32 {
    let row = ((y - grid.min.y) / row_height(grid).max(1.0)).floor() as i32;
    (LOW_PITCH + PITCH_ROWS as i32 - 1 - row).clamp(LOW_PITCH, LOW_PITCH + PITCH_ROWS as i32 - 1)
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
