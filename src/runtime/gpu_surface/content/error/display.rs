use super::GpuSurfaceContentError;
use std::fmt;

impl fmt::Display for GpuSurfaceContentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAtlas { width, height } => {
                write!(
                    formatter,
                    "invalid GPU atlas {width}x{height}: atlas is empty"
                )
            }
            Self::NonFiniteAtlasSourceRect { source_rect } => write!(
                formatter,
                "invalid GPU atlas source rect {source_rect:?}: coordinates must be finite"
            ),
            Self::EmptyAtlasSourceRect { source_rect } => write!(
                formatter,
                "invalid GPU atlas source rect {source_rect:?}: rect must have positive area"
            ),
            Self::AtlasSourceRectOutOfBounds {
                source_rect,
                atlas_width,
                atlas_height,
            } => write!(
                formatter,
                "invalid GPU atlas source rect {source_rect:?}: outside atlas {atlas_width}x{atlas_height}"
            ),
            Self::InvalidSignalBandCount => {
                write!(
                    formatter,
                    "invalid GPU signal content: band count must be positive"
                )
            }
            Self::EmptySignal {
                frames,
                available_frames,
            } => write!(
                formatter,
                "invalid GPU signal content: {frames} declared frames but {available_frames} available frames"
            ),
            Self::InvalidSignalFrameRange { frame_range } => write!(
                formatter,
                "invalid GPU signal frame range [{}, {}]: range must be finite and positive",
                frame_range[0], frame_range[1]
            ),
            Self::SignalSummaryShapeMismatch {
                frames,
                band_count,
                summary_frames,
                summary_band_count,
            } => write!(
                formatter,
                "invalid GPU signal summary: declared {frames} frames/{band_count} bands but summary has {summary_frames} frames/{summary_band_count} bands"
            ),
            Self::EmptySignalSummary => {
                write!(formatter, "invalid GPU signal summary: no summary levels")
            }
            Self::InvalidSignalSummaryLevel {
                level_index,
                bucket_count,
                band_count,
            } => write!(
                formatter,
                "invalid GPU signal summary level {level_index}: {bucket_count} buckets are not complete {band_count}-band groups"
            ),
            Self::InvalidSignalGainPreview { preview } => write!(
                formatter,
                "invalid GPU signal gain preview {preview:?}: preview values must be finite"
            ),
            Self::EmptyShaderKey => {
                write!(
                    formatter,
                    "invalid GPU shader surface: shader key must not be empty"
                )
            }
            Self::EmptyShaderEntryPoint { shader_key } => write!(
                formatter,
                "invalid GPU shader surface {shader_key}: vertex entry point must not be empty"
            ),
            Self::EmptyShaderFragmentEntryPoint { shader_key } => write!(
                formatter,
                "invalid GPU shader surface {shader_key}: fragment entry point must not be empty when provided"
            ),
            Self::MissingShaderFragmentEntryPoint { shader_key } => write!(
                formatter,
                "invalid GPU shader surface {shader_key}: WGSL source requires a fragment entry point for direct rendering"
            ),
            Self::EmptyShaderSource { shader_key } => write!(
                formatter,
                "invalid GPU shader surface {shader_key}: WGSL source must not be empty when provided"
            ),
            Self::EmptyShaderVertexCount { shader_key } => write!(
                formatter,
                "invalid GPU shader surface {shader_key}: vertex count must be positive"
            ),
        }
    }
}

impl std::error::Error for GpuSurfaceContentError {}
