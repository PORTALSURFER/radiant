//! Backend-neutral GPU surface content contracts.

use crate::gui::types::{ImageRgba, Rect};
use std::sync::Arc;

use super::GpuSignalSummary;
mod error;
mod model;
mod validation;
pub use error::GpuSurfaceContentError;
pub use model::{
    GpuShaderSurfaceDescriptor, GpuShaderSurfaceDescriptorParts, GpuSignalGainPreview,
    GpuSignalRenderShape,
};
use validation::{
    validate_atlas, validate_shader_descriptor, validate_signal_gain_preview,
    validate_signal_render_shape, validate_signal_summary_shape,
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
        /// Optional circular frame offset preview. Positive offsets move signal
        /// content later in the visible timeline and wrap around source bounds.
        sample_slide_frame_offset: i64,
    },
    /// Opaque custom shader payload routed through the normal GPU-surface path.
    CustomShader {
        /// Backend-neutral shader identity and payload descriptor.
        descriptor: Arc<GpuShaderSurfaceDescriptor>,
    },
}

impl GpuSurfaceContent {
    /// Validate this retained GPU-surface payload and return typed diagnostics.
    pub fn validate(&self) -> Result<(), GpuSurfaceContentError> {
        match self {
            Self::RgbaAtlas { source_rect, atlas } => validate_atlas(atlas, *source_rect),
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
                gain_preview,
                sample_slide_frame_offset: _,
            } => {
                validate_signal_summary_shape(*frames, *band_count, summary)?;
                validate_signal_gain_preview(*gain_preview)?;
                let sample_count = summary_sample_count(summary);
                validate_signal_render_shape(
                    *frames,
                    *band_count,
                    *frame_range,
                    summary.frames,
                    sample_count,
                )
                .map(|_| ())
            }
            Self::CustomShader { descriptor } => validate_shader_descriptor(descriptor),
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
                gain_preview,
                sample_slide_frame_offset: _,
            } => {
                if validate_signal_summary_shape(*frames, *band_count, summary).is_err() {
                    return None;
                }
                if validate_signal_gain_preview(*gain_preview).is_err() {
                    return None;
                }
                let sample_count = summary_sample_count(summary);
                validate_signal_render_shape(
                    *frames,
                    *band_count,
                    *frame_range,
                    summary.frames,
                    sample_count,
                )
                .ok()
            }
            Self::RgbaAtlas { .. } | Self::CustomShader { .. } => None,
        }
    }
}

fn summary_sample_count(summary: &GpuSignalSummary) -> usize {
    summary
        .levels
        .iter()
        .map(|level| level.buckets.len())
        .max()
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "content/tests.rs"]
mod tests;
