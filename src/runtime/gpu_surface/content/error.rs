//! Typed diagnostics for retained GPU-surface content validation.

use super::GpuSignalGainPreview;
use crate::gui::types::Rect;

mod display;

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
    /// A signal-summary level has a zero or non-ascending bucket width.
    InvalidSignalSummaryLevelWidth {
        /// Invalid summary level index.
        level_index: usize,
        /// Invalid source-frame width represented by one bucket.
        bucket_frames: usize,
        /// Previous level width when this is not the first level.
        previous_bucket_frames: Option<usize>,
    },
    /// A signal-summary level has the wrong number of interleaved buckets.
    InvalidSignalSummaryLevelBucketCount {
        /// Invalid summary level index.
        level_index: usize,
        /// Source-frame width represented by one bucket.
        bucket_frames: usize,
        /// Number of interleaved buckets supplied by the host.
        bucket_count: usize,
        /// Exact bucket count required by the declared frame and band shape.
        expected_bucket_count: usize,
    },
    /// A signal-summary bucket contains non-finite or reversed extrema.
    InvalidSignalSummaryBucketExtrema {
        /// Invalid summary level index.
        level_index: usize,
        /// Interleaved bucket index inside the level.
        bucket_index: usize,
        /// Invalid minimum value.
        min: f32,
        /// Invalid maximum value.
        max: f32,
    },
    /// A GPU signal gain preview contains a non-finite control value.
    InvalidSignalGainPreview {
        /// Invalid preview controls.
        preview: GpuSignalGainPreview,
    },
    /// A custom shader descriptor has no stable shader identity.
    EmptyShaderKey,
    /// A custom shader descriptor has no shader entry point.
    EmptyShaderEntryPoint {
        /// Stable application-defined shader or pipeline identity.
        shader_key: String,
    },
    /// A custom shader descriptor has an empty fragment shader entry point.
    EmptyShaderFragmentEntryPoint {
        /// Stable application-defined shader or pipeline identity.
        shader_key: String,
    },
    /// A custom shader descriptor carried WGSL source without a fragment entry point.
    MissingShaderFragmentEntryPoint {
        /// Stable application-defined shader or pipeline identity.
        shader_key: String,
    },
    /// A custom shader descriptor carried empty WGSL source.
    EmptyShaderSource {
        /// Stable application-defined shader or pipeline identity.
        shader_key: String,
    },
    /// A custom shader descriptor requested no drawable vertices.
    EmptyShaderVertexCount {
        /// Stable application-defined shader or pipeline identity.
        shader_key: String,
    },
}
