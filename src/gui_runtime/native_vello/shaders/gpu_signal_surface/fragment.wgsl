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
    var raw_signal = 0.0;
    if (band_count > 3u) {
        raw_signal = band_peak_at(band_query(in.local.x, 3u, band_count), summary_window);
    }
    raw_signal = clamp(raw_signal * preview_gain, 0.0, 1.0);
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
    var white_signal = high_signal;
    if (band_count > 2u) {
        let neighbor_span = pixel_width * 1.15;
        white_signal = max(
            white_signal,
            max(
                projected_band_peak(band_query(in.local.x - neighbor_span, 2u, band_count), summary_window) * preview_gain,
                projected_band_peak(band_query(in.local.x + neighbor_span, 2u, band_count), summary_window) * preview_gain,
            ),
        );
    }
    return signal_visual_fragment(
        SignalVisualInput(
            in.local,
            params.dest,
            params.cursor_ratio,
            params.cursor_width,
            params.cursor_color,
            band_count,
        ),
        SignalVisualBands(low_signal, mid_signal, high_signal, raw_signal, white_signal),
    );
}
