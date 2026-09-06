use super::*;
use crate::runtime::GpuSignalSummary;

fn samples(frames: usize, bands: usize) -> Vec<f32> {
    (0..frames * bands)
        .map(|i| (i as f32 * 0.17).sin())
        .collect()
}

fn tile_request(
    first_frame: usize,
    bucket_frames: usize,
    bucket_count: usize,
    wrap: bool,
) -> BoundedSignalTileRequest {
    BoundedSignalTileRequest {
        first_frame,
        bucket_frames,
        bucket_count,
        wrap,
    }
}

fn scalar_bucket(
    samples: &[f32],
    frames: usize,
    bands: usize,
    request: BoundedSignalTileRequest,
    bucket: usize,
    band: usize,
) -> GpuSignalSummaryBucket {
    let mut value: Option<GpuSignalSummaryBucket> = None;
    for step in 0..request.bucket_frames {
        let frame = request.first_frame + bucket * request.bucket_frames + step;
        if !request.wrap && frame >= frames {
            break;
        }
        let sample = samples[(if request.wrap { frame % frames } else { frame }) * bands + band];
        let sample = if sample.is_finite() { sample } else { 0.0 };
        let entry = value.get_or_insert(GpuSignalSummaryBucket {
            min: sample,
            max: sample,
        });
        entry.min = entry.min.min(sample);
        entry.max = entry.max.max(sample);
    }
    value.unwrap_or_default()
}

#[test]
fn overview_matches_legacy_for_odd_sources_and_edge_spikes() {
    let mut raw = samples(8193, 2);
    raw[8192 * 2] = 99.0;
    raw[8192 * 2 + 1] = -99.0;

    let legacy = GpuSignalSummary::from_interleaved_samples(&raw, 8193, 2);
    let bounded = build_bounded_overview(&raw, 8193, 2, || false).unwrap();

    assert!(bounded.levels[0].bucket_frames > 1);
    for level in &bounded.levels {
        let legacy_level = legacy
            .levels
            .iter()
            .find(|legacy_level| legacy_level.bucket_frames == level.bucket_frames)
            .unwrap();
        assert_eq!(&*level.buckets, &*legacy_level.buckets);
    }
    assert!(bounded.logical_summary_bytes() <= MAX_BYTES);
    assert_eq!(
        bounded.logical_summary_bytes(),
        bounded_overview_bytes(8193, 2).unwrap()
    );
}

#[test]
fn nonfinite_samples_match_legacy_zero_sanitization() {
    let raw = [
        f32::NAN,
        0.5,
        f32::INFINITY,
        -0.5,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    let legacy = GpuSignalSummary::from_interleaved_samples(&raw, 6, 1);
    let overview = build_bounded_overview(&raw, 6, 1, || false).unwrap();
    let tile = build_bounded_tile(&raw, 6, 1, tile_request(0, 2, 3, false), || false).unwrap();

    for level in &overview.levels {
        let legacy_level = legacy
            .levels
            .iter()
            .find(|legacy_level| legacy_level.bucket_frames == level.bucket_frames)
            .unwrap();
        assert_eq!(level.buckets.as_ref(), legacy_level.buckets.as_ref());
    }
    assert_eq!(
        tile.buckets.as_ref(),
        [
            GpuSignalSummaryBucket { min: 0.0, max: 0.5 },
            GpuSignalSummaryBucket {
                min: -0.5,
                max: 0.0
            },
            GpuSignalSummaryBucket { min: 0.0, max: 0.0 },
        ]
    );
}

#[test]
fn tiles_match_scalar_reference_for_truncated_and_wrapped_ranges() {
    let raw = (0..10).map(|x| x as f32).collect::<Vec<_>>();
    let plain = build_bounded_tile(&raw, 10, 1, tile_request(8, 4, 1, false), || false).unwrap();
    assert_eq!(plain.first_frame, 8);
    assert_eq!(plain.source_frames, 10);
    assert_eq!(plain.bucket_frames, 4);
    assert_eq!(plain.buckets.len(), 1);
    assert_eq!(
        plain.buckets[0],
        scalar_bucket(&raw, 10, 1, tile_request(8, 4, 1, false), 0, 0)
    );

    let wrapped = build_bounded_tile(&raw, 10, 1, tile_request(8, 4, 2, true), || false).unwrap();
    for bucket in 0..2 {
        assert_eq!(
            wrapped.buckets[bucket],
            scalar_bucket(&raw, 10, 1, tile_request(8, 4, 2, true), bucket, 0)
        );
    }

    assert!(matches!(
        build_bounded_tile(&raw, 10, 1, tile_request(8, 4, 2, false), || false),
        Err(BoundedSignalError::InvalidRange)
    ));
}

#[test]
fn caps_allow_many_bands_when_bytes_fit_and_reject_products() {
    let raw = samples(2, 5);
    let tile = build_bounded_tile(&raw, 2, 5, tile_request(0, 1, 2, false), || false).unwrap();
    assert_eq!(tile.band_count, 5);
    assert_eq!(
        tile.logical_summary_bytes(),
        5 * 2 * std::mem::size_of::<GpuSignalSummaryBucket>()
    );

    let huge_tile = vec![0.0; 8192 * 5];
    assert!(matches!(
        build_bounded_tile(&huge_tile, 8192, 5, tile_request(0, 1, 8192, false), || {
            false
        }),
        Err(BoundedSignalError::Capacity)
    ));

    let too_many_bands = MAX_BYTES / std::mem::size_of::<GpuSignalSummaryBucket>() + 1;
    let overview_source = vec![0.0; too_many_bands];
    assert!(matches!(
        build_bounded_overview(&overview_source, 1, too_many_bands, || false),
        Err(BoundedSignalError::Capacity)
    ));
    assert!(matches!(
        bounded_overview_bytes(0, 1),
        Err(BoundedSignalError::InvalidShape)
    ));
}

#[test]
fn invalid_ranges_shapes_and_overflows_are_typed() {
    assert!(matches!(
        build_bounded_tile(&[], 0, 1, tile_request(0, 1, 1, false), || false),
        Err(BoundedSignalError::InvalidShape)
    ));
    let raw = [0.0];
    assert!(matches!(
        build_bounded_tile(&raw, 1, 1, tile_request(1, 1, 1, false), || false),
        Err(BoundedSignalError::InvalidRange)
    ));
    assert!(matches!(
        build_bounded_tile(&raw, 1, 1, tile_request(0, 3, 1, false), || false),
        Err(BoundedSignalError::Capacity)
    ));
    assert!(matches!(
        build_bounded_tile(&raw, 1, 1, tile_request(usize::MAX, 2, 1, true), || false),
        Err(BoundedSignalError::InvalidRange)
    ));
}

#[test]
fn cancellation_stops_inside_large_bucket_and_before_publication() {
    let raw = vec![0.5; 4096];
    let mut probes = 0;
    assert!(matches!(
        build_bounded_tile(&raw, 4096, 1, tile_request(0, 4096, 1, false), || {
            probes += 1;
            probes >= 3
        }),
        Err(BoundedSignalError::Cancelled)
    ));
    assert!(probes >= 3);

    let overview_source = vec![0.5; 4097];
    let mut overview_probes = 0;
    assert!(matches!(
        build_bounded_overview(&overview_source, 4097, 1, || {
            overview_probes += 1;
            overview_probes >= 8
        }),
        Err(BoundedSignalError::Cancelled)
    ));
    assert_eq!(overview_probes, 8);

    assert!(matches!(
        build_bounded_overview(&raw, 4096, 1, || true),
        Err(BoundedSignalError::Cancelled)
    ));
}
