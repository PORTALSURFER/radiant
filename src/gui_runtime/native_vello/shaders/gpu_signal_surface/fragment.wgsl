@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let frames = u32(max(params.frame_range.z, 1.0));
    let band_count = u32(max(params.frame_range.w, 1.0));
    let start = params.frame_range.x;
    let end = max(params.frame_range.y, start + 1.0);
    let bucket_frames = max(params.summary_meta.x, 1.0);
    let bucket_count = u32(max(params.summary_meta.y, 1.0));
    let bucket_offset = max(params.summary_meta.w, 0.0);
    let visible = end - start;
    let pixel_width = 1.0 / max(params.dest.z, 1.0);
    let summary_window = SignalSummaryWindow(start, visible, bucket_frames, bucket_count, bucket_offset, f32(frames));
    let frame_position = clamp(
        (start + visible * clamp(in.local.x, 0.0, 1.0)) / max(f32(frames) - 1.0, 1.0),
        0.0,
        1.0,
    );
    let preview_gain = preview_gain_at_position(frame_position);
    let y = abs(in.local.y - 0.5) * 2.0;
    let base_feather = max(0.17 / max(params.dest.y, 1.0), 0.00018);
    let vignette = (1.0 - y) * (1.0 - y);
    var color = vec4<f32>(0.082, 0.094, 0.098, 1.0);
    color = blend(vec3<f32>(0.106, 0.118, 0.122), vignette * 0.18, color);

    let band_colors = array<vec4<f32>, 4>(
        vec4<f32>(0.655, 0.231, 0.188, 0.88),
        vec4<f32>(0.843, 0.290, 0.220, 0.92),
        vec4<f32>(0.847, 0.839, 0.816, 0.78),
        vec4<f32>(0.925, 0.910, 0.875, 0.05),
    );
    let inner_colors = array<vec3<f32>, 4>(
        vec3<f32>(0.561, 0.188, 0.157),
        vec3<f32>(0.776, 0.259, 0.200),
        vec3<f32>(0.910, 0.898, 0.866),
        vec3<f32>(0.925, 0.910, 0.875),
    );
    let ridge_colors = array<vec3<f32>, 4>(
        vec3<f32>(0.914, 0.345, 0.263),
        vec3<f32>(0.941, 0.416, 0.333),
        vec3<f32>(0.953, 0.941, 0.906),
        vec3<f32>(0.953, 0.941, 0.906),
    );
    let glow_colors = array<vec3<f32>, 4>(
        vec3<f32>(0.208, 0.106, 0.098),
        vec3<f32>(0.290, 0.129, 0.110),
        vec3<f32>(0.647, 0.620, 0.569),
        vec3<f32>(0.510, 0.486, 0.443),
    );
    let band_scales = array<f32, 4>(0.93, 0.43, 0.046, 0.02);
    let band_gains = array<f32, 4>(1.02, 1.10, 2.08, 0.12);
    let band_gamma = array<f32, 4>(1.03, 0.94, 0.42, 1.70);
    var raw_signal = 0.0;
    if (band_count > 3u) {
        raw_signal = projected_band_peak(band_query(in.local.x, 3u, band_count), summary_window);
    }
    raw_signal = clamp(raw_signal * preview_gain, 0.0, 1.0);
    let display_peak = pow(clamp(raw_signal * 1.02, 0.0, 1.0), 0.54);
    let raw_carrier = smoothstep(0.010, 0.55, display_peak);
    var low_signal = 0.0;
    if (band_count > 0u) {
        low_signal = projected_band_peak(band_query(in.local.x, 0u, band_count), summary_window);
    }
    low_signal = clamp(low_signal * preview_gain, 0.0, 1.0);
    var mid_signal = 0.0;
    if (band_count > 1u) {
        mid_signal = projected_band_peak(band_query(in.local.x, 1u, band_count), summary_window);
    }
    mid_signal = clamp(mid_signal * preview_gain, 0.0, 1.0);
    var high_signal = 0.0;
    if (band_count > 2u) {
        high_signal = projected_band_peak(band_query(in.local.x, 2u, band_count), summary_window);
    }
    high_signal = clamp(high_signal * preview_gain, 0.0, 1.0);
    let low_peak_ownership = smoothstep(0.10, 0.42, low_signal);
    let mid_dominance = smoothstep(0.18, 0.54, mid_signal) * (1.0 - low_peak_ownership * 0.55);
    let high_dominance = smoothstep(0.10, 0.30, high_signal) * (1.0 - low_peak_ownership * 0.80);
    for (var band = 0u; band < min(band_count, 4u); band = band + 1u) {
        var peak = projected_band_peak(band_query(in.local.x, band, band_count), summary_window);
        if (band == 0u) {
            peak = low_signal;
        } else if (band == 1u) {
            peak = mid_signal;
        } else if (band == 2u) {
            peak = high_signal;
        } else if (band == 3u) {
            peak = clamp(peak * preview_gain, 0.0, 1.0);
        }
        var visible_peak = peak;
        var noise_floor = 0.004;
        if (band == 1u || band == 2u) {
            noise_floor = 0.00065;
        }
        if (visible_peak < noise_floor) {
            visible_peak = 0.0;
        }
        let shaped_peak = pow(clamp(visible_peak * band_gains[band], 0.0, 1.0), band_gamma[band]);
        let intensity = clamp(shaped_peak * 1.04, 0.0, 1.0);
        var quiet_presence = 0.0;
        if (band == 1u) {
            quiet_presence = smoothstep(0.0007, 0.018, visible_peak);
        } else if (band == 2u) {
            quiet_presence = smoothstep(0.0007, 0.014, visible_peak);
        }
        var extent = shaped_peak * band_scales[band] * 0.86;
        if (band == 0u) {
            let low_carrier = smoothstep(0.030, 0.28, low_signal) * raw_carrier;
            extent = max(extent, display_peak * 0.90 * low_carrier);
            extent = min(extent, display_peak * 0.96);
        }
        if (band == 1u) {
            let mid_carrier = smoothstep(0.012, 0.24, mid_signal) * raw_carrier;
            let mid_extent_limit = mix(0.58, 0.86, mid_dominance);
            let mid_extent_target = mix(0.50, 0.80, mid_dominance);
            extent = max(extent, quiet_presence * 0.010);
            extent = max(extent, display_peak * mid_extent_target * mid_carrier);
            extent = min(extent, display_peak * mid_extent_limit);
        } else if (band == 2u) {
            let high_carrier = smoothstep(0.006, 0.16, high_signal) * raw_carrier;
            let high_extent_target = mix(0.080, 0.86, high_dominance);
            extent = max(extent, quiet_presence * 0.0036);
            extent = max(extent, display_peak * high_extent_target * high_carrier);
        }
        let edge = abs(y - extent);
        var aa = max(fwidth(y - extent) * 0.44, base_feather);
        var coverage_softness = 0.34;
        if (band == 1u) {
            aa = max(fwidth(y - extent) * 0.38, base_feather * 0.78);
            coverage_softness = 0.24;
        } else if (band == 2u) {
            aa = max(fwidth(y - extent) * 0.22, base_feather * 0.36);
            coverage_softness = 0.14;
        }
        let coverage = smoothstep(extent + aa * coverage_softness, extent - aa * coverage_softness, y);
        let ridge = (1.0 - smoothstep(aa * 0.08, aa * 0.56, edge)) * smoothstep(0.008, 0.030, shaped_peak);
        let inside = clamp(1.0 - y / max(extent, 0.001), 0.0, 1.0);
        let inner_light = inside * inside;
        let shell_light = clamp(y / max(extent, 0.001), 0.0, 1.0);
        let edge_halo = smoothstep(extent + aa * 0.70, extent, y) * (1.0 - coverage);
        let heat_mix = smoothstep(0.38, 0.96, intensity);
        var low_depth = 0.0;
        var low_lift = vec3<f32>(0.0);
        var low_belly = 0.0;
        var ridge_seed = ridge_colors[band];
        if (band == 0u) {
            low_depth = smoothstep(0.03, 0.82, visible_peak);
            let low_inner_warm = low_depth * inside * inside * 0.06;
            let low_outer_coral = low_depth * smoothstep(0.40, 0.95, shell_light) * 0.08;
            let low_edge = low_depth * (1.0 - smoothstep(aa * 0.8, aa * 4.0, edge)) * 0.03;
            let belly = clamp(1.0 - inside, 0.0, 1.0);
            low_belly = low_depth * belly * belly * 0.025;
            low_lift = vec3<f32>(0.032, 0.010, 0.008) * low_outer_coral
                + vec3<f32>(0.028, 0.008, 0.006) * low_inner_warm
                + vec3<f32>(0.014, 0.004, 0.003) * low_edge;
            ridge_seed = mix(ridge_seed, vec3<f32>(0.914, 0.345, 0.263), low_depth * 0.06);
        }
        var low_band = 0.0;
        if (band == 0u) {
            low_band = 1.0;
        }
        var body_rgb = mix(
            mix(
                band_colors[band].rgb * (1.0 - low_belly * 0.18),
                inner_colors[band],
                inner_light * (0.08 + low_depth * 0.015),
            ) + low_lift,
            ridge_colors[band],
            shell_light * (0.006 + low_depth * 0.003) + heat_mix * 0.008,
        );
        if (band == 0u) {
            let low_gradient = smoothstep(0.16, 0.92, shell_light);
            let low_center = mix(vec3<f32>(0.455, 0.145, 0.125), band_colors[band].rgb, inside * 0.10);
            let low_edge = mix(vec3<f32>(0.820, 0.270, 0.215), ridge_colors[band], low_gradient * 0.54);
            body_rgb = mix(low_center, low_edge, low_gradient * 0.52 + heat_mix * 0.035);
            body_rgb = body_rgb + vec3<f32>(0.026, 0.006, 0.004) * inner_light * low_depth;
        } else if (band == 1u) {
            let mid_gradient = smoothstep(0.12, 0.90, shell_light);
            let mid_center = mix(vec3<f32>(0.650, 0.180, 0.135), band_colors[band].rgb, inside * 0.08);
            let mid_edge = mix(vec3<f32>(0.900, 0.320, 0.245), ridge_colors[band], mid_gradient * 0.46);
            body_rgb = mix(mid_center, mid_edge, mid_gradient * 0.44 + heat_mix * 0.030);
            body_rgb = body_rgb + vec3<f32>(0.035, 0.006, 0.000) * inner_light * intensity;
        } else if (band == 2u) {
            let high_core_tint = smoothstep(0.065, 0.68, shaped_peak) * (0.58 + inner_light * 0.24);
            let high_air = smoothstep(0.18, 0.90, shell_light) * 0.12;
            let high_body = mix(
                vec3<f32>(0.78, 0.79, 0.78),
                vec3<f32>(0.95, 0.94, 0.90),
                high_core_tint,
            );
            let high_edge = mix(vec3<f32>(0.66, 0.67, 0.66), high_body, inner_light * 0.84 + heat_mix * 0.06);
            body_rgb = high_edge + vec3<f32>(0.045, 0.040, 0.032) * high_air;
        }
        let ridge_rgb = mix(
            ridge_seed,
            vec3<f32>(0.95, 0.66, 0.54),
            heat_mix * 0.018 * (1.0 - low_band),
        );
        var presence = smoothstep(0.006, 0.046, shaped_peak);
        if (band == 1u) {
            presence = max(presence, quiet_presence * 0.42);
        } else if (band == 2u) {
            presence = max(presence, quiet_presence * 0.34);
        }
        let alpha_boost = 0.66 + intensity * 0.26 + inner_light * 0.025;
        var band_alpha_scale = 1.0;
        if (band == 0u) {
            band_alpha_scale = 0.86;
        } else if (band == 1u) {
            band_alpha_scale = 0.94;
        }
        if (band == 2u) {
            band_alpha_scale = 0.46 + inner_light * 0.30;
        } else if (band == 3u) {
            band_alpha_scale = 0.12;
        }
        var ridge_alpha_scale = 1.0;
        if (band == 0u) {
            ridge_alpha_scale = 0.030;
        } else if (band == 1u) {
            ridge_alpha_scale = 0.070;
        }
        color = blend(glow_colors[band], edge_halo * band_colors[band].a * (0.0007 + intensity * 0.0020) * presence, color);
        color = blend(body_rgb, band_colors[band].a * coverage * alpha_boost * 0.94 * presence * band_alpha_scale, color);
        color = blend(ridge_rgb, ridge * band_colors[band].a * (0.24 + intensity * 0.08) * ridge_alpha_scale, color);
    }
    let heat = clamp(low_signal * 0.14 + mid_signal * 0.46 + high_signal * 0.96, 0.0, 1.0);
    let hot = smoothstep(0.58, 1.0, heat);
    let center_width = max(0.22 / max(params.dest.y, 1.0), 0.00068);
    let center_core_width = max(0.070 / max(params.dest.y, 1.0), 0.00024);
    let center = 1.0 - smoothstep(0.0, center_width, abs(in.local.y - 0.5));
    let center_core = 1.0 - smoothstep(0.0, center_core_width, abs(in.local.y - 0.5));
    var white_signal = high_signal;
    if (band_count > 2u) {
        let neighbor_span = pixel_width * 1.15;
        white_signal = max(
            white_signal,
            max(
                band_peak_at(band_query(in.local.x - neighbor_span, 2u, band_count), summary_window) * preview_gain,
                band_peak_at(band_query(in.local.x + neighbor_span, 2u, band_count), summary_window) * preview_gain,
            ),
        );
    }
    let high_core = pow(smoothstep(0.018, 0.44, white_signal), 0.54);
    color = blend(vec3<f32>(0.914, 0.345, 0.263), center * hot * 0.007, color);
    color = blend(vec3<f32>(0.953, 0.941, 0.906), high_core * (center * 0.28 + center_core * 0.78), color);

    let cursor_half_width = max(params.cursor_width / max(params.dest.z, 1.0), 0.0005);
    if (params.cursor_ratio >= 0.0 && abs(in.local.x - params.cursor_ratio) <= cursor_half_width) {
        color = params.cursor_color;
    }
    return color;
}
