//! Retained GPU surface composed through Radiant's application builders.

use radiant::gui::types::ImageRgba;
use radiant::layout::{Point, Rect, Vector2};
use radiant::prelude::*;
use std::sync::Arc;

fn demo_atlas() -> Arc<ImageRgba> {
    let width = 256usize;
    let height = 128usize;
    let mut pixels = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            let u = x as f32 / (width - 1) as f32;
            let v = y as f32 / (height - 1) as f32;
            let pulse = ((u * 10.0).sin() * 0.5 + 0.5) * (1.0 - (v - 0.5).abs() * 1.6);
            pixels.extend_from_slice(&[
                (24.0 + u * 42.0) as u8,
                (34.0 + pulse * 170.0) as u8,
                (48.0 + v * 110.0) as u8,
                255,
            ]);
        }
    }
    Arc::new(ImageRgba::new(width, height, pixels).expect("atlas dimensions match pixels"))
}

fn demo_view() -> View {
    let atlas = demo_atlas();
    column([
        text("GPU surface").size(180.0, 28.0),
        gpu_surface(
            7,
            1,
            GpuSurfaceContent::RgbaAtlas {
                source_rect: Rect::from_min_size(
                    Point::new(0.0, 0.0),
                    Vector2::new(atlas.width as f32, atlas.height as f32),
                ),
                atlas,
            },
        )
        .size(360.0, 180.0)
        .width(360.0)
        .height(180.0),
    ])
    .padding(24.0)
    .spacing(12.0)
    .align_cross(radiant::layout::CrossAlign::Start)
}

fn main() -> radiant::Result {
    radiant::window("Radiant GPU Surface")
        .size(440, 280)
        .run(demo_view())
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::runtime::PaintPrimitive;
    use radiant::theme::ThemeTokens;

    #[test]
    fn gpu_surface_example_lowers_to_retained_gpu_primitive() {
        let surface = demo_view().into_surface();
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
            panic!("example should emit a GPU surface primitive");
        };
        assert_eq!(gpu.key, 7);
        assert_eq!(gpu.revision, 1);
        assert_eq!(gpu.rect.width(), 360.0);
        assert_eq!(gpu.rect.height(), 180.0);
    }
}
