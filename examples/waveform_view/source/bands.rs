use super::{BAND_COUNT, WaveformBand};
use std::sync::Arc;

pub(super) fn split_frequency_bands(
    samples: &[f32],
    sample_rate: u32,
) -> [WaveformBand; BAND_COUNT] {
    let low_160 = lowpass(samples, sample_rate, 160.0);
    let low_700 = lowpass(samples, sample_rate, 700.0);
    let low_2k8 = lowpass(samples, sample_rate, 2_800.0);
    let low = low_160.clone();
    let low_mid = subtract_samples(&low_700, &low_160);
    let mid = subtract_samples(&low_2k8, &low_700);
    let high = subtract_samples(samples, &low_2k8);
    [
        WaveformBand::new(normalized_band(low, 1.45)),
        WaveformBand::new(normalized_band(low_mid, 1.25)),
        WaveformBand::new(normalized_band(mid, 1.1)),
        WaveformBand::new(normalized_band(high, 0.95)),
    ]
}

pub(crate) fn interleaved_band_samples(bands: &[WaveformBand; BAND_COUNT]) -> Arc<[f32]> {
    let frames = bands
        .first()
        .map(|band| band.samples.len())
        .unwrap_or_default();
    let mut samples = Vec::with_capacity(frames.saturating_mul(BAND_COUNT));
    for frame in 0..frames {
        for band in bands {
            samples.push(band.samples.get(frame).copied().unwrap_or_default());
        }
    }
    samples.into()
}

pub(crate) fn downmix_to_mono(samples: &[f32], channels: usize, frames: usize) -> Vec<f32> {
    let channels = channels.max(1);
    (0..frames)
        .map(|frame| {
            let start = frame * channels;
            let sum = samples[start..start + channels]
                .iter()
                .copied()
                .sum::<f32>();
            (sum / channels as f32).clamp(-1.0, 1.0)
        })
        .collect()
}

fn lowpass(samples: &[f32], sample_rate: u32, cutoff_hz: f32) -> Vec<f32> {
    let alpha = (1.0 - (-std::f32::consts::TAU * cutoff_hz / sample_rate.max(1) as f32).exp())
        .clamp(0.0, 1.0);
    let mut value = 0.0_f32;
    samples
        .iter()
        .map(|sample| {
            value += alpha * (*sample - value);
            value
        })
        .collect()
}

fn subtract_samples(left: &[f32], right: &[f32]) -> Vec<f32> {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left - right).clamp(-1.0, 1.0))
        .collect()
}

fn normalized_band(mut samples: Vec<f32>, gain: f32) -> Vec<f32> {
    let peak = samples
        .iter()
        .map(|sample| sample.abs())
        .fold(0.0_f32, f32::max)
        .max(0.001);
    let scale = gain / peak.max(0.32);
    for sample in &mut samples {
        *sample = (*sample * scale).clamp(-1.0, 1.0);
    }
    samples
}
