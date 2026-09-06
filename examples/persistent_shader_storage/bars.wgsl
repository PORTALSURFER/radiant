
struct Params {
    dest: vec4<f32>,
    source: vec4<f32>,
    target_size: vec2<f32>,
    overlay_ratios: array<vec4<f32>, 2>,
    overlay_widths: array<vec4<f32>, 2>,
    overlay_colors: array<vec4<f32>, 8>,
};

@group(0) @binding(0)
var<uniform> params: Params;

@group(0) @binding(2) var<storage, read> values: array<f32>;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
};

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
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

@fragment
fn fragment_main(in: VertexOut) -> @location(0) vec4<f32> {
    let index = min(u32(in.local.x * 64.0), 63u);
    let value = clamp(values[index], 0.0, 1.0);
    if 1.0 - in.local.y <= value {
        return vec4<f32>(0.18, 0.75, 0.82, 1.0);
    }
    return vec4<f32>(0.04, 0.06, 0.08, 1.0);
}
