use super::*;
use crate::gui::types::{ImageRgba, Point, Vector2};
use crate::runtime::{GpuSignalSummaryBucket, GpuSignalSummaryLevel};

#[test]
fn signal_render_shape_rejects_invalid_payload_contracts() {
    let samples: Arc<[f32]> = [0.0, 1.0].into();
    let invalid_band_count = GpuSurfaceContent::SignalBands {
        frames: 2,
        band_count: 0,
        frame_range: [0.0, 2.0],
        samples: Arc::clone(&samples),
    };
    let invalid_range = GpuSurfaceContent::SignalBands {
        frames: 2,
        band_count: 1,
        frame_range: [2.0, 2.0],
        samples,
    };

    assert!(!invalid_band_count.is_renderable());
    assert_eq!(invalid_band_count.signal_render_shape(), None);
    assert!(!invalid_range.is_renderable());
    assert_eq!(invalid_range.signal_render_shape(), None);
}

#[test]
fn signal_render_shape_uses_effective_available_frame_count() {
    let content = GpuSurfaceContent::SignalBands {
        frames: 8,
        band_count: 2,
        frame_range: [0.0, 8.0],
        samples: [0.0, 1.0, 0.5, -0.25].into(),
    };

    assert_eq!(
        content.signal_render_shape(),
        Some(GpuSignalRenderShape {
            frames: 2,
            band_count: 2,
            frame_range: [0.0, 8.0],
            sample_count: 4,
        })
    );
}

#[test]
fn signal_summary_payload_must_match_declared_shape() {
    let summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
        &[0.0, 1.0, -0.5, 0.25],
        2,
        2,
    ));
    let content = GpuSurfaceContent::SignalSummaryBands {
        frames: 2,
        band_count: 1,
        frame_range: [0.0, 2.0],
        summary,
        gain_preview: None,
    };

    assert!(!content.is_renderable());
    assert_eq!(content.signal_render_shape(), None);
}

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
fn gpu_surface_content_validation_reports_signal_errors() {
    let invalid_band_count = GpuSurfaceContent::SignalBands {
        frames: 2,
        band_count: 0,
        frame_range: [0.0, 2.0],
        samples: Arc::<[f32]>::from([0.0, 1.0]),
    };
    let invalid_range = GpuSurfaceContent::SignalBands {
        frames: 2,
        band_count: 1,
        frame_range: [2.0, 2.0],
        samples: Arc::<[f32]>::from([0.0, 1.0]),
    };

    assert_eq!(
        invalid_band_count.validate(),
        Err(GpuSurfaceContentError::InvalidSignalBandCount)
    );
    assert_eq!(
        invalid_range.validate(),
        Err(GpuSurfaceContentError::InvalidSignalFrameRange {
            frame_range: [2.0, 2.0],
        })
    );
}

#[test]
fn gpu_surface_content_validation_reports_summary_shape_errors() {
    let summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
        &[0.0, 1.0, -0.5, 0.25],
        2,
        2,
    ));
    let content = GpuSurfaceContent::SignalSummaryBands {
        frames: 2,
        band_count: 1,
        frame_range: [0.0, 2.0],
        summary,
        gain_preview: None,
    };

    assert_eq!(
        content.validate(),
        Err(GpuSurfaceContentError::SignalSummaryShapeMismatch {
            frames: 2,
            band_count: 1,
            summary_frames: 2,
            summary_band_count: 2,
        })
    );
}

#[test]
fn gpu_surface_content_validation_rejects_zero_band_summary_before_level_checks() {
    let content = GpuSurfaceContent::SignalSummaryBands {
        frames: 1,
        band_count: 0,
        frame_range: [0.0, 1.0],
        summary: Arc::new(GpuSignalSummary {
            frames: 1,
            band_count: 0,
            levels: vec![GpuSignalSummaryLevel {
                bucket_frames: 1,
                buckets: Arc::<[GpuSignalSummaryBucket]>::from([GpuSignalSummaryBucket::default()]),
            }],
        }),
        gain_preview: None,
    };

    assert_eq!(
        content.validate(),
        Err(GpuSurfaceContentError::InvalidSignalBandCount)
    );
}

#[test]
fn gpu_surface_content_validation_rejects_non_finite_gain_preview() {
    let summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
        &[0.0, 1.0],
        2,
        1,
    ));
    let preview = GpuSignalGainPreview {
        start: 0.0,
        end: 1.0,
        gain: f32::NAN,
        fade_in_length: 0.0,
        fade_in_curve: 0.5,
        fade_in_mute: 0.0,
        fade_out_length: 0.0,
        fade_out_curve: 0.5,
        fade_out_mute: 0.0,
    };
    let content = GpuSurfaceContent::SignalSummaryBands {
        frames: 2,
        band_count: 1,
        frame_range: [0.0, 2.0],
        summary,
        gain_preview: Some(preview),
    };

    match content.validate() {
        Err(GpuSurfaceContentError::InvalidSignalGainPreview { preview }) => {
            assert!(preview.gain.is_nan());
        }
        other => panic!("expected invalid gain preview, got {other:?}"),
    }
    assert_eq!(content.signal_render_shape(), None);
}
