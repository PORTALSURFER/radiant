use super::{ImageRgba, ImageRgbaError};
use std::sync::Arc;

#[test]
fn image_rgba_exposes_read_only_dimensions_and_pixels() {
    let pixels: Arc<[u8]> = vec![1, 2, 3, 4, 5, 6, 7, 8].into();
    let image = ImageRgba::try_from_shared(2, 1, Arc::clone(&pixels)).expect("valid image");

    assert_eq!(image.width(), 2);
    assert_eq!(image.height(), 1);
    assert_eq!(image.pixels(), pixels.as_ref());
    assert!(Arc::ptr_eq(image.shared_pixels(), &pixels));
}

#[test]
fn image_rgba_try_new_reports_length_mismatch() {
    let error = ImageRgba::try_new(2, 2, vec![255; 15]).expect_err("invalid byte count");

    assert_eq!(
        error,
        ImageRgbaError {
            width: 2,
            height: 2,
            actual_len: 15,
            expected_len: Some(16),
        }
    );
    assert_eq!(
        error.to_string(),
        "invalid RGBA image 2x2: expected 16 bytes, got 15"
    );
    assert!(ImageRgba::new(2, 2, vec![255; 15]).is_none());
}

#[test]
fn image_rgba_try_new_reports_dimension_overflow() {
    let error = ImageRgba::try_new(usize::MAX, 2, Vec::new()).expect_err("overflowing size");

    assert_eq!(error.expected_len, None);
    assert_eq!(
        error.to_string(),
        format!(
            "invalid RGBA image {}x2: byte length overflows usize",
            usize::MAX
        )
    );
}
