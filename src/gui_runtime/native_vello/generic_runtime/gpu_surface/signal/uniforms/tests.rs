use super::*;
use crate::gui::types::Rect as UiRect;
use crate::gui_runtime::native_vello::generic_runtime::gpu_surface::gpu_surface_types::SignalBodyCacheKeyParts;
use crate::gui_runtime::native_vello::generic_runtime::gpu_surface::passes::surface_pixel_extent;
use crate::layout::{Point, Vector2};
use crate::runtime::{
    GpuSignalRenderShape, GpuSignalSummary, GpuSignalSummaryBucket, GpuSignalSummaryLevel,
};
use crate::theme::DpiScale;
use std::sync::Arc;

#[test]
fn signal_uniforms_group_shape_level_and_gain_preview() {
    let shape = GpuSignalRenderShape {
        frames: 128,
        band_count: 2,
        frame_range: [16.0, 80.0],
        sample_count: 8,
    };
    let level = GpuSignalSummaryLevel {
        bucket_frames: 4,
        buckets: Arc::from([GpuSignalSummaryBucket::default(); 8]),
    };
    let body_key = SignalBodyCacheKey::new(SignalBodyCacheKeyParts {
        revision: 9,
        extent: surface_pixel_extent(
            UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(96.0, 48.0)),
            DpiScale::default(),
        )
        .expect("finite rect has an extent"),
        frames: shape.frames,
        band_count: shape.band_count,
        frame_range: shape.frame_range,
        sample_count: level.buckets.len(),
        level_index: 1,
        gain_preview: None,
    });

    let source = SignalRenderSource {
        shape,
        summary: Arc::new(GpuSignalSummary {
            frames: shape.frames,
            band_count: shape.band_count,
            levels: vec![level.clone()],
        }),
        gain_preview: None,
    };
    let selected = SelectedSignalLevel {
        index: 1,
        level: &level,
    };

    let uniforms = signal_uniforms(&source, &selected, body_key);

    assert_eq!(uniforms.dest, [0.0, 0.0, 96.0, 48.0]);
    assert_eq!(uniforms.frame_range, [16.0, 80.0, 128.0, 2.0]);
    assert_eq!(uniforms.summary_meta, [4.0, 4.0, 1.0, 0.0]);
    assert_eq!(uniforms.gain_preview_a, [0.0; 4]);
}

#[test]
fn signal_gain_preview_uniforms_mark_active_preview() {
    let preview = GpuSignalGainPreview {
        start: 0.1,
        end: 0.8,
        gain: 0.75,
        fade_in_length: 0.25,
        fade_in_curve: 0.4,
        fade_in_mute: 0.0,
        fade_in_outer_gain: 1.0,
        fade_out_length: 0.2,
        fade_out_curve: 0.6,
        fade_out_mute: 0.1,
        fade_out_outer_gain: 0.5,
    };

    let uniforms = signal_gain_preview_uniforms(Some(preview));

    assert_eq!(uniforms[0], [1.0, 0.1, 0.8, 0.75]);
    assert_eq!(uniforms[1], [0.25, 0.4, 0.2, 0.6]);
    assert_eq!(uniforms[2], [0.0, 0.1, 1.0, 0.5]);
}
