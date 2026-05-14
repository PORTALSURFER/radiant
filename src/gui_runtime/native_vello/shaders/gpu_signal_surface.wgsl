struct Params {
    dest: vec4<f32>,
    frame_range: vec4<f32>,
    summary_meta: vec4<f32>,
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
    let y = abs(in.local.y - 0.5) * 2.0;
    let base_feather = max(0.17 / max(params.dest.y, 1.0), 0.00018);

    let vignette = (1.0 - y) * (1.0 - y);
    var color = vec4<f32>(0.0018, 0.0018, 0.0018, 1.0);
    color = blend(vec3<f32>(0.010, 0.010, 0.009), vignette * 0.045, color);

    let band_colors = array<vec4<f32>, 4>(
        vec4<f32>(0.00, 0.55, 0.84, 0.94),
        vec4<f32>(0.84, 0.35, 0.02, 0.88),
        vec4<f32>(1.00, 1.00, 0.99, 1.00),
        vec4<f32>(1.00, 1.00, 0.98, 0.05),
    );
    let inner_colors = array<vec3<f32>, 4>(
        vec3<f32>(0.20, 0.84, 0.96),
        vec3<f32>(0.88, 0.45, 0.06),
        vec3<f32>(1.00, 1.00, 1.00),
        vec3<f32>(1.00, 1.00, 0.98),
    );
    let ridge_colors = array<vec3<f32>, 4>(
        vec3<f32>(0.06, 0.72, 0.94),
        vec3<f32>(0.86, 0.36, 0.04),
        vec3<f32>(1.00, 1.00, 1.00),
        vec3<f32>(1.00, 1.00, 1.00),
    );
    let glow_colors = array<vec3<f32>, 4>(
        vec3<f32>(0.00, 0.18, 0.26),
        vec3<f32>(0.34, 0.08, 0.00),
        vec3<f32>(0.98, 0.88, 0.66),
        vec3<f32>(0.78, 0.72, 0.62),
    );
    let band_scales = array<f32, 4>(0.93, 0.45, 0.046, 0.02);
    let band_gains = array<f32, 4>(0.98, 0.94, 2.08, 0.12);
    let band_gamma = array<f32, 4>(1.05, 1.02, 0.42, 1.70);
    var low_signal = 0.0;
    var mid_signal = 0.0;
    var high_signal = 0.0;
    for (var band = 0u; band < min(band_count, 4u); band = band + 1u) {
        let peak = projected_band_peak(in.local.x, pixel_width, band, band_count, start, visible, bucket_frames, bucket_count, frames_per_pixel);
        if (band == 0u) {
            low_signal = peak;
        } else if (band == 1u) {
            mid_signal = peak;
        } else if (band == 2u) {
            high_signal = peak;
        }
        var visible_peak = peak;
        if (visible_peak < 0.004) {
            visible_peak = 0.0;
        }
        let shaped_peak = pow(clamp(visible_peak * band_gains[band], 0.0, 1.0), band_gamma[band]);
        let intensity = clamp(shaped_peak * 1.04, 0.0, 1.0);
        let extent = shaped_peak * band_scales[band] * 0.86;
        let edge = abs(y - extent);
        var aa = max(fwidth(y - extent) * 0.44, base_feather);
        var coverage_softness = 0.28;
        if (band == 2u) {
            aa = max(fwidth(y - extent) * 0.16, base_feather * 0.26);
            coverage_softness = 0.055;
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
        if (band == 0u || band == 1u) {
            let soft_gradient = inner_light * 0.24 + shell_light * 0.04 + heat_mix * 0.055;
            let outer_tint = mix(band_colors[band].rgb, ridge_colors[band], shell_light * 0.055);
            body_rgb = mix(outer_tint, inner_colors[band], soft_gradient);
        } else if (band == 2u) {
            let white_snap = smoothstep(0.02, 0.46, shaped_peak) * (0.72 + inner_light * 0.22);
            body_rgb = mix(vec3<f32>(0.92, 0.93, 0.90), vec3<f32>(1.0, 1.0, 1.0), white_snap);
        }
        let ridge_rgb = mix(
            ridge_seed,
            vec3<f32>(1.0, 0.92, 0.62),
            heat_mix * 0.05 * (1.0 - low_band * 0.95),
        );
        let presence = smoothstep(0.006, 0.046, shaped_peak);
        let alpha_boost = 0.66 + intensity * 0.26 + inner_light * 0.025;
        var band_alpha_scale = 1.0;
        if (band == 0u) {
            band_alpha_scale = 0.86;
        } else if (band == 1u) {
            band_alpha_scale = 0.82;
        }
        if (band == 2u) {
            band_alpha_scale = 1.34;
        } else if (band == 3u) {
            band_alpha_scale = 0.12;
        }
        var ridge_alpha_scale = 1.0;
        if (band == 0u || band == 1u) {
            ridge_alpha_scale = 0.045;
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
    let high_core = pow(smoothstep(0.035, 0.44, high_signal), 0.52);
    color = blend(vec3<f32>(0.82, 0.38, 0.08), center * hot * 0.006, color);
    color = blend(vec3<f32>(1.00, 1.00, 1.00), high_core * (center * 0.34 + center_core * 0.74), color);

    let cursor_half_width = max(params.cursor_width / max(params.dest.z, 1.0), 0.0005);
    if (params.cursor_ratio >= 0.0 && abs(in.local.x - params.cursor_ratio) <= cursor_half_width) {
        color = params.cursor_color;
    }
    return color;
}
