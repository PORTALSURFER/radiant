struct Params {
    dest: vec4<f32>,
    source: vec4<f32>,
    target_size: vec2<f32>,
    overlay_ratios: vec4<f32>,
    overlay_widths: vec4<f32>,
    overlay_colors: array<vec4<f32>, 4>,
};

@group(0) @binding(0)
var<uniform> params: Params;
@group(0) @binding(1)
var surface_texture: texture_2d<f32>;
@group(0) @binding(2)
var surface_sampler: sampler;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) uv: vec2<f32>,
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
    let texture_size = vec2<f32>(textureDimensions(surface_texture));
    let source_pixel = params.source.xy + local * params.source.zw;
    var out: VertexOut;
    out.position = vec4<f32>(clip, 0.0, 1.0);
    out.local = local;
    out.uv = source_pixel / texture_size;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var color = textureSample(surface_texture, surface_sampler, in.uv);
    for (var index = 0u; index < 4u; index = index + 1u) {
        let ratio = params.overlay_ratios[index];
        let half_width = max(params.overlay_widths[index] / max(params.dest.z, 1.0), 0.0005);
        if (ratio >= 0.0 && abs(in.local.x - ratio) <= half_width) {
            color = params.overlay_colors[index];
        }
    }
    return color;
}
