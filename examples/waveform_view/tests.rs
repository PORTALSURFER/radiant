use super::*;
use radiant::runtime::{RuntimeBridge, SurfaceRuntime, TransientOverlayContext};
use std::path::PathBuf;

fn synthetic_file(mono_samples: Vec<f32>, sample_rate: u32, channels: usize) -> WaveformFile {
    waveform_file_from_mono_samples(
        PathBuf::from("synthetic-test-waveform"),
        sample_rate,
        channels,
        mono_samples,
    )
}

#[test]
fn stereo_samples_downmix_to_single_mono_stream() {
    let mono = downmix_to_mono(&[1.0, -1.0, 0.6, 0.2], 2, 2);

    assert_eq!(mono, vec![0.0, 0.4]);
}

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

#[test]
fn synthetic_waveform_renders_nonblank_mono_image() {
    let mono_samples: Vec<f32> = (0..512)
        .map(|index| ((index as f32 / 16.0).sin() * 0.8).clamp(-1.0, 1.0))
        .collect();
    let file = synthetic_file(mono_samples, 48_000, 2);

    let image = render_waveform_image(&file, WaveformViewport::full(file.frames), 128, 48);

    assert_eq!(image.width, 128);
    assert_eq!(image.height, 48);
    assert!(
        image
            .pixels
            .chunks_exact(4)
            .any(|pixel| pixel[0] > 240 && pixel[1] > 180 && pixel[2] > 150),
        "waveform ridge should produce visible bright pixels"
    );
}

#[test]
fn waveform_widget_paints_cached_body_and_cursor_overlay() {
    let mono_samples: Vec<f32> = (0..512)
        .map(|index| ((index as f32 / 16.0).sin() * 0.8).clamp(-1.0, 1.0))
        .collect();
    let file = Arc::new(synthetic_file(mono_samples, 48_000, 2));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(320.0, 96.0));
    let mut widget = WaveformWidget::new(
        Arc::clone(&file),
        WaveformViewport::full(file.frames),
        Some(0.42),
    );
    let mut primitives = Vec::new();

    assert_eq!(
        widget.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(160.0, 48.0)
            }
        ),
        None,
        "hover cursor updates should stay local to the widget"
    );
    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                capabilities,
                ..
            }) if capabilities.runtime_overlays.pointer_vertical_line.is_some()
                && capabilities.fast_pointer_move
                && capabilities.coalesce_vertical_wheel
        )),
        "waveform body should use a GPU signal primitive so zoom does not regenerate pixels"
    );
    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                overlays,
                ..
            }) if matches!(
                overlays.as_slice(),
                [GpuSurfaceOverlay::VerticalCursor { ratio, .. }] if (*ratio - 0.42).abs() < 0.001
            )
        )),
        "playhead/cursor state should render as a lightweight GPU-surface overlay"
    );
    assert!(
        !primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::FillPolygon(_)))
    );
    assert!(
        !primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::StrokePolyline(_))),
        "cursor line should be handled by the GPU waveform shader"
    );
}

#[test]
fn waveform_widget_omits_cursor_overlay_when_absent() {
    let mono_samples: Vec<f32> = (0..512)
        .map(|index| ((index as f32 / 16.0).sin() * 0.8).clamp(-1.0, 1.0))
        .collect();
    let file = Arc::new(synthetic_file(mono_samples, 48_000, 2));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(320.0, 96.0));
    let widget = WaveformWidget::new(Arc::clone(&file), WaveformViewport::full(file.frames), None);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                overlays,
                ..
            }) if overlays.is_empty()
        )),
        "non-playing waveform surfaces should not carry a playhead overlay"
    );
}

