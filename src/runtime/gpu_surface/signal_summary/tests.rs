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

#[test]
fn signal_summary_base_level_maps_samples_without_merging() {
    let samples = [0.25, f32::NAN, -0.5, f32::INFINITY];
    let summary = GpuSignalSummary::from_interleaved_samples(&samples, 2, 2);

    assert_eq!(
        &summary.levels[0].buckets[..],
        &[
            GpuSignalSummaryBucket {
                min: 0.25,
                max: 0.25,
            },
            GpuSignalSummaryBucket::default(),
            GpuSignalSummaryBucket {
                min: -0.5,
                max: -0.5,
            },
            GpuSignalSummaryBucket::default(),
        ]
    );
}

#[test]
fn signal_summary_presizes_level_vector_for_power_of_two_pyramid() {
    let samples = [0.0; 16];
    let summary = GpuSignalSummary::from_interleaved_samples(&samples, 16, 1);

    assert_eq!(summary.levels.len(), 5);
    assert!(summary.levels.capacity() >= 5);
}

#[test]
fn signal_summary_presizes_level_vector_for_empty_input() {
    let summary = GpuSignalSummary::from_interleaved_samples(&[], 0, 2);

    assert_eq!(summary.levels.len(), 1);
    assert!(summary.levels.capacity() >= 1);
}

#[test]
fn signal_summary_level_lookup_uses_nearest_bucket_size() {
    let samples = (0..64).map(|index| index as f32).collect::<Vec<_>>();
    let summary = GpuSignalSummary::from_interleaved_samples(&samples, 64, 1);

    assert_eq!(summary.level_for_frames_per_pixel(0.5), 0);
    assert_eq!(summary.level_for_frames_per_pixel(1.49), 0);
    assert_eq!(summary.level_for_frames_per_pixel(1.5), 0);
    assert_eq!(summary.level_for_frames_per_pixel(1.51), 1);
    assert_eq!(
        summary.level_for_frames_per_pixel(f32::INFINITY),
        summary.levels.len() - 1
    );
}

#[test]
fn empty_signal_summary_level_lookup_defaults_to_zero() {
    let summary = GpuSignalSummary {
        frames: 0,
        band_count: 1,
        levels: Vec::new(),
    };

    assert_eq!(summary.level_for_frames_per_pixel(4.0), 0);
}
