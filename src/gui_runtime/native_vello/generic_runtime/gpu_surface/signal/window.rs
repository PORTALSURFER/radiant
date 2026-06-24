use crate::runtime::{GpuSignalSummaryBucket, GpuSignalSummaryLevel};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SignalBucketWindow {
    pub(super) start: usize,
    pub(super) end: usize,
}

pub(super) fn signal_bucket_window(
    frame_range: [f32; 2],
    level: &GpuSignalSummaryLevel,
    band_count: usize,
) -> Option<SignalBucketWindow> {
    let band_count = band_count.max(1);
    let bucket_count = level.buckets.len() / band_count;
    if bucket_count == 0 {
        return None;
    }
    let bucket_frames = level.bucket_frames.max(1) as f32;
    let start = floor_bucket_index(frame_range[0], bucket_frames, bucket_count);
    let end = ceil_bucket_index(frame_range[1], bucket_frames, bucket_count)
        .max(start + 1)
        .min(bucket_count);
    Some(SignalBucketWindow { start, end })
}

fn floor_bucket_index(frame: f32, bucket_frames: f32, bucket_count: usize) -> usize {
    bucket_index(frame, bucket_frames, bucket_count, f32::floor, 0.0)
        .min(bucket_count.saturating_sub(1))
}

fn ceil_bucket_index(frame: f32, bucket_frames: f32, bucket_count: usize) -> usize {
    bucket_index(frame, bucket_frames, bucket_count, f32::ceil, 1.0).min(bucket_count)
}

fn bucket_index(
    frame: f32,
    bucket_frames: f32,
    bucket_count: usize,
    round: fn(f32) -> f32,
    min_index: f32,
) -> usize {
    if !frame.is_finite() {
        return min_index as usize;
    }
    round(frame.max(0.0) / bucket_frames.max(1.0)).clamp(min_index, bucket_count as f32) as usize
}

impl SignalBucketWindow {
    pub(super) fn bucket_count(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub(super) fn sample_count(self, band_count: usize) -> usize {
        self.bucket_count().saturating_mul(band_count.max(1))
    }

    pub(super) fn buckets(
        self,
        level: &GpuSignalSummaryLevel,
        band_count: usize,
    ) -> &[GpuSignalSummaryBucket] {
        let band_count = band_count.max(1);
        let start = self.start.saturating_mul(band_count);
        let end = self.end.saturating_mul(band_count).min(level.buckets.len());
        &level.buckets[start..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(
        bucket_frames: usize,
        bucket_count: usize,
        band_count: usize,
    ) -> GpuSignalSummaryLevel {
        GpuSignalSummaryLevel {
            bucket_frames,
            buckets: vec![
                GpuSignalSummaryBucket::default();
                bucket_count.saturating_mul(band_count)
            ]
            .into(),
        }
    }

    #[test]
    fn signal_bucket_window_uploads_only_visible_buckets() {
        let level = level(4, 32, 2);

        let window =
            signal_bucket_window([16.0, 80.0], &level, 2).expect("visible range has buckets");

        assert_eq!(window, SignalBucketWindow { start: 4, end: 20 });
        assert_eq!(window.bucket_count(), 16);
        assert_eq!(window.buckets(&level, 2).len(), 32);
    }

    #[test]
    fn signal_bucket_window_clamps_to_summary_bounds() {
        let level = level(8, 4, 3);

        let window =
            signal_bucket_window([24.0, 96.0], &level, 3).expect("clamped range has buckets");

        assert_eq!(window, SignalBucketWindow { start: 3, end: 4 });
        assert_eq!(window.buckets(&level, 3).len(), 3);
    }
}
