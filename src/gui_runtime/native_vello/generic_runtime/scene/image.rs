use super::super::GenericSharedPixelBytes;
use crate::gui_runtime::native_vello::*;

pub(in crate::gui_runtime::native_vello::generic_runtime::scene) fn encode_image(
    scene: &mut Scene,
    pixels: Arc<[u8]>,
    image_width: usize,
    image_height: usize,
    source_rect: Option<UiRect>,
    rect: UiRect,
) {
    let (Ok(width), Ok(height)) = (u32::try_from(image_width), u32::try_from(image_height)) else {
        return;
    };
    if width == 0 || height == 0 || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let image_data = ImageData {
        data: Blob::new(Arc::new(GenericSharedPixelBytes(pixels))),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::Alpha,
        width,
        height,
    };
    let full_source = UiRect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(width as f32, height as f32),
    );
    let source = source_rect.unwrap_or(full_source).clamp_to(full_source);
    if source.width() <= 0.0 || source.height() <= 0.0 {
        return;
    }
    let transform = Affine::translate((
        rect.min.x as f64 - source.min.x as f64 * rect.width() as f64 / source.width() as f64,
        rect.min.y as f64 - source.min.y as f64 * rect.height() as f64 / source.height() as f64,
    )) * Affine::scale_non_uniform(
        rect.width() as f64 / source.width() as f64,
        rect.height() as f64 / source.height() as f64,
    );
    if source_rect.is_some() {
        scene.push_clip_layer(Fill::NonZero, Affine::IDENTITY, &to_kurbo_rect(rect));
    }
    scene.draw_image(&image_data, transform);
    if source_rect.is_some() {
        scene.pop_layer();
    }
}
