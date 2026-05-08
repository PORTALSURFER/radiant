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
    let p1 = band_peak_at(x - pixel_width, band, band_count, start, visible, bucket_frames, bucket_count);
    let p2 = band_peak_at(x, band, band_count, start, visible, bucket_frames, bucket_count);
    let p3 = band_peak_at(x + pixel_width, band, band_count, start, visible, bucket_frames, bucket_count);
    return p1 * 0.24 + p2 * 0.52 + p3 * 0.24;
}

fn projected_band_peak(x: f32, pixel_width: f32, band: u32, band_count: u32, start: f32, visible: f32, bucket_frames: f32, bucket_count: u32, frames_per_pixel: f32) -> f32 {
    let peak = smoothed_band_peak(x, pixel_width, band, band_count, start, visible, bucket_frames, bucket_count);
    let left = smoothed_band_peak(x - pixel_width, pixel_width, band, band_count, start, visible, bucket_frames, bucket_count);
    let right = smoothed_band_peak(x + pixel_width, pixel_width, band, band_count, start, visible, bucket_frames, bucket_count);
    let neighbor = max(left, right);
    let corner_limit = mix(0.24, 0.095, smoothstep(18.0, 260.0, frames_per_pixel));
    let corner_delta = max(peak - neighbor, 0.0);
    let corner_strength = smoothstep(corner_limit, corner_limit * 2.8, corner_delta);
    return mix(peak, neighbor + corner_limit, corner_strength * 0.42);
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
    let base_feather = max(0.55 / max(params.dest.y, 1.0), 0.00075);

    let vignette = (1.0 - y) * (1.0 - y);
    var color = vec4<f32>(0.004, 0.005, 0.005, 1.0);
    color = blend(vec3<f32>(0.012, 0.030, 0.044), vignette * 0.18, color);

    let band_colors = array<vec4<f32>, 4>(
        vec4<f32>(0.015, 0.16, 0.82, 0.46),
        vec4<f32>(0.58, 0.25, 0.07, 0.42),
        vec4<f32>(0.98, 0.46, 0.07, 0.50),
        vec4<f32>(1.00, 0.92, 0.66, 0.62),
    );
    let inner_colors = array<vec3<f32>, 4>(
        vec3<f32>(0.06, 0.70, 1.00),
        vec3<f32>(0.92, 0.44, 0.16),
        vec3<f32>(1.00, 0.68, 0.20),
        vec3<f32>(1.00, 1.00, 0.88),
    );
    let ridge_colors = array<vec3<f32>, 4>(
        vec3<f32>(0.02, 0.92, 1.00),
        vec3<f32>(0.95, 0.52, 0.20),
        vec3<f32>(1.00, 0.76, 0.28),
        vec3<f32>(1.00, 1.00, 0.96),
    );
    let glow_colors = array<vec3<f32>, 4>(
        vec3<f32>(0.00, 0.34, 1.00),
        vec3<f32>(0.78, 0.23, 0.04),
        vec3<f32>(1.00, 0.36, 0.02),
        vec3<f32>(1.00, 0.82, 0.36),
    );
    let band_scales = array<f32, 4>(1.00, 0.82, 0.72, 0.48);
    var low_signal = 0.0;
    var mid_signal = 0.0;
    var high_signal = 0.0;
    for (var band = 0u; band < min(band_count, 4u); band = band + 1u) {
        let peak = projected_band_peak(in.local.x, pixel_width, band, band_count, start, visible, bucket_frames, bucket_count, frames_per_pixel);
        if (band == 0u) {
            low_signal = peak;
        } else if (band == min(2u, band_count - 1u)) {
            mid_signal = peak;
        } else if (band == min(3u, band_count - 1u)) {
            high_signal = peak;
        }
        let intensity = clamp(peak * 1.04, 0.0, 1.0);
        let extent = peak * band_scales[band] * 0.86;
        let edge = abs(y - extent);
        let aa = max(fwidth(y - extent) * 0.90, base_feather);
        let coverage = smoothstep(extent + aa, extent - aa, y);
        let ridge = 1.0 - smoothstep(aa * 0.35, aa * 1.70, edge);
        let inside = clamp(1.0 - y / max(extent, 0.001), 0.0, 1.0);
        let inner_light = inside * inside;
        let shell_light = clamp(y / max(extent, 0.001), 0.0, 1.0);
        let edge_halo = smoothstep(extent + aa * 8.0, extent, y) * (1.0 - coverage);
        let heat_mix = smoothstep(0.38, 0.96, intensity);
        var low_depth = 0.0;
        var low_lift = vec3<f32>(0.0);
        var low_belly = 0.0;
        var ridge_seed = ridge_colors[band];
        if (band == 0u) {
            low_depth = smoothstep(0.03, 0.82, peak);
            let low_inner_cyan = low_depth * inside * inside * inside;
            let low_outer_blue = low_depth * smoothstep(0.28, 0.92, shell_light);
            let low_edge = low_depth * (1.0 - smoothstep(aa * 1.5, aa * 9.0, edge));
            let belly = clamp(1.0 - inside, 0.0, 1.0);
            low_belly = low_depth * belly * belly;
            low_lift = vec3<f32>(0.0, 0.17, 0.34) * low_outer_blue
                + vec3<f32>(0.0, 0.24, 0.44) * low_inner_cyan
                + vec3<f32>(0.0, 0.10, 0.20) * low_edge;
            ridge_seed = mix(ridge_seed, vec3<f32>(0.32, 0.95, 1.00), low_depth * 0.55);
        }
        let low_band = select(0.0, 1.0, band == 0u);
        let body_rgb = mix(
            mix(
                band_colors[band].rgb * (1.0 - low_belly * 0.18),
                inner_colors[band],
                inner_light * (0.46 + low_depth * 0.16),
            ) + low_lift,
            ridge_colors[band],
            shell_light * (0.14 + low_depth * 0.10) + heat_mix * 0.12,
        );
        let ridge_rgb = mix(
            ridge_seed,
            vec3<f32>(1.0, 0.86, 0.38),
            heat_mix * 0.30 * (1.0 - low_band * 0.70),
        );
        let alpha_boost = 0.62 + intensity * 0.28 + inner_light * 0.15;
        color = blend(glow_colors[band], edge_halo * band_colors[band].a * (0.06 + intensity * 0.09 + low_depth * 0.05), color);
        color = blend(body_rgb, band_colors[band].a * coverage * alpha_boost, color);
        color = blend(ridge_rgb, ridge * band_colors[band].a * (0.22 + intensity * 0.30), color);
    }
    let heat = clamp(low_signal * 0.45 + mid_signal * 0.70 + high_signal * 1.10, 0.0, 1.0);
    let hot = smoothstep(0.52, 0.98, heat);
    let center_width = max(5.0 / max(params.dest.y, 1.0), 0.014);
    let center = 1.0 - smoothstep(0.0, center_width, abs(in.local.y - 0.5));
    color = blend(vec3<f32>(1.00, 0.72, 0.28), center * (0.14 + hot * 0.14), color);
    color = blend(vec3<f32>(1.00, 0.98, 0.84), center * high_signal * 0.24, color);

    let cursor_half_width = max(params.cursor_width / max(params.dest.z, 1.0), 0.0005);
    if (params.cursor_ratio >= 0.0 && abs(in.local.x - params.cursor_ratio) <= cursor_half_width) {
        color = params.cursor_color;
    }
    return color;
}
