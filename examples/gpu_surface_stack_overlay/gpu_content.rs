use super::*;
use crate::view::{SURFACE_HEIGHT, SURFACE_WIDTH};

pub(super) fn demo_gpu_content() -> GpuSurfaceContent {
    let width = SURFACE_WIDTH as usize;
    let height = SURFACE_HEIGHT as usize;
    GpuSurfaceContent::RgbaAtlas {
        atlas: Arc::clone(demo_gpu_atlas()),
        source_rect: Rect::from_min_size(
            Point::new(0.0, 0.0),
            Vector2::new(width as f32, height as f32),
        ),
    }
}

fn demo_gpu_atlas() -> &'static Arc<ImageRgba> {
    static ATLAS: OnceLock<Arc<ImageRgba>> = OnceLock::new();
    ATLAS.get_or_init(build_demo_gpu_atlas)
}

fn build_demo_gpu_atlas() -> Arc<ImageRgba> {
    let width = SURFACE_WIDTH as usize;
    let height = SURFACE_HEIGHT as usize;
    let mut pixels = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        let center = height as f32 * 0.5;
        let wave = ((y as f32 - center).abs() / center).clamp(0.0, 1.0);
        for x in 0..width {
            let phase = x as f32 / width as f32;
            let trace = (phase * std::f32::consts::TAU * 12.0).sin().abs();
            let bright = wave < trace * 0.72 + 0.04;
            let shade = if bright {
                180
            } else {
                30 + (phase * 50.0) as u8
            };
            pixels.extend_from_slice(&[shade / 3, shade, shade.saturating_add(45), 255]);
        }
    }
    Arc::new(ImageRgba::new(width, height, pixels).expect("valid demo image"))
}
