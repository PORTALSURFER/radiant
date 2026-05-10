//! Backend-neutral retained GPU surface model.

use crate::gui::types::{ImageRgba, Rect, Rgba8};
use std::sync::Arc;

/// Runtime interaction capabilities for retained GPU surfaces.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuSurfaceCapabilities {
    /// Whether pointer motion inside this surface can update runtime-owned overlays
    /// without refreshing the projected app surface.
    pub fast_pointer_move: bool,
    /// Whether vertical wheel deltas over this surface can be coalesced until redraw.
    pub coalesce_vertical_wheel: bool,
    /// Optional native-runtime hover cursor policy for this surface.
    pub native_hover_cursor: Option<GpuHoverCursor>,
}

/// Native-runtime hover cursor styling for retained GPU surfaces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuHoverCursor {
    /// Cursor color.
    pub color: Rgba8,
    /// Cursor width in logical pixels.
    pub width: f32,
}

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

/// CPU-built min/max pyramid for retained GPU signal surfaces.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuSignalSummary {
    /// Total source frame count represented by the pyramid.
    pub frames: usize,
    /// Number of interleaved bands per frame.
    pub band_count: usize,
    /// Summary levels in ascending bucket size order.
    pub levels: Vec<GpuSignalSummaryLevel>,
}

/// One min/max pyramid level.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuSignalSummaryLevel {
    /// Number of source frames represented by one bucket.
    pub bucket_frames: usize,
    /// Interleaved bucket-major band summaries.
    pub buckets: Arc<[GpuSignalSummaryBucket]>,
}

/// Min/max summary for one band in one bucket.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GpuSignalSummaryBucket {
    /// Minimum sample value inside the bucket.
    pub min: f32,
    /// Maximum sample value inside the bucket.
    pub max: f32,
}

impl GpuSignalSummary {
    /// Build a retained min/max pyramid from interleaved frame-major band samples.
    pub fn from_interleaved_samples(samples: &[f32], frames: usize, band_count: usize) -> Self {
        let frames = frames.min(samples.len() / band_count.max(1));
        let band_count = band_count.max(1);
        let mut levels = Vec::new();
        let mut bucket_frames = 1usize;
        let mut previous_buckets: Option<Arc<[GpuSignalSummaryBucket]>> = None;
        while bucket_frames <= frames.max(1) {
            let buckets = match previous_buckets.as_deref() {
                Some(previous) => {
                    merge_signal_summary_level(previous, frames, band_count, bucket_frames)
                }
                None => build_signal_summary_level(samples, frames, band_count, bucket_frames),
            };
            levels.push(GpuSignalSummaryLevel {
                bucket_frames,
                buckets: Arc::clone(&buckets),
            });
            previous_buckets = Some(buckets);
            if bucket_frames >= frames.max(1) {
                break;
            }
            bucket_frames = bucket_frames.saturating_mul(2).max(bucket_frames + 1);
        }
        Self {
            frames,
            band_count,
            levels,
        }
    }

    /// Return the preferred level for the provided frames-per-pixel ratio.
    pub fn level_for_frames_per_pixel(&self, frames_per_pixel: f32) -> usize {
        let target = frames_per_pixel.max(1.0);
        self.levels
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| {
                let left_delta = (left.bucket_frames as f32 - target).abs();
                let right_delta = (right.bucket_frames as f32 - target).abs();
                left_delta
                    .partial_cmp(&right_delta)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(index, _)| index)
            .unwrap_or_default()
    }
}

fn build_signal_summary_level(
    samples: &[f32],
    frames: usize,
    band_count: usize,
    bucket_frames: usize,
) -> Arc<[GpuSignalSummaryBucket]> {
    let bucket_count = frames.div_ceil(bucket_frames.max(1)).max(1);
    let mut buckets = Vec::with_capacity(bucket_count.saturating_mul(band_count));
    for bucket in 0..bucket_count {
        let start = bucket.saturating_mul(bucket_frames).min(frames);
        let end = ((bucket + 1).saturating_mul(bucket_frames))
            .min(frames)
            .max(start + 1);
        for band in 0..band_count {
            let mut summary = GpuSignalSummaryBucket {
                min: f32::INFINITY,
                max: f32::NEG_INFINITY,
            };
            for frame in start..end {
                let value = samples
                    .get(frame.saturating_mul(band_count).saturating_add(band))
                    .copied()
                    .unwrap_or_default();
                summary.min = summary.min.min(value);
                summary.max = summary.max.max(value);
            }
            if !summary.min.is_finite() || !summary.max.is_finite() {
                summary = GpuSignalSummaryBucket::default();
            }
            buckets.push(summary);
        }
    }
    buckets.into()
}

fn merge_signal_summary_level(
    previous: &[GpuSignalSummaryBucket],
    frames: usize,
    band_count: usize,
    bucket_frames: usize,
) -> Arc<[GpuSignalSummaryBucket]> {
    let bucket_count = frames.div_ceil(bucket_frames.max(1)).max(1);
    let previous_bucket_count = previous.len() / band_count.max(1);
    let mut buckets = Vec::with_capacity(bucket_count.saturating_mul(band_count));
    for bucket in 0..bucket_count {
        let first = bucket.saturating_mul(2);
        let second = first + 1;
        for band in 0..band_count {
            let mut summary = previous
                .get(first.saturating_mul(band_count).saturating_add(band))
                .copied()
                .unwrap_or_default();
            if second < previous_bucket_count
                && let Some(next) =
                    previous.get(second.saturating_mul(band_count).saturating_add(band))
            {
                summary.min = summary.min.min(next.min);
                summary.max = summary.max.max(next.max);
            }
            buckets.push(summary);
        }
    }
    buckets.into()
}

/// Lightweight GPU-surface overlay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GpuSurfaceOverlay {
    /// Vertical cursor line positioned as a 0..1 ratio inside the destination rect.
    VerticalCursor {
        /// Horizontal cursor position as a 0..1 ratio inside the destination rect.
        ratio: f32,
        /// Cursor color.
        color: Rgba8,
        /// Cursor width in logical pixels.
        width: f32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_summary_merges_partial_higher_level_buckets() {
        let samples = [-0.1, 0.2, -0.7, 0.4, 0.3, -0.8, 0.9, -0.2, -0.5, 0.1];
        let summary = GpuSignalSummary::from_interleaved_samples(&samples, 5, 2);
        let level = summary
            .levels
            .iter()
            .find(|level| level.bucket_frames == 4)
            .expect("4-frame summary level");

        assert_eq!(
            &level.buckets[..],
            &[
                GpuSignalSummaryBucket {
                    min: -0.7,
                    max: 0.9
                },
                GpuSignalSummaryBucket {
                    min: -0.8,
                    max: 0.4
                },
                GpuSignalSummaryBucket {
                    min: -0.5,
                    max: -0.5
                },
                GpuSignalSummaryBucket { min: 0.1, max: 0.1 },
            ]
        );
    }
}
