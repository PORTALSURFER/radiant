//! Backend-neutral GPU surface content contracts.

use crate::gui::types::{ImageRgba, Rect};
use std::sync::Arc;

use super::GpuSignalSummary;
mod error;
mod validation;
pub use error::GpuSurfaceContentError;
use validation::{
    validate_atlas_source_rect, validate_signal_render_shape, validate_signal_summary_shape,
};

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
        /// Optional gain envelope preview applied by the GPU renderer.
        gain_preview: Option<GpuSignalGainPreview>,
    },
}

/// Optional GPU-side gain envelope for retained signal rendering.
///
/// The preview is intentionally normalized and backend-neutral: hosts can
/// preview destructive fade/gain edits without rebuilding or re-uploading the
/// retained signal payload on each pointer update.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuSignalGainPreview {
    /// Normalized selection start.
    pub start: f32,
    /// Normalized selection end.
    pub end: f32,
    /// Gain applied inside the selection after fades.
    pub gain: f32,
    /// Fade-in length as a fraction of the selection width.
    pub fade_in_length: f32,
    /// Fade-in curve tension.
    pub fade_in_curve: f32,
    /// Fade-in outer extension length as a fraction of the selection width.
    ///
    /// Kept under the historical "mute" field name for API compatibility.
    pub fade_in_mute: f32,
    /// Fade-out length as a fraction of the selection width.
    pub fade_out_length: f32,
    /// Fade-out curve tension.
    pub fade_out_curve: f32,
    /// Fade-out outer extension length as a fraction of the selection width.
    ///
    /// Kept under the historical "mute" field name for API compatibility.
    pub fade_out_mute: f32,
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
                gain_preview: _,
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
                gain_preview: _,
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

#[cfg(test)]
#[path = "content/tests.rs"]
mod tests;
