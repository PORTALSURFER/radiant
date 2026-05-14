use super::*;

#[test]
fn signal_summary_pyramid_preserves_band_min_max_and_level_selection() {
    let samples: Arc<[f32]> = [
        -0.1, 0.2, -0.7, 0.4, 0.3, -0.8, 0.9, -0.2, -0.5, 0.1, 0.6, -0.6,
    ]
    .into_iter()
    .collect();
    let summary = GpuSignalSummary::from_interleaved_samples(&samples, 6, 2);

    assert_eq!(summary.levels[0].bucket_frames, 1);
    assert_eq!(summary.levels[0].buckets[0].min, -0.1);
    assert_eq!(summary.levels[0].buckets[0].max, -0.1);
    assert!(summary.levels.iter().any(|level| {
        level.bucket_frames >= 4 && level.buckets[0].min <= -0.7 && level.buckets[0].max >= 0.9
    }));
    assert_eq!(summary.level_for_frames_per_pixel(1.0), 0);
    assert!(summary.level_for_frames_per_pixel(5.0) > 0);
}

#[test]
fn gpu_signal_shader_uses_summary_sampling_without_looped_sample_scan() {
    assert!(!super::super::super::gpu_surface::GPU_SIGNAL_SHADER.contains("loop"));
    assert!(!super::super::super::gpu_surface::GPU_SIGNAL_SHADER.contains("fn band_peak("));
    assert!(super::super::super::gpu_surface::GPU_SIGNAL_SHADER.contains("summary_peak"));
}

#[test]
fn gpu_signal_shader_keeps_waveform_bands_visually_distinct() {
    let shader = super::super::super::gpu_surface::GPU_SIGNAL_SHADER;

    assert!(shader.contains("band_scales = array<f32, 4>(0.94, 0.49, 0.13, 0.02)"));
    assert!(shader.contains("band_gamma = array<f32, 4>(1.08, 0.98, 0.76, 1.70)"));
    assert!(shader.contains("vec4<f32>(0.00, 0.18, 0.92, 0.78)"));
    assert!(shader.contains("vec4<f32>(1.00, 0.39, 0.00, 0.76)"));
    assert!(shader.contains("vec4<f32>(1.00, 1.00, 0.96, 0.86)"));
    assert!(!shader.contains("vec4<f32>(0.08, 0.84, 0.36"));
}
