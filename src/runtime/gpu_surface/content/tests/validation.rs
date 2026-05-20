use super::super::*;
use crate::runtime::{GpuSignalSummaryBucket, GpuSignalSummaryLevel};

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
