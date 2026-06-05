//! CPU-built summary pyramids for retained GPU signal surfaces.

use std::sync::Arc;

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
#[repr(C)]
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
        let mut levels = Vec::with_capacity(signal_summary_level_count(frames));
        let mut bucket_frames = 1usize;
        let mut previous_buckets: Option<Arc<[GpuSignalSummaryBucket]>> = None;
        while bucket_frames <= frames.max(1) {
            let buckets = match previous_buckets.as_deref() {
                Some(previous) => {
                    merge_signal_summary_level(previous, frames, band_count, bucket_frames)
                }
                None => build_signal_summary_base_level(samples, frames, band_count),
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
        let upper = self
            .levels
            .partition_point(|level| (level.bucket_frames as f32) <= target);
        if upper == 0 {
            return 0;
        }
        if upper >= self.levels.len() {
            return self.levels.len().saturating_sub(1);
        }

        let lower = upper - 1;
        let lower_delta = (self.levels[lower].bucket_frames as f32 - target).abs();
        let upper_delta = (self.levels[upper].bucket_frames as f32 - target).abs();
        if lower_delta <= upper_delta {
            lower
        } else {
            upper
        }
    }
}

fn signal_summary_level_count(frames: usize) -> usize {
    let frames = frames.max(1);
    usize::BITS as usize - frames.leading_zeros() as usize
}

fn build_signal_summary_base_level(
    samples: &[f32],
    frames: usize,
    band_count: usize,
) -> Arc<[GpuSignalSummaryBucket]> {
    if frames == 0 {
        return vec![GpuSignalSummaryBucket::default(); band_count].into();
    }
    let sample_count = frames.saturating_mul(band_count);
    let mut buckets = Vec::with_capacity(sample_count);
    for value in samples.iter().copied().take(sample_count) {
        if value.is_finite() {
            buckets.push(GpuSignalSummaryBucket {
                min: value,
                max: value,
            });
        } else {
            buckets.push(GpuSignalSummaryBucket::default());
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
        let first_offset = first.saturating_mul(band_count);
        let second_offset = first_offset.saturating_add(band_count);
        for band in 0..band_count {
            let mut summary = previous
                .get(first_offset + band)
                .copied()
                .unwrap_or_default();
            if second < previous_bucket_count
                && let Some(next) = previous.get(second_offset + band)
            {
                summary.min = summary.min.min(next.min);
                summary.max = summary.max.max(next.max);
            }
            buckets.push(summary);
        }
    }
    buckets.into()
}

#[cfg(test)]
#[path = "signal_summary/tests.rs"]
mod tests;
