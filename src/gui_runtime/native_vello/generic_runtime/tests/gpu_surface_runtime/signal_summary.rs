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
fn gpu_signal_shader_parses_as_wgsl() {
    naga::front::wgsl::parse_str(super::super::super::gpu_surface::GPU_SIGNAL_SHADER)
        .expect("signal surface shader should parse before runtime");
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
fn gpu_signal_shader_smooths_colored_bands_only_within_one_destination_pixel() {
    let shader = super::super::super::gpu_surface::GPU_SIGNAL_SHADER;

    assert!(
        shader
            .contains("fn smoothed_band_peak(query: SignalBandQuery, window: SignalSummaryWindow)")
    );
    assert!(shader.contains("let half_pixel = 0.5 / max(params.dest.z, 1.0);"));
    assert!(shader.contains("if (boundary_distance > half_pixel)"));
    assert!(shader.contains("let left_peak = summary_peak"));
    assert!(shader.contains("let right_peak = summary_peak"));
    assert!(shader.contains("/ max(half_pixel * 2.0, 0.000001),"));
    assert!(shader.contains("return mix(left_peak, right_peak, transition);"));
}

fn interpolation_boundary_distance(bucket_position: f32, bucket_width: f32) -> f32 {
    let bucket_fraction = bucket_position.fract();
    let boundary = if bucket_fraction < 0.5 {
        bucket_position.floor()
    } else {
        bucket_position.ceil()
    };
    (bucket_position - boundary).abs() * bucket_width
}

#[test]
fn gpu_signal_shader_interpolation_boundary_distance_has_a_half_pixel_radius() {
    let pixel_width = 1.0 / 100.0;
    let half_pixel = pixel_width * 0.5;
    let bucket_width = 0.25;

    assert!((interpolation_boundary_distance(1.02, bucket_width) - half_pixel).abs() < 0.000001);
    assert!(interpolation_boundary_distance(1.019, bucket_width) < half_pixel);
    assert!(interpolation_boundary_distance(1.021, bucket_width) > half_pixel);
    assert!((half_pixel * 2.0 - pixel_width).abs() < 0.000001);
}

#[test]
fn gpu_signal_shader_keeps_raw_carrier_nearest_bucket_and_reuses_precomputed_bands() {
    let shader = super::super::super::gpu_surface::GPU_SIGNAL_SHADER;

    assert!(shader.contains(
        "raw_signal = band_peak_at(band_query(in.local.x, 3u, band_count), summary_window);"
    ));
    assert!(!shader.contains(
        "raw_signal = projected_band_peak(band_query(in.local.x, 3u, band_count), summary_window);"
    ));
    assert!(shader.contains(
        "let band_signals = array<f32, 4>(low_signal, mid_signal, high_signal, raw_signal);"
    ));
    assert!(shader.contains("let peak = band_signals[band];"));
    assert!(!shader.contains("var peak = projected_band_peak"));
}

#[test]
fn gpu_signal_shader_smooths_high_band_center_neighbors() {
    let shader = super::super::super::gpu_surface::GPU_SIGNAL_SHADER;

    assert!(shader.contains("let neighbor_span = pixel_width * 1.15;"));
    assert!(shader.contains(
        "projected_band_peak(band_query(in.local.x - neighbor_span, 2u, band_count), summary_window)"
    ));
    assert!(shader.contains(
        "projected_band_peak(band_query(in.local.x + neighbor_span, 2u, band_count), summary_window)"
    ));
    assert!(!shader.contains(
        "band_peak_at(band_query(in.local.x - neighbor_span, 2u, band_count), summary_window)"
    ));
    assert!(!shader.contains(
        "band_peak_at(band_query(in.local.x + neighbor_span, 2u, band_count), summary_window)"
    ));
}

#[test]
fn gpu_signal_shader_has_no_summary_sample_scan_loop() {
    let summary = super::super::super::gpu_surface::GPU_SIGNAL_SHADER
        .split_once("fn preview_curve_value")
        .expect("summary shader should precede preview helpers")
        .0;

    assert!(!summary.contains("for ("));
    assert!(!summary.contains("while ("));
}

#[test]
fn gpu_signal_shader_does_not_cap_gain_preview() {
    let shader = super::super::super::gpu_surface::GPU_SIGNAL_SHADER;

    assert!(shader.contains("return gain * max(params.gain_preview_a.w, 0.0);"));
    assert!(!shader.contains("clamp(params.gain_preview_a.w, 0.0, 4.0)"));
}

fn shader_vec4_array(shader: &str, declaration: &str) -> Vec<[f32; 4]> {
    let start = shader
        .find(declaration)
        .unwrap_or_else(|| panic!("missing shader declaration: {declaration}"));
    let body = shader[start..]
        .split_once(");")
        .expect("shader vec4 array should terminate with `);`")
        .0;

    body.lines()
        .filter_map(|line| {
            let values = line.split_once("vec4<f32>(")?.1.split_once(')')?.0;
            let values = values
                .split(',')
                .map(|value| value.trim().parse::<f32>().expect("shader color channel"))
                .collect::<Vec<_>>();
            (values.len() == 4).then(|| [values[0], values[1], values[2], values[3]])
        })
        .collect()
}

fn shader_vec3_mix_color(shader: &str, declaration: &str) -> [f32; 3] {
    let start = shader
        .find(declaration)
        .unwrap_or_else(|| panic!("missing shader declaration: {declaration}"));
    let values = shader[start..]
        .split_once("mix(vec3<f32>(")
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(values, _)| values)
        .expect("shader vec3 mix should contain an edge tint");
    let values = values
        .split(',')
        .map(|value| value.trim().parse::<f32>().expect("shader color channel"))
        .collect::<Vec<_>>();
    let [red, green, blue] = values.as_slice() else {
        panic!("shader vec3 mix should contain three color channels");
    };
    [*red, *green, *blue]
}

