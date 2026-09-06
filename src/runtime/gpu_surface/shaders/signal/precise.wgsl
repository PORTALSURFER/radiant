// Precise signal wrapper for the existing custom-shader ABI.
//
// `query` remains in the uploaded bucket window's local coordinates. The CPU
// validates all ranges before encoding this payload, so no absolute source
// frame is converted to f32 here.

struct SurfaceParams {
    dest: vec4<f32>,
    source: vec4<f32>,
    target_size: vec2<f32>,
    overlay_ratios: array<vec4<f32>, 2>,
    overlay_widths: array<vec4<f32>, 2>,
    overlay_colors: array<vec4<f32>, 8>,
};

struct PreciseMetadata {
    bucket_count: u32,
    band_count: u32,
    bucket_frames: u32,
    reserved_b: u32,
};

struct PrecisePresentation {
    // Local bucket start, span, optional normalized wrap boundary, and the
    // local bucket count to subtract after wrapping.
    query: vec4<f32>,
    gain_a: vec4<f32>,
    gain_b: vec4<f32>,
    gain_c: vec4<f32>,
    cursor: vec4<f32>,
    cursor_color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> params: SurfaceParams;
@group(0) @binding(1)
var<uniform> metadata: PreciseMetadata;
@group(0) @binding(2)
var<storage, read> summary_values: array<f32>;
@group(0) @binding(3)
var<uniform> presentation: PrecisePresentation;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
};

