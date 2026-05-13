//! Backend-neutral GPU surface content contracts.

use crate::gui::types::{ImageRgba, Rect};
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

impl GpuSurfaceContent {
    /// Return whether this payload is valid enough for a backend to render.
    pub fn is_renderable(&self) -> bool {
        match self {
            Self::RgbaAtlas { source_rect, atlas } => atlas_source_rect_is_renderable(
                *source_rect,
                atlas.width as f32,
                atlas.height as f32,
            ),
            Self::SignalBands { .. } | Self::SignalSummaryBands { .. } => {
                self.signal_render_shape().is_some()
            }
        }
    }

    /// Resolve the renderer-facing shape for signal payloads.
    pub fn signal_render_shape(&self) -> Option<GpuSignalRenderShape> {
        match self {
            Self::SignalBands {
                frames,
                band_count,
                frame_range,
                samples,
            } => signal_render_shape(
                *frames,
                *band_count,
                *frame_range,
                samples.len() / (*band_count).max(1),
                samples.len(),
            ),
            Self::SignalSummaryBands {
                frames,
                band_count,
                frame_range,
                summary,
            } => {
                if summary.frames != *frames
                    || summary.band_count != *band_count
                    || summary.levels.is_empty()
                    || summary.levels.iter().any(|level| {
                        level.buckets.len() < *band_count || level.buckets.len() % *band_count != 0
                    })
                {
                    return None;
                }
                let sample_count = summary
                    .levels
                    .iter()
                    .map(|level| level.buckets.len())
                    .max()
                    .unwrap_or_default();
                signal_render_shape(
                    *frames,
                    *band_count,
                    *frame_range,
                    summary.frames,
                    sample_count,
                )
            }
            Self::RgbaAtlas { .. } => None,
        }
    }
}

fn atlas_source_rect_is_renderable(source_rect: Rect, atlas_width: f32, atlas_height: f32) -> bool {
    atlas_width > 0.0
        && atlas_height > 0.0
        && source_rect.min.x.is_finite()
        && source_rect.min.y.is_finite()
        && source_rect.max.x.is_finite()
        && source_rect.max.y.is_finite()
        && source_rect.width() > 0.0
        && source_rect.height() > 0.0
        && source_rect.min.x >= 0.0
        && source_rect.min.y >= 0.0
        && source_rect.max.x <= atlas_width
        && source_rect.max.y <= atlas_height
}

fn signal_render_shape(
    requested_frames: usize,
    band_count: usize,
    frame_range: [f32; 2],
    available_frames: usize,
    sample_count: usize,
) -> Option<GpuSignalRenderShape> {
    let frames = requested_frames.min(available_frames);
    let visible = frame_range[1] - frame_range[0];
    (frames > 0
        && band_count > 0
        && sample_count >= band_count
        && frame_range[0].is_finite()
        && frame_range[1].is_finite()
        && visible.is_finite()
        && visible > 0.0)
        .then_some(GpuSignalRenderShape {
            frames,
            band_count,
            frame_range,
            sample_count,
        })
}

#[cfg(test)]
#[path = "content/tests.rs"]
mod tests;
