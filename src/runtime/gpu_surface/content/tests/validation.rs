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
        sample_slide_frame_offset: 0,
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

fn summary_content(frames: usize, levels: Vec<GpuSignalSummaryLevel>) -> GpuSurfaceContent {
    GpuSurfaceContent::SignalSummaryBands {
        frames,
        band_count: 1,
        frame_range: [0.0, frames as f32],
        summary: Arc::new(GpuSignalSummary {
            frames,
            band_count: 1,
            levels,
        }),
        gain_preview: None,
        sample_slide_frame_offset: 0,
    }
}

fn level(bucket_frames: usize, extrema: &[(f32, f32)]) -> GpuSignalSummaryLevel {
    GpuSignalSummaryLevel {
        bucket_frames,
        buckets: extrema
            .iter()
            .map(|(min, max)| GpuSignalSummaryBucket {
                min: *min,
                max: *max,
            })
            .collect::<Vec<_>>()
            .into(),
    }
}

#[test]
fn gpu_surface_content_validation_accepts_valid_multi_level_summary() {
    let content = summary_content(
        4,
        vec![
            level(1, &[(0.0, 0.1), (-0.2, 0.2), (-0.3, 0.4), (0.0, 0.5)]),
            level(2, &[(-0.2, 0.2), (-0.3, 0.5)]),
            level(4, &[(-0.3, 0.5)]),
        ],
    );

    assert_eq!(content.validate(), Ok(()));
}

#[test]
fn gpu_surface_content_validation_rejects_zero_and_nonascending_level_widths() {
    let zero = summary_content(1, vec![level(0, &[(0.0, 0.0)])]);
    assert_eq!(
        zero.validate(),
        Err(GpuSurfaceContentError::InvalidSignalSummaryLevelWidth {
            level_index: 0,
            bucket_frames: 0,
            previous_bucket_frames: None,
        })
    );

    let descending = summary_content(
        4,
        vec![
            level(2, &[(-1.0, 1.0), (-1.0, 1.0)]),
            level(1, &[(0.0, 0.0); 4]),
        ],
    );
    assert_eq!(
        descending.validate(),
        Err(GpuSurfaceContentError::InvalidSignalSummaryLevelWidth {
            level_index: 1,
            bucket_frames: 1,
            previous_bucket_frames: Some(2),
        })
    );
}

#[test]
fn gpu_surface_content_validation_rejects_wrong_summary_bucket_count() {
    let content = summary_content(4, vec![level(2, &[(-1.0, 1.0)])]);

    assert_eq!(
        content.validate(),
        Err(
            GpuSurfaceContentError::InvalidSignalSummaryLevelBucketCount {
                level_index: 0,
                bucket_frames: 2,
                bucket_count: 1,
                expected_bucket_count: 2,
            }
        )
    );
}

#[test]
fn gpu_surface_content_validation_rejects_non_finite_and_reversed_extrema() {
    for (min, max) in [(f32::NAN, 1.0), (-1.0, f32::INFINITY), (1.0, -1.0)] {
        let content = summary_content(1, vec![level(1, &[(min, max)])]);
        match content.validate() {
            Err(GpuSurfaceContentError::InvalidSignalSummaryBucketExtrema {
                level_index,
                bucket_index,
                min: actual_min,
                max: actual_max,
            }) => {
                assert_eq!(level_index, 0);
                assert_eq!(bucket_index, 0);
                assert!(actual_min.to_bits() == min.to_bits());
                assert!(actual_max.to_bits() == max.to_bits());
            }
            other => panic!("expected invalid extrema error, got {other:?}"),
        }
    }
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
        sample_slide_frame_offset: 0,
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
        fade_in_outer_gain: 1.0,
        fade_out_length: 0.0,
        fade_out_curve: 0.5,
        fade_out_mute: 0.0,
        fade_out_outer_gain: 1.0,
    };
    let content = GpuSurfaceContent::SignalSummaryBands {
        frames: 2,
        band_count: 1,
        frame_range: [0.0, 2.0],
        summary,
        gain_preview: Some(preview),
        sample_slide_frame_offset: 0,
    };

    match content.validate() {
        Err(GpuSurfaceContentError::InvalidSignalGainPreview { preview }) => {
            assert!(preview.gain.is_nan());
        }
        other => panic!("expected invalid gain preview, got {other:?}"),
    }
    assert_eq!(content.signal_render_shape(), None);
}

