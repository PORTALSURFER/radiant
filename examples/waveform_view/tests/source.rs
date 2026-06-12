use super::*;

#[test]
fn interleaved_signal_bands_preserve_frame_order() {
    let bands = [
        WaveformBand::new(vec![1.0, 2.0]),
        WaveformBand::new(vec![3.0, 4.0]),
        WaveformBand::new(vec![5.0, 6.0]),
        WaveformBand::new(vec![7.0, 8.0]),
    ];

    assert_eq!(
        interleaved_band_samples(&bands).as_ref(),
        &[1.0, 3.0, 5.0, 7.0, 2.0, 4.0, 6.0, 8.0]
    );
}

#[test]
fn summary_stats_match_raw_range_stats() {
    let samples: Vec<f32> = (0..4096)
        .map(|index| ((index as f32 / 13.0).sin() * 0.7).clamp(-1.0, 1.0))
        .collect();
    let summary = WaveformSummary::from_samples(&samples);

    let summarized = summary.stats(&samples, 37, 3901);
    let raw = band_stats(&samples, 37, 3901);
    assert!((summarized.peak - raw.peak).abs() < 0.000_001);
    assert!((summarized.rms - raw.rms).abs() < 0.000_001);
}

#[test]
fn default_waveform_source_uses_synthetic_signal_without_input_path() {
    let file = synthetic_signal_source();

    assert!(file.sample_rate > 0);
    assert!(!file.mono_samples.is_empty());
    assert_eq!(file.frames, file.mono_samples.len());
    let image = render_waveform_image(&file, WaveformViewport::full(file.frames), 320, 96);
    assert_eq!(image.width, 320);
}
