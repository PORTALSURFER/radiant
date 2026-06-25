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

struct SignalSummaryWindow {
    start: f32,
    visible: f32,
    bucket_frames: f32,
    bucket_count: u32,
    bucket_offset: f32,
    frames: f32,
};

struct SignalBandQuery {
    x: f32,
    band: u32,
    band_count: u32,
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
