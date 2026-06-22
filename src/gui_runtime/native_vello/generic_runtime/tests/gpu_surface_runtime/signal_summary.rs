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
fn gpu_signal_shader_groups_projection_parameters() {
    let shader = super::super::super::gpu_surface::GPU_SIGNAL_SHADER;

    assert!(shader.contains("struct SignalSummaryWindow"));
    assert!(shader.contains("struct SignalBandQuery"));
    assert!(
        shader.contains("fn band_peak_at(query: SignalBandQuery, window: SignalSummaryWindow)")
    );
    assert!(
        shader.contains(
            "fn projected_band_peak(query: SignalBandQuery, window: SignalSummaryWindow)"
        )
    );
    assert!(!shader.contains("fn band_peak_at(x: f32, band: u32, band_count: u32"));
    assert!(!shader.contains("fn projected_band_peak(x: f32, pixel_width: f32"));
}

#[test]
fn gpu_signal_shader_keeps_waveform_bands_visually_distinct() {
    let shader = super::super::super::gpu_surface::GPU_SIGNAL_SHADER;

    assert!(shader.contains("band_scales = array<f32, 4>(0.93, 0.43, 0.046, 0.02)"));
    assert!(shader.contains("band_gamma = array<f32, 4>(1.03, 0.94, 0.42, 1.70)"));
    assert!(shader.contains("raw_signal = projected_band_peak"));
    assert!(shader.contains("display_peak"));
    assert!(shader.contains("vec4<f32>(0.00, 0.52, 0.74, 0.98)"));
    assert!(shader.contains("vec4<f32>(0.70, 0.16, 0.00, 0.96)"));
    assert!(shader.contains("vec4<f32>(0.96, 0.98, 0.94, 0.74)"));
    assert!(shader.contains("let low_gradient = smoothstep(0.16, 0.92, shell_light);"));
    assert!(shader.contains("let mid_gradient = smoothstep(0.12, 0.90, shell_light);"));
    assert!(shader.contains("let high_edge = mix(vec3<f32>(0.68, 0.84, 0.86), high_body"));
    assert!(shader.contains("coverage_softness = 0.24;"));
    assert!(shader.contains("coverage_softness = 0.14;"));
    assert!(shader.contains("band_alpha_scale = 0.46 + inner_light * 0.30;"));
    assert!(!shader.contains("vec4<f32>(0.08, 0.84, 0.36"));
}

#[test]
fn gpu_signal_shader_uses_raw_peak_to_shape_colored_bands() {
    let shader = super::super::super::gpu_surface::GPU_SIGNAL_SHADER;

    assert!(shader.contains("let display_peak = pow(clamp(raw_signal * 1.02, 0.0, 1.0), 0.54);"));
    assert!(shader.contains("let raw_carrier = smoothstep(0.010, 0.55, display_peak);"));
    assert!(shader.contains("let low_peak_ownership = smoothstep(0.10, 0.42, low_signal);"));
    assert!(shader.contains(
        "let mid_dominance = smoothstep(0.18, 0.54, mid_signal) * (1.0 - low_peak_ownership * 0.55);"
    ));
    assert!(shader.contains(
        "let high_dominance = smoothstep(0.10, 0.30, high_signal) * (1.0 - low_peak_ownership * 0.80);"
    ));
    assert!(
        shader.contains("let low_carrier = smoothstep(0.030, 0.28, low_signal) * raw_carrier;")
    );
    assert!(
        shader.contains("let mid_carrier = smoothstep(0.012, 0.24, mid_signal) * raw_carrier;")
    );
    assert!(shader.contains("display_peak * 0.90 * low_carrier"));
    assert!(shader.contains("display_peak * mid_extent_target * mid_carrier"));
    assert!(shader.contains("let high_extent_target = mix(0.080, 0.86, high_dominance);"));
    assert!(shader.contains("display_peak * high_extent_target * high_carrier"));
    assert!(shader.contains("let high_core = pow(smoothstep(0.018, 0.44, white_signal), 0.54);"));
    assert!(!shader.contains("max(mid_signal * 0.70"));
    assert!(!shader.contains("high_signal * 0.35"));
    assert!(!shader.contains("display_peak_line"));
    assert!(!shader.contains("display_coverage"));
}

#[test]
fn gpu_signal_shader_previews_outer_fade_extensions_as_crossfades() {
    let shader = super::super::super::gpu_surface::GPU_SIGNAL_SHADER;

    assert!(shader.contains("return outer_gain * (1.0 - preview_curve_value(t, fade_in_curve));"));
    assert!(shader.contains("return outer_gain * preview_curve_value(t, fade_out_curve);"));
    assert!(!shader.contains("if (position >= mute_start && position <= selection_start)"));
    assert!(!shader.contains("if (position >= selection_end && position <= mute_end)"));
}
