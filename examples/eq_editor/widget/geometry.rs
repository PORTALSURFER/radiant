use radiant::{
    gui::visualization::{HorizontalLogValueAxis, VerticalValueAxis},
    prelude::*,
};

use super::super::model::{MAX_FREQ_HZ, MAX_GAIN_DB, MIN_FREQ_HZ, MIN_GAIN_DB};

pub(crate) fn x_for_freq(plot: Rect, freq_hz: f32) -> f32 {
    freq_axis(plot).x_for_value(freq_hz)
}

pub(crate) fn y_for_gain(plot: Rect, gain_db: f32) -> f32 {
    VerticalValueAxis::new(plot, MIN_GAIN_DB, MAX_GAIN_DB).y_for_value(gain_db)
}

pub(super) fn freq_for_x(plot: Rect, x: f32) -> f32 {
    freq_axis(plot).value_for_x(x)
}

pub(super) fn gain_for_y(plot: Rect, y: f32) -> f32 {
    VerticalValueAxis::new(plot, MIN_GAIN_DB, MAX_GAIN_DB).value_for_y(y)
}

pub(crate) fn freq_for_ratio(ratio: f32) -> f32 {
    freq_axis(Rect::default()).value_for_ratio(ratio)
}

fn freq_axis(plot: Rect) -> HorizontalLogValueAxis {
    HorizontalLogValueAxis::new(plot, MIN_FREQ_HZ, MAX_FREQ_HZ)
}