fn rgb_distance(left: [f32; 4], right: [f32; 4]) -> f32 {
    let squared = (0..3)
        .map(|channel| (left[channel] - right[channel]).powi(2))
        .sum::<f32>();
    squared.sqrt()
}

#[test]
fn gpu_signal_shader_keeps_waveform_bands_visually_distinct() {
    let shader = super::super::super::gpu_surface::GPU_SIGNAL_SHADER;

    assert!(shader.contains("band_scales = array<f32, 4>(0.93, 0.43, 0.046, 0.02)"));
    assert!(shader.contains("band_gamma = array<f32, 4>(1.03, 0.94, 0.42, 1.70)"));
    assert!(shader.contains("raw_signal = band_peak_at"));
    assert!(shader.contains("display_peak"));
    let band_colors = shader_vec4_array(shader, "let band_colors = array<vec4<f32>, 4>(");
    assert_eq!(band_colors.len(), 4);
    let [low, mid, high, raw_outer] = band_colors[..] else {
        panic!("waveform band palette should contain low, mid, high, and raw colors");
    };

    // The palette is a semantic contract: blue low band, coral mid band, and
    // bright cool-neutral high band, with the raw/outer band only a faint aid.
    assert!(low[2] > low[0] + 0.20 && low[2] > low[1] + 0.20);
    assert!(mid[0] > mid[1] + 0.30 && mid[0] > mid[2] + 0.30);
    let high_min = high[0].min(high[1]).min(high[2]);
    let high_max = high[0].max(high[1]).max(high[2]);
    assert!(high_min >= 0.80, "high band should remain bright: {high:?}");
    assert!(high[2] >= high[0], "high band should stay cool: {high:?}");
    assert!(
        high_max - high_min <= 0.12,
        "high band should be near-neutral: {high:?}"
    );
    assert!(
        raw_outer[3] <= 0.10,
        "raw/outer band should be low-opacity: {raw_outer:?}"
    );
    assert!(raw_outer[3] < high[3]);
    assert!(rgb_distance(low, mid) > 0.20);
    assert!(rgb_distance(low, high) > 0.20);
    assert!(rgb_distance(mid, high) > 0.20);
    assert!(shader.contains("let low_gradient = smoothstep(0.16, 0.92, shell_light);"));
    assert!(shader.contains("let mid_gradient = smoothstep(0.12, 0.90, shell_light);"));
    let high_edge_tint = shader_vec3_mix_color(shader, "let high_edge = mix(");
    let high_edge_min = high_edge_tint[0]
        .min(high_edge_tint[1])
        .min(high_edge_tint[2]);
    let high_edge_max = high_edge_tint[0]
        .max(high_edge_tint[1])
        .max(high_edge_tint[2]);
    assert!(
        high_edge_min >= 0.68,
        "high edge tint should remain bright: {high_edge_tint:?}"
    );
    assert!(
        high_edge_tint[2] >= high_edge_tint[0] && high_edge_tint[2] >= high_edge_tint[1],
        "high edge tint should stay cool: {high_edge_tint:?}"
    );
    assert!(
        high_edge_max - high_edge_min <= 0.12,
        "high edge tint should be near-neutral: {high_edge_tint:?}"
    );
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