#[test]
fn custom_shader_content_validation_reports_descriptor_errors() {
    let empty_key = GpuSurfaceContent::CustomShader {
        descriptor: Arc::new(GpuShaderSurfaceDescriptor::new(" ")),
    };
    let empty_entry = GpuSurfaceContent::CustomShader {
        descriptor: Arc::new(GpuShaderSurfaceDescriptor::new("meter").entry_point("")),
    };
    let empty_fragment_entry = GpuSurfaceContent::CustomShader {
        descriptor: Arc::new(GpuShaderSurfaceDescriptor::new("meter").fragment_entry_point(" ")),
    };
    let empty_source = GpuSurfaceContent::CustomShader {
        descriptor: Arc::new(GpuShaderSurfaceDescriptor::new("meter").wgsl_source(" ")),
    };
    let source_without_fragment_entry = GpuSurfaceContent::CustomShader {
        descriptor: Arc::new(GpuShaderSurfaceDescriptor::new("meter").wgsl_source(
            "@vertex fn vertex_main() -> @builtin(position) vec4<f32> { return vec4<f32>(); }",
        )),
    };
    let empty_vertices = GpuSurfaceContent::CustomShader {
        descriptor: Arc::new(GpuShaderSurfaceDescriptor::new("meter").vertex_count(0)),
    };

    assert_eq!(
        empty_key.validate(),
        Err(GpuSurfaceContentError::EmptyShaderKey)
    );
    assert_eq!(
        empty_entry.validate(),
        Err(GpuSurfaceContentError::EmptyShaderEntryPoint {
            shader_key: String::from("meter"),
        })
    );
    assert_eq!(
        empty_fragment_entry.validate(),
        Err(GpuSurfaceContentError::EmptyShaderFragmentEntryPoint {
            shader_key: String::from("meter"),
        })
    );
    assert_eq!(
        empty_source.validate(),
        Err(GpuSurfaceContentError::EmptyShaderSource {
            shader_key: String::from("meter"),
        })
    );
    assert_eq!(
        source_without_fragment_entry.validate(),
        Err(GpuSurfaceContentError::MissingShaderFragmentEntryPoint {
            shader_key: String::from("meter"),
        })
    );
    assert_eq!(
        empty_vertices.validate(),
        Err(GpuSurfaceContentError::EmptyShaderVertexCount {
            shader_key: String::from("meter"),
        })
    );
}

#[test]
fn custom_shader_content_carries_backend_neutral_payloads() {
    let descriptor = GpuShaderSurfaceDescriptor::new("spectral-meter")
        .wgsl_source(
            "@vertex fn vertex_main() -> @builtin(position) vec4<f32> { return vec4<f32>(); }\n@fragment fn fragment_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }",
        )
        .entry_point("vertex_main")
        .fragment_entry_point("fragment_main")
        .uniform_bytes([1, 2, 3, 4])
        .storage_bytes([5, 6])
        .vertex_count(6);
    let content = GpuSurfaceContent::CustomShader {
        descriptor: Arc::new(descriptor),
    };

    assert!(content.is_renderable());
    assert_eq!(content.signal_render_shape(), None);
    let GpuSurfaceContent::CustomShader { descriptor } = content else {
        panic!("expected custom shader content");
    };
    assert_eq!(descriptor.shader_key, "spectral-meter");
    assert!(descriptor.wgsl_source.as_deref().is_some_and(|source| {
        source.contains("@vertex")
            && source.contains("vertex_main")
            && source.contains("@fragment")
            && source.contains("fragment_main")
    }));
    assert_eq!(descriptor.entry_point, "vertex_main");
    assert_eq!(
        descriptor.fragment_entry_point.as_deref(),
        Some("fragment_main")
    );
    assert_eq!(descriptor.uniform_bytes.as_ref(), &[1, 2, 3, 4]);
    assert_eq!(descriptor.storage_bytes.as_ref(), &[5, 6]);
    assert_eq!(descriptor.vertex_count, 6);
}
