//! Typed diagnostics for retained GPU-surface content validation.

use super::GpuSignalGainPreview;
use crate::gui::types::Rect;
use std::fmt;

/// Error returned when retained GPU-surface content cannot be rendered safely.
#[derive(Clone, Debug, PartialEq)]
pub enum GpuSurfaceContentError {
    /// An RGBA atlas has zero width or height.
    EmptyAtlas {
        /// Atlas width in pixels.
        width: usize,
        /// Atlas height in pixels.
        height: usize,
    },
    /// The RGBA atlas source rectangle contains a non-finite coordinate.
    NonFiniteAtlasSourceRect {
        /// Invalid source rectangle.
        source_rect: Rect,
    },
    /// The RGBA atlas source rectangle has no positive area.
    EmptyAtlasSourceRect {
        /// Invalid source rectangle.
        source_rect: Rect,
    },
    /// The RGBA atlas source rectangle extends outside the atlas bounds.
    AtlasSourceRectOutOfBounds {
        /// Invalid source rectangle.
        source_rect: Rect,
        /// Atlas width in pixels.
        atlas_width: usize,
        /// Atlas height in pixels.
        atlas_height: usize,
    },
    /// Signal content declared zero interleaved bands.
    InvalidSignalBandCount,
    /// Signal content has no renderable source frames.
    EmptySignal {
        /// Declared frame count.
        frames: usize,
        /// Available source frame count after payload validation.
        available_frames: usize,
    },
    /// Signal content has an invalid visible frame range.
    InvalidSignalFrameRange {
        /// Invalid visible frame range.
        frame_range: [f32; 2],
    },
    /// A signal-summary payload does not match the declared frame or band shape.
    SignalSummaryShapeMismatch {
        /// Declared frame count.
        frames: usize,
        /// Declared band count.
        band_count: usize,
        /// Summary frame count.
        summary_frames: usize,
        /// Summary band count.
        summary_band_count: usize,
    },
    /// A signal-summary payload contains no valid summary levels.
    EmptySignalSummary,
    /// A signal-summary level does not contain complete interleaved band buckets.
    InvalidSignalSummaryLevel {
        /// Invalid summary level index.
        level_index: usize,
        /// Number of buckets in the invalid level.
        bucket_count: usize,
        /// Expected interleaved band count.
        band_count: usize,
    },
    /// A GPU signal gain preview contains a non-finite control value.
    InvalidSignalGainPreview {
        /// Invalid preview controls.
        preview: GpuSignalGainPreview,
    },
}

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
        }
    }
}

impl std::error::Error for GpuSurfaceContentError {}
