//! Backend-neutral custom shader render canvas composed through Radiant builders.

use radiant::prelude::*;
use radiant::runtime::{GpuShaderSurfaceDescriptor, RenderCanvasContent, render_canvas};
use std::sync::Arc;

#[derive(Default)]
struct DemoState;

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {}

const DEMO_SHADER_WGSL: &str = r#"
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
    return vec4<f32>(0.16 + in.local.x * 0.28, 0.72, 0.82 - in.local.y * 0.24, 1.0);
}
"#;

fn shader_descriptor() -> Arc<GpuShaderSurfaceDescriptor> {
    Arc::new(
        GpuShaderSurfaceDescriptor::new("demo/custom-meter")
            .wgsl_source(DEMO_SHADER_WGSL)
            .entry_point("vertex_main")
            .fragment_entry_point("fragment_main")
            .vertex_count(6),
    )
}

fn demo_view(_state: &DemoState) -> View<DemoMessage> {
    column([
        text("Custom shader surface").size(260.0, 28.0),
        render_canvas(
            91,
            4,
            RenderCanvasContent::CustomShader {
                descriptor: shader_descriptor(),
            },
        )
        .id(21)
        .size(360.0, 140.0),
        text("Backends without a matching shader pipeline report this surface as unsupported.")
            .wrap()
            .size(360.0, 48.0),
    ])
    .padding(24.0)
    .spacing(12.0)
    .align_cross(radiant::layout::CrossAlign::Start)
}

fn main() -> radiant::Result {
    radiant::app(DemoState)
        .title("Radiant Custom Shader Surface")
        .size(440, 280)
        .view(|state| demo_view(state))
        .update(|_state, message| match message {})
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::layout::{Point, Rect, Vector2};
    use radiant::runtime::PaintPrimitive;
    use radiant::theme::ThemeTokens;

    #[test]
    fn custom_shader_surface_example_lowers_to_render_canvas_primitive() {
        let surface = demo_view(&DemoState).into_surface();
        let layout = radiant::layout::layout_tree(
            &surface.layout_node(),
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(440.0, 280.0)),
        );
        let plan = surface.paint_plan(&layout, &ThemeTokens::default());

        let gpu = plan
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::GpuSurface(surface) => Some(surface),
                _ => None,
            });

        let Some(gpu) = gpu else {
            panic!("example should emit a custom shader render canvas primitive");
        };
        assert_eq!(gpu.widget_id, 21);
        assert_eq!(gpu.key, 91);
        assert_eq!(gpu.revision, 4);
        assert_eq!(gpu.rect.width(), 360.0);
        assert_eq!(gpu.rect.height(), 140.0);
        let RenderCanvasContent::CustomShader { descriptor } = &gpu.content else {
            panic!("expected custom shader content");
        };
        assert_eq!(descriptor.shader_key, "demo/custom-meter");
        assert!(descriptor.wgsl_source.as_deref().is_some_and(|source| {
            source.contains("@vertex")
                && source.contains("vertex_index")
                && source.contains("@fragment")
                && source.contains("fragment_main")
        }));
        assert_eq!(descriptor.entry_point, "vertex_main");
        assert_eq!(
            descriptor.fragment_entry_point.as_deref(),
            Some("fragment_main")
        );
        assert!(descriptor.uniform_bytes.is_empty());
        assert!(descriptor.storage_bytes.is_empty());
        assert_eq!(descriptor.vertex_count, 6);
    }

    #[test]
    fn custom_shader_surface_example_descriptor_is_valid() {
        let content = RenderCanvasContent::CustomShader {
            descriptor: shader_descriptor(),
        };

        assert_eq!(content.validate(), Ok(()));
    }
}
