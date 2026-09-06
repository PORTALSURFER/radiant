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

#[test]
fn cancellable_summary_stops_before_work() {
    let mut probes = 0;
    let summary =
        GpuSignalSummary::from_interleaved_samples_cancellable(&[0.0; 2048], 2048, 1, || {
            probes += 1;
            true
        });

    assert!(summary.is_none());
    assert_eq!(probes, 1);
}

#[test]
fn cancellable_summary_stops_during_base_level() {
    let mut probes = 0;
    let summary =
        GpuSignalSummary::from_interleaved_samples_cancellable(&[0.0; 2048], 2048, 1, || {
            probes += 1;
            probes >= 4
        });

    assert!(summary.is_none());
    assert_eq!(probes, 4, "the second 1024-bucket base chunk cancels");
}

#[test]
fn cancellable_summary_stops_during_merge_level() {
    let mut probes = 0;
    let summary =
        GpuSignalSummary::from_interleaved_samples_cancellable(&[0.0; 4096], 4096, 1, || {
            probes += 1;
            probes >= 9
        });

    assert!(summary.is_none());
    assert_eq!(probes, 9, "the second 1024-bucket merge chunk cancels");
}

#[test]
fn cancellable_summary_does_not_publish_after_a_late_cancellation() {
    let mut probes = 0;
    let summary = GpuSignalSummary::from_interleaved_samples_cancellable(&[0.5], 1, 1, || {
        probes += 1;
        probes >= 4
    });

    assert!(summary.is_none());
    assert_eq!(probes, 4, "the final probe prevents ready publication");
}

#[test]
fn cancellable_summary_matches_legacy_output_without_cancellation() {
    let samples = [
        0.25,
        f32::NAN,
        -0.5,
        f32::INFINITY,
        0.75,
        -1.0,
        f32::NEG_INFINITY,
        0.0,
        0.125,
        -0.25,
        0.5,
        -0.75,
        1.0,
        -0.125,
        0.625,
    ];
    let legacy = GpuSignalSummary::from_interleaved_samples(&samples, 5, 3);
    let cancellable =
        GpuSignalSummary::from_interleaved_samples_cancellable(&samples, 5, 3, || false)
            .expect("an always-false cancellation probe returns the completed summary");

    assert_eq!(cancellable, legacy);
}
