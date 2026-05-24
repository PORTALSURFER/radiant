use super::super::model::{EqBand, EqBandKind, MAX_GAIN_DB, MIN_GAIN_DB};

pub(crate) fn response_gain_db(bands: &[EqBand], freq_hz: f32) -> f32 {
    bands
        .iter()
        .filter(|band| band.enabled)
        .map(|band| band_visual_gain(*band, freq_hz))
        .sum::<f32>()
        .clamp(MIN_GAIN_DB, MAX_GAIN_DB)
}

fn band_visual_gain(band: EqBand, freq_hz: f32) -> f32 {
    let octave_delta = (freq_hz / band.freq_hz).max(0.001).log2();
    match band.kind {
        EqBandKind::Bell => bell_gain(band, octave_delta),
        EqBandKind::HighPass => -18.0 / (1.0 + ((freq_hz / band.freq_hz).max(0.001)).powf(5.0)),
        EqBandKind::HighShelf => high_shelf_gain(band, octave_delta),
    }
}

fn bell_gain(band: EqBand, octave_delta: f32) -> f32 {
    let width = (1.1 / band.q.max(0.2)).max(0.14);
    band.gain_db * (-(octave_delta * octave_delta) / (2.0 * width * width)).exp()
}

fn high_shelf_gain(band: EqBand, octave_delta: f32) -> f32 {
    let blend = 1.0 / (1.0 + (-octave_delta * 3.0).exp());
    band.gain_db * blend
}
