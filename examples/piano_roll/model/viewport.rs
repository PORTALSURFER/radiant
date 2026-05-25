use super::super::{LOW_PITCH, PITCH_ROWS, TOTAL_BEATS};

const MIN_VISIBLE_BEATS: f32 = 4.0;
const MIN_VISIBLE_PITCHES: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PianoRollViewport {
    pub(crate) beat_start: f32,
    pub(crate) visible_beats: f32,
    pub(crate) pitch_start: i32,
    pub(crate) visible_pitches: usize,
}

impl Default for PianoRollViewport {
    fn default() -> Self {
        Self {
            beat_start: 0.0,
            visible_beats: TOTAL_BEATS,
            pitch_start: LOW_PITCH,
            visible_pitches: PITCH_ROWS,
        }
    }
}

impl PianoRollViewport {
    pub(crate) fn beat_end(self) -> f32 {
        (self.beat_start + self.visible_beats).min(TOTAL_BEATS)
    }

    pub(crate) fn pitch_end(self) -> i32 {
        self.pitch_start + self.visible_pitches as i32 - 1
    }

    pub(crate) fn row_count(self) -> usize {
        self.visible_pitches
    }

    pub(super) fn pan(&mut self, beat_delta: f32, pitch_delta: i32) {
        let panned = self.panned(beat_delta, pitch_delta);
        self.beat_start = panned.beat_start;
        self.pitch_start = panned.pitch_start;
    }

    pub(crate) fn panned(self, beat_delta: f32, pitch_delta: i32) -> Self {
        let max_pitch_start = Self::max_pitch_start(self.visible_pitches);
        Self {
            beat_start: (self.beat_start + beat_delta)
                .clamp(0.0, (TOTAL_BEATS - self.visible_beats).max(0.0)),
            pitch_start: (self.pitch_start + pitch_delta).clamp(LOW_PITCH, max_pitch_start),
            ..self
        }
    }

    pub(crate) fn pan_delta_to(self, target: Self) -> (f32, i32) {
        (
            target.beat_start - self.beat_start,
            target.pitch_start - self.pitch_start,
        )
    }

    pub(super) fn zoom_time(&mut self, factor: f32) {
        let center = self.beat_start + self.visible_beats * 0.5;
        self.visible_beats = (self.visible_beats * factor).clamp(MIN_VISIBLE_BEATS, TOTAL_BEATS);
        self.beat_start =
            (center - self.visible_beats * 0.5).clamp(0.0, TOTAL_BEATS - self.visible_beats);
    }

    pub(crate) fn can_zoom_time(self, factor: f32) -> bool {
        let next_visible = (self.visible_beats * factor).clamp(MIN_VISIBLE_BEATS, TOTAL_BEATS);
        (next_visible - self.visible_beats).abs() > f32::EPSILON
    }

    pub(super) fn zoom(&mut self, time_factor: Option<f32>, rows_delta: i32) {
        if let Some(factor) = time_factor {
            self.zoom_time(factor);
        }
        self.zoom_pitch(rows_delta);
    }

    pub(super) fn zoom_pitch(&mut self, rows_delta: i32) {
        let center = self.pitch_start + self.visible_pitches as i32 / 2;
        self.visible_pitches = (self.visible_pitches as i32 + rows_delta)
            .clamp(MIN_VISIBLE_PITCHES as i32, PITCH_ROWS as i32)
            as usize;
        let max_pitch_start = Self::max_pitch_start(self.visible_pitches);
        self.pitch_start =
            (center - self.visible_pitches as i32 / 2).clamp(LOW_PITCH, max_pitch_start);
    }

    fn max_pitch_start(visible_pitches: usize) -> i32 {
        LOW_PITCH + PITCH_ROWS as i32 - visible_pitches as i32
    }
}
