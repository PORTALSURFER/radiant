use super::super::*;

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
