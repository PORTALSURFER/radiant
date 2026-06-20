fn summary_peak(bucket: u32, band: u32, band_count: u32, bucket_count: u32) -> f32 {
    if (bucket >= bucket_count || band >= band_count) {
        return 0.0;
    }
    let index = (bucket * band_count + band) * 2u;
    let low = summary_values[index];
    let high = summary_values[index + 1u];
    return max(abs(low), abs(high));
}

fn band_query(x: f32, band: u32, band_count: u32) -> SignalBandQuery {
    return SignalBandQuery(x, band, band_count);
}

fn band_peak_at(query: SignalBandQuery, window: SignalSummaryWindow) -> f32 {
    let center = clamp(query.x, 0.0, 1.0);
    let frame = max(window.start + window.visible * center, 0.0);
    let bucket = u32(clamp(floor(frame / max(window.bucket_frames, 1.0)), 0.0, f32(window.bucket_count - 1u)));
    return summary_peak(bucket, query.band, query.band_count, window.bucket_count);
}

fn smoothed_band_peak(query: SignalBandQuery, window: SignalSummaryWindow) -> f32 {
    return band_peak_at(query, window);
}

fn projected_band_peak(query: SignalBandQuery, window: SignalSummaryWindow) -> f32 {
    return smoothed_band_peak(query, window);
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
            let outer_gain = clamp(params.gain_preview_c.z, 0.0, 1.0);
            return outer_gain * (1.0 - preview_curve_value(t, fade_in_curve));
        }
    }
    let fade_out_mute = max(params.gain_preview_c.y, 0.0);
    if (fade_out_mute > 0.0) {
        let fade_end = selection_end + width * fade_out_mute;
        if (position >= selection_end && position <= fade_end) {
            let t = clamp((position - selection_end) / max(fade_end - selection_end, 0.000001), 0.0, 1.0);
            let fade_out_curve = clamp(params.gain_preview_b.w, 0.0, 1.0);
            let outer_gain = clamp(params.gain_preview_c.w, 0.0, 1.0);
            return outer_gain * preview_curve_value(t, fade_out_curve);
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