#[test]
fn waveform_playback_uses_paint_only_transient_playhead() {
    let mono_samples: Vec<f32> = (0..512)
        .map(|index| ((index as f32 / 16.0).sin() * 0.8).clamp(-1.0, 1.0))
        .collect();
    let file = Arc::new(synthetic_file(mono_samples, 48_000, 2));
    let mut runtime = SurfaceRuntime::new(
        radiant::app(WaveformApp {
            file,
            viewport: WaveformViewport::full(512),
            zoom_anchor_ratio: 0.5,
            playing: true,
            playhead_ratio: 0.25,
        })
        .view(view)
        .animated_transient_overlay_at(
            60,
            |state| state.playing,
            |state, context, primitives| {
                paint_playhead_overlay(state, context.plan, context.animation_time, primitives);
            },
        )
        .update_with(|state, message, context| {
            state.apply_interaction(message);
            context.request_repaint();
        })
        .into_bridge(),
        Vector2::new(1280.0, 560.0),
    );

    let activity = runtime.bridge_mut().animation_activity();
    assert!(activity.needs_animation());
    assert!(!activity.needs_frame_message());
    assert!(
        !runtime.bridge_mut().queue_animation_frame(),
        "waveform playhead animation should not enqueue app frame messages"
    );

    let plan = runtime.paint_plan(&ThemeTokens::default());
    let mut primitives = Vec::new();
    runtime.bridge_mut().paint_transient_overlay(
        TransientOverlayContext::new(
            &plan,
            Vector2::new(1280.0, 560.0),
            Duration::from_millis(500),
        ),
        &mut primitives,
    );

    assert!(
        primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRect(fill) if fill.widget_id == WAVEFORM_WIDGET_ID
                && fill.rect.height() > 0.0
        )),
        "paint-only playback should append a playhead over the cached waveform paint plan"
    );
}

#[test]
fn zoom_and_pan_keep_viewport_inside_sample() {
    let mono_samples = vec![0.0; 20_000];
    let file = Arc::new(synthetic_file(mono_samples, 48_000, 1));
    let mut app = WaveformApp {
        file,
        viewport: WaveformViewport::full(20_000),
        zoom_anchor_ratio: 0.5,
        playing: false,
        playhead_ratio: 0.5,
    };

    app.zoom_around_anchor(0.5, 0.5);
    assert!(app.viewport.visible_frames() < 20_000);
    app.pan_by_visible_fraction(100.0);
    assert_eq!(app.viewport.end, 20_000);
    app.pan_by_visible_fraction(-100.0);
    assert_eq!(app.viewport.start, 0);
}

#[test]
fn wheel_zoom_and_scrollbar_offset_update_viewport() {
    let mono_samples = vec![0.0; 20_000];
    let file = Arc::new(synthetic_file(mono_samples, 48_000, 1));
    let mut app = WaveformApp {
        file,
        viewport: WaveformViewport::full(20_000),
        zoom_anchor_ratio: 0.5,
        playing: false,
        playhead_ratio: 0.5,
    };

    app.apply_interaction(WaveformInteraction::Wheel {
        delta: Vector2::new(0.0, -40.0),
        anchor_ratio: 0.25,
    });
    assert!(app.viewport.is_zoomed_in(20_000));

    app.apply_interaction(WaveformInteraction::ScrollTo {
        offset_fraction: 1.0,
    });
    assert_eq!(app.viewport.end, 20_000);
}

#[test]
fn zoom_around_anchor_keeps_anchor_frame_at_same_ratio() {
    let mono_samples = vec![0.0; 20_000];
    let file = Arc::new(synthetic_file(mono_samples, 48_000, 1));
    let mut app = WaveformApp {
        file,
        viewport: WaveformViewport {
            start: 2_000,
            end: 12_000,
        },
        zoom_anchor_ratio: 0.5,
        playing: false,
        playhead_ratio: 0.5,
    };
    let ratio = 0.25;
    let before_anchor = app.viewport.start as f32 + app.viewport.visible_frames() as f32 * ratio;

    app.zoom_around_anchor(0.5, ratio);

    let after_anchor = app.viewport.start as f32 + app.viewport.visible_frames() as f32 * ratio;
    assert!((before_anchor - after_anchor).abs() <= 1.0);
}

#[test]
fn summary_stats_match_raw_range_stats() {
    let samples: Vec<f32> = (0..4096)
        .map(|index| ((index as f32 / 13.0).sin() * 0.7).clamp(-1.0, 1.0))
        .collect();
    let summary = WaveformSummary::from_samples(&samples);

    let summarized = summary.stats(&samples, 37, 3901);
    let raw = band_stats(&samples, 37, 3901);
    assert!((summarized.peak - raw.peak).abs() < 0.000_001);
    assert!((summarized.rms - raw.rms).abs() < 0.000_001);
}

#[test]
fn default_waveform_source_uses_synthetic_signal_without_input_path() {
    let file = load_waveform_source(None).expect("synthetic waveform should load");

    assert!(file.sample_rate > 0);
    assert!(!file.mono_samples.is_empty());
    assert_eq!(file.frames, file.mono_samples.len());
    let image = render_waveform_image(&file, WaveformViewport::full(file.frames), 320, 96);
    assert_eq!(image.width, 320);
}
