struct Params {
    dest: vec4<f32>,
    frame_range: vec4<f32>,
    summary_meta: vec4<f32>,
    gain_preview_a: vec4<f32>,
    gain_preview_b: vec4<f32>,
    gain_preview_c: vec4<f32>,
    target_size: vec2<f32>,
    cursor_ratio: f32,
    cursor_width: f32,
    cursor_color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> params: Params;
@group(0) @binding(1)
var<storage, read> summary_values: array<f32>;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    let local = corners[vertex_index];
    let pixel = params.dest.xy + local * params.dest.zw;
    let clip = vec2<f32>(
        pixel.x / params.target_size.x * 2.0 - 1.0,
        1.0 - pixel.y / params.target_size.y * 2.0,
    );
    var out: VertexOut;
    out.position = vec4<f32>(clip, 0.0, 1.0);
    out.local = local;
    return out;
}

fn summary_peak(bucket: u32, band: u32, band_count: u32, bucket_count: u32) -> f32 {
    if (bucket >= bucket_count || band >= band_count) {
        return 0.0;
    }
    let index = (bucket * band_count + band) * 2u;
    let low = summary_values[index];
    let high = summary_values[index + 1u];
    return max(abs(low), abs(high));
}

fn band_peak_at(x: f32, band: u32, band_count: u32, start: f32, visible: f32, bucket_frames: f32, bucket_count: u32) -> f32 {
    let center = clamp(x, 0.0, 1.0);
    let frame = max(start + visible * center, 0.0);
    let bucket = u32(clamp(floor(frame / max(bucket_frames, 1.0)), 0.0, f32(bucket_count - 1u)));
    return summary_peak(bucket, band, band_count, bucket_count);
}

fn smoothed_band_peak(x: f32, pixel_width: f32, band: u32, band_count: u32, start: f32, visible: f32, bucket_frames: f32, bucket_count: u32) -> f32 {
    return band_peak_at(x, band, band_count, start, visible, bucket_frames, bucket_count);
}

fn projected_band_peak(x: f32, pixel_width: f32, band: u32, band_count: u32, start: f32, visible: f32, bucket_frames: f32, bucket_count: u32, frames_per_pixel: f32) -> f32 {
    return smoothed_band_peak(x, pixel_width, band, band_count, start, visible, bucket_frames, bucket_count);
}

fn preview_curve_value(t: f32, curve: f32) -> f32 {
    if (curve <= 0.0) {
        return clamp(t, 0.0, 1.0);
    }
    let x = clamp(t, 0.0, 1.0);
    let x2 = x * x;
    let x3 = x2 * x;
    let smootherstep = x3 * (x * (x * 6.0 - 15.0) + 10.0);
    return x * (1.0 - curve) + smootherstep * curve;
}

fn preview_gain_at_position(position: f32) -> f32 {
    if (params.gain_preview_a.x < 0.5) {
        return 1.0;
    }
    let selection_start = min(params.gain_preview_a.y, params.gain_preview_a.z);
    let selection_end = max(params.gain_preview_a.y, params.gain_preview_a.z);
    let width = selection_end - selection_start;
    if (width <= 0.0) {
        return 1.0;
    }

    let fade_in_mute = max(params.gain_preview_c.x, 0.0);
    if (fade_in_mute > 0.0) {
        let fade_start = selection_start - width * fade_in_mute;
        if (position >= fade_start && position <= selection_start) {
            let t = clamp((position - fade_start) / max(selection_start - fade_start, 0.000001), 0.0, 1.0);
            let fade_in_curve = clamp(params.gain_preview_b.y, 0.0, 1.0);
            return 1.0 - preview_curve_value(t, fade_in_curve);
        }
    }
    let fade_out_mute = max(params.gain_preview_c.y, 0.0);
    if (fade_out_mute > 0.0) {
        let fade_end = selection_end + width * fade_out_mute;
        if (position >= selection_end && position <= fade_end) {
            let t = clamp((position - selection_end) / max(fade_end - selection_end, 0.000001), 0.0, 1.0);
            let fade_out_curve = clamp(params.gain_preview_b.w, 0.0, 1.0);
            return preview_curve_value(t, fade_out_curve);
        }
    }
    if (position < selection_start || position > selection_end) {
        return 1.0;
    }

    var gain = 1.0;
    let fade_in_len = width * clamp(params.gain_preview_b.x, 0.0, 1.0);
    if (fade_in_len > 0.0) {
        let time_in = position - selection_start;
        if (time_in < fade_in_len) {
            gain = gain * preview_curve_value(time_in / fade_in_len, clamp(params.gain_preview_b.y, 0.0, 1.0));
        }
    }
    let fade_out_len = width * clamp(params.gain_preview_b.z, 0.0, 1.0);
    if (fade_out_len > 0.0) {
        let time_until_end = selection_end - position;
        if (time_until_end < fade_out_len) {
            gain = gain * preview_curve_value(time_until_end / fade_out_len, clamp(params.gain_preview_b.w, 0.0, 1.0));
        }
    }
    return gain * clamp(params.gain_preview_a.w, 0.0, 4.0);
}