struct SignalBandQuery {
    x: f32,
    band: u32,
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

fn summary_peak(bucket: u32, band: u32) -> f32 {
    if (bucket >= metadata.bucket_count || band >= metadata.band_count) {
        return 0.0;
    }
    let index = (bucket * metadata.band_count + band) * 2u;
    return max(abs(summary_values[index]), abs(summary_values[index + 1u]));
}

fn band_query(x: f32, band: u32) -> SignalBandQuery {
    return SignalBandQuery(x, band);
}

fn summary_bucket_at(x: f32) -> f32 {
    let local_x = clamp(x, 0.0, 1.0);
    var bucket_position = presentation.query.x + presentation.query.y * local_x;
    if (local_x >= presentation.query.z) {
        bucket_position = bucket_position - presentation.query.w;
    }
    return bucket_position;
}

fn band_peak_at(query: SignalBandQuery) -> f32 {
    let bucket_position = summary_bucket_at(query.x);
    let bucket = u32(clamp(
        floor(bucket_position),
        0.0,
        f32(metadata.bucket_count - 1u),
    ));
    return summary_peak(bucket, query.band);
}

fn smoothed_band_peak(query: SignalBandQuery) -> f32 {
    let bucket_position = summary_bucket_at(query.x);
    let bucket_fraction = fract(bucket_position);
    var boundary = ceil(bucket_position);
    if (bucket_fraction < 0.5) {
        boundary = floor(bucket_position);
    }
    let bucket_frames = f32(metadata.bucket_frames);
    let bucket_width = bucket_frames / max(presentation.query.y * bucket_frames, 1.0);
    let pixel_width = 1.0 / max(params.dest.z, 1.0);
    let transition_width = min(pixel_width, bucket_width);
    let boundary_distance = abs(bucket_position - boundary) * bucket_width;
    if (boundary_distance > transition_width * 0.5) {
        return band_peak_at(query);
    }
    let last_bucket = metadata.bucket_count - 1u;
    let left_bucket = u32(clamp(boundary - 1.0, 0.0, f32(last_bucket)));
    let right_bucket = u32(clamp(boundary, 0.0, f32(last_bucket)));
    let left_peak = summary_peak(left_bucket, query.band);
    let right_peak = summary_peak(right_bucket, query.band);
    let transition = clamp(
        0.5 + (bucket_position - boundary) * bucket_width / max(transition_width, 0.000001),
        0.0,
        1.0,
    );
    return mix(left_peak, right_peak, transition);
}

fn projected_band_peak(query: SignalBandQuery) -> f32 {
    return smoothed_band_peak(query);
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
    if (presentation.gain_a.x < 0.5) {
        return 1.0;
    }
    let selection_start = min(presentation.gain_a.y, presentation.gain_a.z);
    let selection_end = max(presentation.gain_a.y, presentation.gain_a.z);
    let width = selection_end - selection_start;
    if (width <= 0.0) {
        return 1.0;
    }

    let fade_in_extension = max(presentation.gain_c.x, 0.0);
    if (fade_in_extension > 0.0) {
        let fade_start = selection_start - width * fade_in_extension;
        if (position >= fade_start && position <= selection_start) {
            let t = clamp((position - fade_start) / max(selection_start - fade_start, 0.000001), 0.0, 1.0);
            let fade_in_curve = clamp(presentation.gain_b.y, 0.0, 1.0);
            let outer_gain = clamp(presentation.gain_c.z, 0.0, 1.0);
            return outer_gain * (1.0 - preview_curve_value(t, fade_in_curve));
        }
    }
    let fade_out_extension = max(presentation.gain_c.y, 0.0);
    if (fade_out_extension > 0.0) {
        let fade_end = selection_end + width * fade_out_extension;
        if (position >= selection_end && position <= fade_end) {
            let t = clamp((position - selection_end) / max(fade_end - selection_end, 0.000001), 0.0, 1.0);
            let fade_out_curve = clamp(presentation.gain_b.w, 0.0, 1.0);
            let outer_gain = clamp(presentation.gain_c.w, 0.0, 1.0);
            return outer_gain * preview_curve_value(t, fade_out_curve);
        }
    }
    if (position < selection_start || position > selection_end) {
        return 1.0;
    }

    var gain = 1.0;
    let fade_in_len = width * clamp(presentation.gain_b.x, 0.0, 1.0);
    if (fade_in_len > 0.0) {
        let time_in = position - selection_start;
        if (time_in < fade_in_len) {
            gain = gain * preview_curve_value(time_in / fade_in_len, clamp(presentation.gain_b.y, 0.0, 1.0));
        }
    }
    let fade_out_len = width * clamp(presentation.gain_b.z, 0.0, 1.0);
    if (fade_out_len > 0.0) {
        let time_until_end = selection_end - position;
        if (time_until_end < fade_out_len) {
            gain = gain * preview_curve_value(time_until_end / fade_out_len, clamp(presentation.gain_b.w, 0.0, 1.0));
        }
    }
    return gain * max(presentation.gain_a.w, 0.0);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let band_count = metadata.band_count;
    let x = clamp(in.local.x, 0.0, 1.0);
    let preview_gain = preview_gain_at_position(x);
    var raw_signal = 0.0;
    if (band_count > 3u) {
        raw_signal = band_peak_at(band_query(x, 3u));
    }
    raw_signal = clamp(raw_signal * preview_gain, 0.0, 1.0);
    var low_signal = 0.0;
    if (band_count > 0u) {
        low_signal = projected_band_peak(band_query(x, 0u));
    }
    low_signal = clamp(low_signal * preview_gain, 0.0, 1.0);
    var mid_signal = 0.0;
    if (band_count > 1u) {
        mid_signal = projected_band_peak(band_query(x, 1u));
    }
    mid_signal = clamp(mid_signal * preview_gain, 0.0, 1.0);
    var high_signal = 0.0;
    if (band_count > 2u) {
        high_signal = projected_band_peak(band_query(x, 2u));
    }
    high_signal = clamp(high_signal * preview_gain, 0.0, 1.0);
    var white_signal = high_signal;
    if (band_count > 2u) {
        let neighbor_span = 1.0 / max(params.dest.z, 1.0) * 1.15;
        white_signal = max(
            white_signal,
            max(
                projected_band_peak(band_query(x - neighbor_span, 2u)) * preview_gain,
                projected_band_peak(band_query(x + neighbor_span, 2u)) * preview_gain,
            ),
        );
    }
    return signal_visual_fragment(
        SignalVisualInput(
            in.local,
            params.dest,
            presentation.cursor.x,
            presentation.cursor.y,
            presentation.cursor_color,
            band_count,
        ),
        SignalVisualBands(low_signal, mid_signal, high_signal, raw_signal, white_signal),
    );
}
