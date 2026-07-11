//! Validation helpers for retained GPU-surface content.

use super::{
    GpuShaderSurfaceDescriptor, GpuSignalGainPreview, GpuSignalRenderShape, GpuSurfaceContentError,
};
use crate::{gui::types::Rect, runtime::GpuSignalSummary};

pub(super) fn validate_atlas_source_rect(
    source_rect: Rect,
    atlas_width: usize,
    atlas_height: usize,
) -> Result<(), GpuSurfaceContentError> {
    if atlas_width == 0 || atlas_height == 0 {
        return Err(GpuSurfaceContentError::EmptyAtlas {
            width: atlas_width,
            height: atlas_height,
        });
    }
    if !source_rect.is_finite() {
        return Err(GpuSurfaceContentError::NonFiniteAtlasSourceRect { source_rect });
    }
    if !source_rect.has_finite_positive_area() {
        return Err(GpuSurfaceContentError::EmptyAtlasSourceRect { source_rect });
    }
    if source_rect.min.x < 0.0
        || source_rect.min.y < 0.0
        || source_rect.max.x > atlas_width as f32
        || source_rect.max.y > atlas_height as f32
    {
        return Err(GpuSurfaceContentError::AtlasSourceRectOutOfBounds {
            source_rect,
            atlas_width,
            atlas_height,
        });
    }
    Ok(())
}

pub(super) fn validate_signal_render_shape(
    requested_frames: usize,
    band_count: usize,
    frame_range: [f32; 2],
    available_frames: usize,
    sample_count: usize,
) -> Result<GpuSignalRenderShape, GpuSurfaceContentError> {
    if band_count == 0 {
        return Err(GpuSurfaceContentError::InvalidSignalBandCount);
    }
    let frames = requested_frames.min(available_frames);
    let visible = frame_range[1] - frame_range[0];
    if frames == 0 || sample_count < band_count {
        return Err(GpuSurfaceContentError::EmptySignal {
            frames: requested_frames,
            available_frames,
        });
    }
    if !frame_range[0].is_finite()
        || !frame_range[1].is_finite()
        || !visible.is_finite()
        || visible <= 0.0
    {
        return Err(GpuSurfaceContentError::InvalidSignalFrameRange { frame_range });
    }
    Ok(GpuSignalRenderShape {
        frames,
        band_count,
        frame_range,
        sample_count,
    })
}

pub(super) fn validate_signal_summary_shape(
    frames: usize,
    band_count: usize,
    summary: &GpuSignalSummary,
) -> Result<(), GpuSurfaceContentError> {
    if band_count == 0 {
        return Err(GpuSurfaceContentError::InvalidSignalBandCount);
    }
    if summary.frames != frames || summary.band_count != band_count {
        return Err(GpuSurfaceContentError::SignalSummaryShapeMismatch {
            frames,
            band_count,
            summary_frames: summary.frames,
            summary_band_count: summary.band_count,
        });
    }
    if summary.levels.is_empty() {
        return Err(GpuSurfaceContentError::EmptySignalSummary);
    }
    let mut previous_bucket_frames = None;
    for (level_index, level) in summary.levels.iter().enumerate() {
        if level.bucket_frames == 0
            || previous_bucket_frames.is_some_and(|previous| level.bucket_frames <= previous)
        {
            return Err(GpuSurfaceContentError::InvalidSignalSummaryLevelWidth {
                level_index,
                bucket_frames: level.bucket_frames,
                previous_bucket_frames,
            });
        }
        let bucket_count = level.buckets.len();
        let expected_bucket_count = frames
            .div_ceil(level.bucket_frames)
            .max(1)
            .saturating_mul(band_count);
        if bucket_count != expected_bucket_count {
            return Err(
                GpuSurfaceContentError::InvalidSignalSummaryLevelBucketCount {
                    level_index,
                    bucket_frames: level.bucket_frames,
                    bucket_count,
                    expected_bucket_count,
                },
            );
        }
        for (bucket_index, bucket) in level.buckets.iter().enumerate() {
            if !bucket.min.is_finite() || !bucket.max.is_finite() || bucket.min > bucket.max {
                return Err(GpuSurfaceContentError::InvalidSignalSummaryBucketExtrema {
                    level_index,
                    bucket_index,
                    min: bucket.min,
                    max: bucket.max,
                });
            }
        }
        previous_bucket_frames = Some(level.bucket_frames);
    }
    Ok(())
}

pub(super) fn validate_signal_gain_preview(
    preview: Option<GpuSignalGainPreview>,
) -> Result<(), GpuSurfaceContentError> {
    let Some(preview) = preview else {
        return Ok(());
    };
    let values = [
        preview.start,
        preview.end,
        preview.gain,
        preview.fade_in_length,
        preview.fade_in_curve,
        preview.fade_in_mute,
        preview.fade_in_outer_gain,
        preview.fade_out_length,
        preview.fade_out_curve,
        preview.fade_out_mute,
        preview.fade_out_outer_gain,
    ];
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(GpuSurfaceContentError::InvalidSignalGainPreview { preview })
    }
}

pub(super) fn validate_shader_descriptor(
    descriptor: &GpuShaderSurfaceDescriptor,
) -> Result<(), GpuSurfaceContentError> {
    if descriptor.shader_key.trim().is_empty() {
        return Err(GpuSurfaceContentError::EmptyShaderKey);
    }
    if descriptor.entry_point.trim().is_empty() {
        return Err(GpuSurfaceContentError::EmptyShaderEntryPoint {
            shader_key: descriptor.shader_key.clone(),
        });
    }
    if descriptor
        .fragment_entry_point
        .as_deref()
        .is_some_and(|entry_point| entry_point.trim().is_empty())
    {
        return Err(GpuSurfaceContentError::EmptyShaderFragmentEntryPoint {
            shader_key: descriptor.shader_key.clone(),
        });
    }
    if let Some(source) = descriptor.wgsl_source.as_deref() {
        if source.trim().is_empty() {
            return Err(GpuSurfaceContentError::EmptyShaderSource {
                shader_key: descriptor.shader_key.clone(),
            });
        }
        if descriptor.fragment_entry_point.is_none() {
            return Err(GpuSurfaceContentError::MissingShaderFragmentEntryPoint {
                shader_key: descriptor.shader_key.clone(),
            });
        }
    }
    if descriptor.vertex_count == 0 {
        return Err(GpuSurfaceContentError::EmptyShaderVertexCount {
            shader_key: descriptor.shader_key.clone(),
        });
    }
    Ok(())
}
