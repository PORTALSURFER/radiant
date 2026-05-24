use radiant::prelude::*;

use super::super::model::{MAX_FREQ_HZ, MAX_GAIN_DB, MIN_FREQ_HZ, MIN_GAIN_DB};

pub(crate) fn x_for_freq(plot: Rect, freq_hz: f32) -> f32 {
    plot.min.x + plot.width() * ratio_for_freq(freq_hz)
}

pub(crate) fn y_for_gain(plot: Rect, gain_db: f32) -> f32 {
    let ratio = ((gain_db.clamp(MIN_GAIN_DB, MAX_GAIN_DB) - MIN_GAIN_DB)
        / (MAX_GAIN_DB - MIN_GAIN_DB))
        .clamp(0.0, 1.0);
    plot.max.y - plot.height() * ratio
}

pub(super) fn freq_for_x(plot: Rect, x: f32) -> f32 {
    freq_for_ratio(((x - plot.min.x) / plot.width().max(1.0)).clamp(0.0, 1.0))
}

pub(super) fn gain_for_y(plot: Rect, y: f32) -> f32 {
    let ratio = ((plot.max.y - y) / plot.height().max(1.0)).clamp(0.0, 1.0);
    MIN_GAIN_DB + (MAX_GAIN_DB - MIN_GAIN_DB) * ratio
}

fn ratio_for_freq(freq_hz: f32) -> f32 {
    let min = MIN_FREQ_HZ.log10();
    let max = MAX_FREQ_HZ.log10();
    ((freq_hz.clamp(MIN_FREQ_HZ, MAX_FREQ_HZ).log10() - min) / (max - min)).clamp(0.0, 1.0)
}

pub(crate) fn freq_for_ratio(ratio: f32) -> f32 {
    let min = MIN_FREQ_HZ.log10();
    let max = MAX_FREQ_HZ.log10();
    10.0_f32.powf(min + (max - min) * ratio.clamp(0.0, 1.0))
}