fn blend(src: vec3<f32>, alpha: f32, dst: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(mix(dst.rgb, src, clamp(alpha, 0.0, 1.0)), 1.0);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let frames = u32(max(params.frame_range.z, 1.0));
    let band_count = u32(max(params.frame_range.w, 1.0));
    let start = params.frame_range.x;
    let end = max(params.frame_range.y, start + 1.0);
    let bucket_frames = max(params.summary_meta.x, 1.0);
    let bucket_count = u32(max(params.summary_meta.y, 1.0));
    let visible = end - start;
    let pixel_width = 1.0 / max(params.dest.z, 1.0);
    let frames_per_pixel = visible * pixel_width;
    let frame_position = clamp(
        (start + visible * clamp(in.local.x, 0.0, 1.0)) / max(f32(frames) - 1.0, 1.0),
        0.0,
        1.0,
    );
    let preview_gain = preview_gain_at_position(frame_position);
    let y = abs(in.local.y - 0.5) * 2.0;
    let base_feather = max(0.17 / max(params.dest.y, 1.0), 0.00018);

    let vignette = (1.0 - y) * (1.0 - y);
    var color = vec4<f32>(0.0018, 0.0018, 0.0018, 1.0);
    color = blend(vec3<f32>(0.010, 0.010, 0.009), vignette * 0.045, color);

    let band_colors = array<vec4<f32>, 4>(
        vec4<f32>(0.00, 0.52, 0.74, 0.98),
        vec4<f32>(0.70, 0.16, 0.00, 0.96),
        vec4<f32>(0.96, 0.98, 0.94, 0.74),
        vec4<f32>(1.00, 1.00, 0.98, 0.05),
    );
    let inner_colors = array<vec3<f32>, 4>(
        vec3<f32>(0.00, 0.40, 0.62),
        vec3<f32>(0.58, 0.10, 0.00),
        vec3<f32>(1.00, 1.00, 1.00),
        vec3<f32>(1.00, 1.00, 0.98),
    );
    let ridge_colors = array<vec3<f32>, 4>(
        vec3<f32>(0.04, 0.96, 1.00),
        vec3<f32>(1.00, 0.26, 0.02),
        vec3<f32>(1.00, 1.00, 1.00),
        vec3<f32>(1.00, 1.00, 1.00),
    );
    let glow_colors = array<vec3<f32>, 4>(
        vec3<f32>(0.00, 0.18, 0.26),
        vec3<f32>(0.24, 0.045, 0.00),
        vec3<f32>(0.98, 0.88, 0.66),
        vec3<f32>(0.78, 0.72, 0.62),
    );
    let band_scales = array<f32, 4>(0.93, 0.43, 0.046, 0.02);
    let band_gains = array<f32, 4>(1.02, 1.10, 2.08, 0.12);
    let band_gamma = array<f32, 4>(1.03, 0.94, 0.42, 1.70);
    var raw_signal = 0.0;
    if (band_count > 3u) {
        raw_signal = projected_band_peak(in.local.x, pixel_width, 3u, band_count, start, visible, bucket_frames, bucket_count, frames_per_pixel);
    }
    raw_signal = clamp(raw_signal * preview_gain, 0.0, 1.0);
    let display_peak = pow(clamp(raw_signal * 1.02, 0.0, 1.0), 0.54);
    let raw_carrier = smoothstep(0.010, 0.55, display_peak);
    var low_signal = 0.0;
    if (band_count > 0u) {
        low_signal = projected_band_peak(in.local.x, pixel_width, 0u, band_count, start, visible, bucket_frames, bucket_count, frames_per_pixel);
    }
    low_signal = clamp(low_signal * preview_gain, 0.0, 1.0);
    var mid_signal = 0.0;
    if (band_count > 1u) {
        mid_signal = projected_band_peak(in.local.x, pixel_width, 1u, band_count, start, visible, bucket_frames, bucket_count, frames_per_pixel);
    }
    mid_signal = clamp(mid_signal * preview_gain, 0.0, 1.0);
    var high_signal = 0.0;
    if (band_count > 2u) {
        high_signal = projected_band_peak(in.local.x, pixel_width, 2u, band_count, start, visible, bucket_frames, bucket_count, frames_per_pixel);
    }
    high_signal = clamp(high_signal * preview_gain, 0.0, 1.0);
    let low_peak_ownership = smoothstep(0.10, 0.42, low_signal);
    let mid_dominance = smoothstep(0.18, 0.54, mid_signal) * (1.0 - low_peak_ownership * 0.55);
    let high_dominance = smoothstep(0.10, 0.30, high_signal) * (1.0 - low_peak_ownership * 0.80);
    for (var band = 0u; band < min(band_count, 4u); band = band + 1u) {
        var peak = projected_band_peak(in.local.x, pixel_width, band, band_count, start, visible, bucket_frames, bucket_count, frames_per_pixel);
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
            let low_inner_cyan = low_depth * inside * inside * 0.06;
            let low_outer_blue = low_depth * smoothstep(0.40, 0.95, shell_light) * 0.08;
            let low_edge = low_depth * (1.0 - smoothstep(aa * 0.8, aa * 4.0, edge)) * 0.03;
            let belly = clamp(1.0 - inside, 0.0, 1.0);
            low_belly = low_depth * belly * belly * 0.025;
            low_lift = vec3<f32>(0.0, 0.030, 0.035) * low_outer_blue
                + vec3<f32>(0.0, 0.026, 0.028) * low_inner_cyan
                + vec3<f32>(0.0, 0.012, 0.014) * low_edge;
            ridge_seed = mix(ridge_seed, vec3<f32>(0.10, 0.82, 1.00), low_depth * 0.06);
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
            let low_center = mix(vec3<f32>(0.00, 0.32, 0.46), band_colors[band].rgb, inside * 0.10);
            let low_edge = mix(vec3<f32>(0.00, 0.58, 0.70), ridge_colors[band], low_gradient * 0.54);
            body_rgb = mix(low_center, low_edge, low_gradient * 0.52 + heat_mix * 0.035);
            body_rgb = body_rgb + vec3<f32>(0.00, 0.020, 0.026) * inner_light * low_depth;
        } else if (band == 1u) {
            let mid_gradient = smoothstep(0.12, 0.90, shell_light);
            let mid_center = mix(vec3<f32>(0.50, 0.080, 0.015), band_colors[band].rgb, inside * 0.08);
            let mid_edge = mix(vec3<f32>(0.86, 0.18, 0.02), ridge_colors[band], mid_gradient * 0.46);
            body_rgb = mix(mid_center, mid_edge, mid_gradient * 0.44 + heat_mix * 0.030);
            body_rgb = body_rgb + vec3<f32>(0.035, 0.006, 0.000) * inner_light * intensity;
        } else if (band == 2u) {
            let high_core_tint = smoothstep(0.065, 0.68, shaped_peak) * (0.58 + inner_light * 0.24);
            let high_air = smoothstep(0.18, 0.90, shell_light) * 0.12;
            let high_body = mix(
                vec3<f32>(0.80, 0.92, 0.94),
                vec3<f32>(1.0, 0.99, 0.90),
                high_core_tint,
            );
            let high_edge = mix(vec3<f32>(0.68, 0.84, 0.86), high_body, inner_light * 0.84 + heat_mix * 0.06);
            body_rgb = high_edge + vec3<f32>(0.0, 0.060, 0.070) * high_air;
        }
        let ridge_rgb = mix(
            ridge_seed,
            vec3<f32>(1.0, 0.92, 0.62),
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
                band_peak_at(in.local.x - neighbor_span, 2u, band_count, start, visible, bucket_frames, bucket_count) * preview_gain,
                band_peak_at(in.local.x + neighbor_span, 2u, band_count, start, visible, bucket_frames, bucket_count) * preview_gain,
            ),
        );
    }
    let high_core = pow(smoothstep(0.018, 0.44, white_signal), 0.54);
    color = blend(vec3<f32>(0.82, 0.38, 0.08), center * hot * 0.007, color);
    color = blend(vec3<f32>(1.00, 1.00, 0.95), high_core * (center * 0.28 + center_core * 0.78), color);

    let cursor_half_width = max(params.cursor_width / max(params.dest.z, 1.0), 0.0005);
    if (params.cursor_ratio >= 0.0 && abs(in.local.x - params.cursor_ratio) <= cursor_half_width) {
        color = params.cursor_color;
    }
    return color;
}
