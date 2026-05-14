//! Backend-neutral GPU surface content contracts.

use crate::gui::types::{ImageRgba, Rect};
use std::fmt;
use std::sync::Arc;

use super::GpuSignalSummary;

/// Backend-neutral retained GPU surface content.
#[derive(Clone, Debug, PartialEq)]
pub enum GpuSurfaceContent {
    /// Shared RGBA atlas payload sampled from a source rectangle.
    RgbaAtlas {
        /// Source rectangle in atlas-pixel coordinates.
        source_rect: Rect,
        /// Shared RGBA atlas payload uploaded once per key/revision by native backends.
        atlas: Arc<ImageRgba>,
    },
    /// Interleaved floating-point signal bands rendered directly at surface resolution.
    SignalBands {
        /// Total frame count in the retained signal data.
        frames: usize,
        /// Number of interleaved bands per frame.
        band_count: usize,
        /// Visible frame range as start/end frame offsets.
        frame_range: [f32; 2],
        /// Interleaved frame-major band samples.
        samples: Arc<[f32]>,
    },
    /// Interleaved floating-point signal summaries rendered directly at surface resolution.
    SignalSummaryBands {
        /// Total frame count in the retained signal data.
        frames: usize,
        /// Number of interleaved bands per frame.
        band_count: usize,
        /// Visible frame range as start/end frame offsets.
        frame_range: [f32; 2],
        /// Precomputed min/max summary pyramid.
        summary: Arc<GpuSignalSummary>,
    },
}

/// Renderable shape resolved from a retained GPU signal payload.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuSignalRenderShape {
    /// Effective source frame count available to the renderer.
    pub frames: usize,
    /// Number of interleaved bands per frame.
    pub band_count: usize,
    /// Visible frame range as start/end frame offsets.
    pub frame_range: [f32; 2],
    /// Number of source sample or summary bucket entries.
    pub sample_count: usize,
}

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
}

impl GpuSurfaceContent {
    /// Validate this retained GPU-surface payload and return typed diagnostics.
    pub fn validate(&self) -> Result<(), GpuSurfaceContentError> {
        match self {
            Self::RgbaAtlas { source_rect, atlas } => {
                validate_atlas_source_rect(*source_rect, atlas.width, atlas.height)
            }
            Self::SignalBands {
                frames,
                band_count,
                frame_range,
                samples,
            } => validate_signal_render_shape(
                *frames,
                *band_count,
                *frame_range,
                samples.len() / (*band_count).max(1),
                samples.len(),
            )
            .map(|_| ()),
            Self::SignalSummaryBands {
                frames,
                band_count,
                frame_range,
                summary,
            } => {
                validate_signal_summary_shape(*frames, *band_count, summary)?;
                let sample_count = summary
                    .levels
                    .iter()
                    .map(|level| level.buckets.len())
                    .max()
                    .unwrap_or_default();
                validate_signal_render_shape(
                    *frames,
                    *band_count,
                    *frame_range,
                    summary.frames,
                    sample_count,
                )
                .map(|_| ())
            }
        }
    }

    /// Return whether this payload is valid enough for a backend to render.
    pub fn is_renderable(&self) -> bool {
        self.validate().is_ok()
    }

    /// Resolve the renderer-facing shape for signal payloads.
    pub fn signal_render_shape(&self) -> Option<GpuSignalRenderShape> {
        match self {
            Self::SignalBands {
                frames,
                band_count,
                frame_range,
                samples,
            } => validate_signal_render_shape(
                *frames,
                *band_count,
                *frame_range,
                samples.len() / (*band_count).max(1),
                samples.len(),
            )
            .ok(),
            Self::SignalSummaryBands {
                frames,
                band_count,
                frame_range,
                summary,
            } => {
                if validate_signal_summary_shape(*frames, *band_count, summary).is_err() {
                    return None;
                }
                let sample_count = summary
                    .levels
                    .iter()
                    .map(|level| level.buckets.len())
                    .max()
                    .unwrap_or_default();
                validate_signal_render_shape(
                    *frames,
                    *band_count,
                    *frame_range,
                    summary.frames,
                    sample_count,
                )
                .ok()
            }
            Self::RgbaAtlas { .. } => None,
        }
    }
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
        }
    }
}

impl std::error::Error for GpuSurfaceContentError {}

fn validate_atlas_source_rect(
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
    if !source_rect.min.x.is_finite()
        || !source_rect.min.y.is_finite()
        || !source_rect.max.x.is_finite()
        || !source_rect.max.y.is_finite()
    {
        return Err(GpuSurfaceContentError::NonFiniteAtlasSourceRect { source_rect });
    }
    if source_rect.width() <= 0.0 || source_rect.height() <= 0.0 {
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

fn validate_signal_render_shape(
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

fn validate_signal_summary_shape(
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
    for (level_index, level) in summary.levels.iter().enumerate() {
        let bucket_count = level.buckets.len();
        if bucket_count < band_count || bucket_count % band_count != 0 {
            return Err(GpuSurfaceContentError::InvalidSignalSummaryLevel {
                level_index,
                bucket_count,
                band_count,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "content/tests.rs"]
mod tests;
