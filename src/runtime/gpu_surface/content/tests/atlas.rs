use super::super::*;
use crate::gui::types::{ImageRgba, Point, Vector2};

#[test]
fn rgba_atlas_source_rect_must_be_inside_atlas() {
    let atlas = Arc::new(ImageRgba::new(8, 4, vec![255; 8 * 4 * 4]).expect("valid atlas"));
    let valid = GpuSurfaceContent::RgbaAtlas {
        source_rect: Rect::from_min_size(Point::new(2.0, 1.0), Vector2::new(4.0, 2.0)),
        atlas: Arc::clone(&atlas),
    };
    let overflows = GpuSurfaceContent::RgbaAtlas {
        source_rect: Rect::from_min_size(Point::new(6.0, 1.0), Vector2::new(4.0, 2.0)),
        atlas: Arc::clone(&atlas),
    };
    let negative_origin = GpuSurfaceContent::RgbaAtlas {
        source_rect: Rect::from_min_size(Point::new(-1.0, 0.0), Vector2::new(4.0, 2.0)),
        atlas,
    };

    assert!(valid.is_renderable());
    assert_eq!(valid.validate(), Ok(()));
    assert!(!overflows.is_renderable());
    assert_eq!(
        overflows.validate(),
        Err(GpuSurfaceContentError::AtlasSourceRectOutOfBounds {
            source_rect: Rect::from_min_size(Point::new(6.0, 1.0), Vector2::new(4.0, 2.0)),
            atlas_width: 8,
            atlas_height: 4,
        })
    );
    assert!(!negative_origin.is_renderable());
}

#[test]
fn rgba_atlas_source_rect_rejects_invalid_geometry_before_bounds() {
    let atlas = Arc::new(ImageRgba::new(8, 4, vec![255; 8 * 4 * 4]).expect("valid atlas"));
    let non_finite = Rect::from_min_size(Point::new(f32::INFINITY, 0.0), Vector2::new(4.0, 2.0));
    let inverted = Rect::from_min_max(Point::new(4.0, 1.0), Point::new(2.0, 3.0));

    assert_eq!(
        (GpuSurfaceContent::RgbaAtlas {
            source_rect: non_finite,
            atlas: Arc::clone(&atlas),
        })
        .validate(),
        Err(GpuSurfaceContentError::NonFiniteAtlasSourceRect {
            source_rect: non_finite,
        })
    );
    assert_eq!(
        (GpuSurfaceContent::RgbaAtlas {
            source_rect: inverted,
            atlas,
        })
        .validate(),
        Err(GpuSurfaceContentError::EmptyAtlasSourceRect {
            source_rect: inverted,
        })
    );
}

#[test]
fn rgba_atlas_rejects_short_long_and_overflowing_payloads_before_geometry() {
    let source_rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0));
    let cases = [
        (
            ImageRgba::from_parts_unchecked(2, 2, vec![255; 15]),
            GpuSurfaceContentError::InvalidAtlasByteLength {
                width: 2,
                height: 2,
                actual_len: 15,
                expected_len: Some(16),
            },
        ),
        (
            ImageRgba::from_parts_unchecked(2, 2, vec![255; 17]),
            GpuSurfaceContentError::InvalidAtlasByteLength {
                width: 2,
                height: 2,
                actual_len: 17,
                expected_len: Some(16),
            },
        ),
        (
            ImageRgba::from_parts_unchecked(usize::MAX, 2, Vec::new()),
            GpuSurfaceContentError::InvalidAtlasByteLength {
                width: usize::MAX,
                height: 2,
                actual_len: 0,
                expected_len: None,
            },
        ),
    ];

    for (atlas, expected_error) in cases {
        let content = GpuSurfaceContent::RgbaAtlas {
            source_rect,
            atlas: Arc::new(atlas),
        };
        assert_eq!(content.validate(), Err(expected_error));
        assert!(!content.is_renderable());
    }
}
