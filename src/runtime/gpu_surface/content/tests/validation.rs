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

#[test]
fn custom_shader_content_validation_reports_descriptor_errors() {
    let empty_key = GpuSurfaceContent::CustomShader {
        descriptor: Arc::new(GpuShaderSurfaceDescriptor::new(" ")),
    };
    let empty_entry = GpuSurfaceContent::CustomShader {
        descriptor: Arc::new(GpuShaderSurfaceDescriptor::new("meter").entry_point("")),
    };
    let empty_source = GpuSurfaceContent::CustomShader {
        descriptor: Arc::new(GpuShaderSurfaceDescriptor::new("meter").wgsl_source(" ")),
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
        empty_source.validate(),
        Err(GpuSurfaceContentError::EmptyShaderSource {
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
            "@fragment fn fragment_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }",
        )
        .entry_point("fragment_main")
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
        source.contains("@fragment") && source.contains("fragment_main")
    }));
    assert_eq!(descriptor.entry_point, "fragment_main");
    assert_eq!(descriptor.uniform_bytes.as_ref(), &[1, 2, 3, 4]);
    assert_eq!(descriptor.storage_bytes.as_ref(), &[5, 6]);
    assert_eq!(descriptor.vertex_count, 6);
}
