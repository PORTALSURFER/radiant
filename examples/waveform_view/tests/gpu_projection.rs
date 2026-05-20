use super::*;

#[test]
fn gpu_low_band_projection_avoids_thin_cuts_without_flattening_detail() {
    let frame_count = 65_536;
    let low_samples: Vec<f32> = (0..frame_count)
        .map(|index| {
            let carrier = (index as f32 / 34.0).sin();
            let contour = 0.28 + 0.58 * (index as f32 / 12_000.0).sin().abs();
            (carrier * contour).clamp(-1.0, 1.0)
        })
        .collect();
    let bands = [
        WaveformBand::new(low_samples.clone()),
        WaveformBand::new(vec![0.0; frame_count]),
        WaveformBand::new(vec![0.0; frame_count]),
        WaveformBand::new(vec![0.0; frame_count]),
    ];

    let gpu_samples = interleaved_band_samples(&bands);
    let low_gpu_samples: Vec<f32> = gpu_samples
        .chunks_exact(BAND_COUNT)
        .map(|frame| frame[0])
        .collect();
    let extents = shader_projected_band_extents(&low_gpu_samples, 192, 0);
    let isolated_cuts = isolated_cut_count(&extents);
    let isolated_spikes = isolated_spike_count(&extents);
    let roughness = extent_roughness(&extents);
    let max_step = max_adjacent_step(&extents);
    let detail_range = extent_range(&extents);

    assert!(
        isolated_cuts <= 1,
        "low-frequency projection should not contain repeated one-column zero-crossing cuts; extents: {extents:?}"
    );
    assert!(
        isolated_spikes <= 1,
        "low-frequency projection should not contain repeated one-column crest spikes; extents: {extents:?}"
    );
    assert!(
        roughness < 0.012,
        "low-frequency projection should stay continuous at full zoom-out"
    );
    assert!(
        max_step < 0.16,
        "low-frequency projection should not contain long vertical edge jumps"
    );
    assert!(
        detail_range > 0.18,
        "low-frequency projection should retain amplitude contour detail, not flatten into a rectangle"
    );
}

fn shader_projected_band_extents(samples: &[f32], columns: usize, _band: usize) -> Vec<f32> {
    let frames_per_pixel = samples.len() as f32 / columns.max(1) as f32;
    (0..columns)
        .map(|column| {
            let peak = smoothed_test_peak(samples, columns, column);
            let left = smoothed_test_peak(samples, columns, column.saturating_sub(1));
            let right = smoothed_test_peak(
                samples,
                columns,
                (column + 1).min(columns.saturating_sub(1)),
            );
            let neighbor = left.max(right);
            let corner_limit =
                0.24 + (0.095 - 0.24) * smoothstep_test(18.0, 260.0, frames_per_pixel);
            let corner_delta = (peak - neighbor).max(0.0);
            let corner_strength = smoothstep_test(corner_limit, corner_limit * 2.8, corner_delta);
            peak + (neighbor + corner_limit - peak) * corner_strength * 0.82
        })
        .collect()
}

fn smoothed_test_peak(samples: &[f32], columns: usize, column: usize) -> f32 {
    weighted_test_projection(samples, columns, column, test_peak_extent)
}

fn weighted_test_projection(
    samples: &[f32],
    columns: usize,
    column: usize,
    project: fn(&[f32], usize, usize) -> f32,
) -> f32 {
    let taps = [
        (column.saturating_sub(1), 0.24),
        (column, 0.52),
        ((column + 1).min(columns.saturating_sub(1)), 0.24),
    ];
    taps.iter()
        .map(|(tap, weight)| project(samples, columns, *tap) * weight)
        .sum()
}

fn test_peak_extent(samples: &[f32], columns: usize, column: usize) -> f32 {
    test_column_samples(samples, columns, column)
        .map(f32::abs)
        .fold(0.0_f32, f32::max)
}

fn test_column_samples(
    samples: &[f32],
    columns: usize,
    column: usize,
) -> impl Iterator<Item = f32> + '_ {
    let start = column * samples.len() / columns.max(1);
    let end = ((column + 1) * samples.len() / columns.max(1))
        .max(start + 1)
        .min(samples.len());
    let span = end.saturating_sub(start).max(1);
    let step = (span / 40).max(1);
    (start..end).step_by(step).map(|frame| samples[frame])
}

fn smoothstep_test(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn isolated_cut_count(extents: &[f32]) -> usize {
    extents
        .windows(3)
        .filter(|window| {
            let neighbor_floor = window[0].min(window[2]);
            neighbor_floor > 0.24 && window[1] < neighbor_floor * 0.54
        })
        .count()
}

fn isolated_spike_count(extents: &[f32]) -> usize {
    extents
        .windows(3)
        .filter(|window| {
            let neighbor_ceiling = window[0].max(window[2]);
            window[1] > 0.32 && window[1] > neighbor_ceiling * 1.42
        })
        .count()
}

fn extent_range(extents: &[f32]) -> f32 {
    let min = extents.iter().copied().fold(f32::INFINITY, f32::min);
    let max = extents.iter().copied().fold(0.0_f32, f32::max);
    max - min
}

fn extent_roughness(extents: &[f32]) -> f32 {
    if extents.len() < 3 {
        return 0.0;
    }
    let total = extents
        .windows(3)
        .map(|window| (window[1] * 2.0 - window[0] - window[2]).abs())
        .sum::<f32>();
    total / (extents.len() - 2) as f32
}

fn max_adjacent_step(extents: &[f32]) -> f32 {
    extents
        .windows(2)
        .map(|window| (window[1] - window[0]).abs())
        .fold(0.0_f32, f32::max)
}
