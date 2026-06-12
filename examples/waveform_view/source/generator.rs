//! Deterministic signal fixtures for the retained waveform example.

#[cfg(test)]
use super::WaveformSummary;
use super::{BAND_COUNT, SignalSource, WaveformBand};
use radiant::runtime::GpuSignalSummary;
use std::sync::Arc;

const SYNTHETIC_SAMPLE_RATE: u32 = 48_000;
const SYNTHETIC_SECONDS: usize = 4;

pub(crate) fn synthetic_signal_source() -> SignalSource {
    let frames = SYNTHETIC_SAMPLE_RATE as usize * SYNTHETIC_SECONDS;
    let samples = (0..frames)
        .map(|frame| {
            let t = frame as f32 / SYNTHETIC_SAMPLE_RATE as f32;
            let envelope = (1.0 - t / SYNTHETIC_SECONDS as f32).clamp(0.18, 1.0);
            let fundamental = (std::f32::consts::TAU * 72.0 * t).sin() * 0.48;
            let detail = (std::f32::consts::TAU * 220.0 * t).sin() * 0.24;
            let shimmer = (std::f32::consts::TAU * 1_760.0 * t).sin() * 0.1;
            ((fundamental + detail + shimmer) * envelope).clamp(-1.0, 1.0)
        })
        .collect::<Vec<_>>();
    signal_source_from_samples(
        String::from("synthetic signal"),
        SYNTHETIC_SAMPLE_RATE,
        1,
        samples,
    )
}

#[cfg(test)]
pub(crate) fn signal_source_from_samples(
    identity: String,
    sample_rate: u32,
    channels: usize,
    samples: Vec<f32>,
) -> SignalSource {
    signal_source_from_samples_impl(identity, sample_rate, channels, samples)
}

#[cfg(not(test))]
fn signal_source_from_samples(
    identity: String,
    sample_rate: u32,
    channels: usize,
    samples: Vec<f32>,
) -> SignalSource {
    signal_source_from_samples_impl(identity, sample_rate, channels, samples)
}

fn signal_source_from_samples_impl(
    identity: String,
    sample_rate: u32,
    channels: usize,
    samples: Vec<f32>,
) -> SignalSource {
    let bands = signal_bands_from_samples(&samples);
    let gpu_signal_samples = interleaved_band_samples(&bands);
    let gpu_signal_summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
        &gpu_signal_samples,
        samples.len(),
        BAND_COUNT,
    ));
    SignalSource {
        identity,
        sample_rate,
        channels,
        frames: samples.len(),
        #[cfg(test)]
        mono_summary: WaveformSummary::from_samples(&samples),
        #[cfg(test)]
        bands,
        #[cfg(test)]
        mono_samples: samples,
        gpu_signal_summary,
    }
}

fn signal_bands_from_samples(samples: &[f32]) -> [WaveformBand; BAND_COUNT] {
    let primary = samples.to_vec();
    let contour = shaped_band(samples, 0.74, |index| {
        0.62 + 0.28 * (index as f32 / 2_400.0).sin()
    });
    let texture = shaped_band(samples, 0.52, |index| {
        if (index / 384) % 2 == 0 { 0.72 } else { 0.38 }
    });
    let accent = shaped_band(samples, 0.36, |index| {
        0.35 + 0.5 * (index as f32 / 97.0).sin().abs()
    });
    [
        WaveformBand::new(primary),
        WaveformBand::new(contour),
        WaveformBand::new(texture),
        WaveformBand::new(accent),
    ]
}

fn shaped_band(samples: &[f32], gain: f32, shape: impl Fn(usize) -> f32) -> Vec<f32> {
    samples
        .iter()
        .enumerate()
        .map(|(index, sample)| (sample * gain * shape(index)).clamp(-1.0, 1.0))
        .collect()
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
