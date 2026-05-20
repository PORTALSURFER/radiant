//! Backend-neutral custom shader GPU surface composed through Radiant builders.

use radiant::prelude::*;
use std::sync::Arc;

#[derive(Default)]
struct DemoState;

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {}

fn shader_descriptor() -> Arc<GpuShaderSurfaceDescriptor> {
    Arc::new(
        GpuShaderSurfaceDescriptor::new("demo/custom-meter")
            .entry_point("main")
            .uniform_bytes([16, 32, 48, 64, 80, 96, 112, 128])
            .storage_bytes([3; 128])
            .vertex_count(6),
    )
}

fn demo_view(_state: &DemoState) -> View<DemoMessage> {
    column([
        text("Custom shader surface").size(260.0, 28.0),
        gpu_surface(
            91,
            4,
            GpuSurfaceContent::CustomShader {
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
    fn custom_shader_surface_example_lowers_to_gpu_surface_primitive() {
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
            panic!("example should emit a custom shader GPU surface primitive");
        };
        assert_eq!(gpu.widget_id, 21);
        assert_eq!(gpu.key, 91);
        assert_eq!(gpu.revision, 4);
        assert_eq!(gpu.rect.width(), 360.0);
        assert_eq!(gpu.rect.height(), 140.0);
        let GpuSurfaceContent::CustomShader { descriptor } = &gpu.content else {
            panic!("expected custom shader content");
        };
        assert_eq!(descriptor.shader_key, "demo/custom-meter");
        assert_eq!(descriptor.entry_point, "main");
        assert_eq!(
            descriptor.uniform_bytes.as_ref(),
            &[16, 32, 48, 64, 80, 96, 112, 128]
        );
        assert_eq!(descriptor.storage_bytes.len(), 128);
        assert_eq!(descriptor.vertex_count, 6);
    }

    #[test]
    fn custom_shader_surface_example_descriptor_is_valid() {
        let content = GpuSurfaceContent::CustomShader {
            descriptor: shader_descriptor(),
        };

        assert_eq!(content.validate(), Ok(()));
    }
}
