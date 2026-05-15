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
    if width == 0 || height == 0 || !rect.has_finite_positive_area() {
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
    let source = match source_rect {
        Some(source) if !source.has_finite_positive_area() => return,
        Some(source) => source.clamp_to(full_source),
        None => full_source,
    };
    let Some(transform) = image_transform(rect, source) else {
        return;
    };
    if source_rect.is_some() {
        scene.push_clip_layer(Fill::NonZero, Affine::IDENTITY, &to_kurbo_rect(rect));
    }
    scene.draw_image(&image_data, transform);
    if source_rect.is_some() {
        scene.pop_layer();
    }
}

fn image_transform(rect: UiRect, source: UiRect) -> Option<Affine> {
    if !rect.has_finite_positive_area() || !source.has_finite_positive_area() {
        return None;
    }
    let scale_x = rect.width() as f64 / source.width() as f64;
    let scale_y = rect.height() as f64 / source.height() as f64;
    if !scale_x.is_finite() || !scale_y.is_finite() {
        return None;
    }
    Some(
        Affine::translate((
            rect.min.x as f64 - source.min.x as f64 * scale_x,
            rect.min.y as f64 - source.min.y as f64 * scale_y,
        )) * Affine::scale_non_uniform(scale_x, scale_y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_transform_rejects_nonfinite_or_empty_geometry() {
        let rect = UiRect::from_min_max(Point::new(10.0, 20.0), Point::new(30.0, 60.0));
        let source = UiRect::from_min_max(Point::new(2.0, 4.0), Point::new(12.0, 24.0));
        let empty = UiRect::from_min_max(Point::new(0.0, 0.0), Point::new(0.0, 1.0));
        let nonfinite = UiRect::from_min_max(Point::new(f32::NAN, 0.0), Point::new(1.0, 1.0));

        assert!(image_transform(rect, source).is_some());
        assert!(image_transform(empty, source).is_none());
        assert!(image_transform(nonfinite, source).is_none());
        assert!(image_transform(rect, empty).is_none());
        assert!(image_transform(rect, nonfinite).is_none());
    }
}
