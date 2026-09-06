use super::*;

const FROZEN_LEGACY_SIGNAL_SHADER: &str =
    include_str!("fixtures/legacy_signal_shader_26102a7.wgsl");

#[test]
fn frozen_legacy_signal_shader_parses_as_wgsl() {
    naga::front::wgsl::parse_str(FROZEN_LEGACY_SIGNAL_SHADER)
        .expect("frozen 26102a7 legacy signal shader should remain a valid reference");
}

#[derive(Clone, Copy, Debug)]
enum PreciseSignalFixtureScenario {
    Base,
    AdjacentPan,
    FractionalZoom,
    SlideForward,
    SlideBackward,
    PartialGain,
}

#[test]
#[ignore = "requires native GPU adapter and offscreen WGPU rendering"]
fn precise_signal_window_matches_near_and_large_origins() {
    let (device, queue) = native_device();
    let buckets = precise_signal_fixture_buckets(64, 4);
    let scenarios = [
        PreciseSignalFixtureScenario::Base,
        PreciseSignalFixtureScenario::AdjacentPan,
        PreciseSignalFixtureScenario::FractionalZoom,
        PreciseSignalFixtureScenario::SlideForward,
        PreciseSignalFixtureScenario::SlideBackward,
        PreciseSignalFixtureScenario::PartialGain,
    ];

    let mut base_pixels = None;
    for scenario in scenarios {
        let near = precise_signal_fixture_pixels(&device, &queue, 0, 1, &buckets, scenario);
        let at_f32_limit =
            precise_signal_fixture_pixels(&device, &queue, 1_u64 << 24, 1, &buckets, scenario);
        let far =
            precise_signal_fixture_pixels(&device, &queue, 1_u64 << 40, 1, &buckets, scenario);
        assert_eq!(near, at_f32_limit, "near versus 2^24 for {scenario:?}");
        assert_eq!(near, far, "near versus 2^40 for {scenario:?}");
        if let Some(base) = base_pixels.as_ref() {
            assert_ne!(
                near, *base,
                "scenario should change the synthetic output: {scenario:?}"
            );
        } else {
            base_pixels = Some(near);
        }
    }

    let legacy = precise_signal_legacy_pixels(&device, &queue, &buckets, 1, [8.25, 24.25]);
    assert_eq!(
        base_pixels.expect("base scenario pixels"),
        legacy,
        "bucket-frame-one precise rendering should exactly match the legacy summary path"
    );
}

#[test]
#[ignore = "requires native GPU adapter and offscreen WGPU rendering"]
fn precise_signal_bucket_smoothing_matches_legacy_for_sub_bucket_views() {
    let (device, queue) = native_device();
    let buckets = precise_signal_fixture_buckets(64, 4);
    // Regression: with four-frame buckets and a two-frame viewport, legacy
    // uses `bucket_frames / max(visible_frames, 1)` = 2.0, not 1.0.
    let near = precise_signal_fixture_pixels(
        &device,
        &queue,
        0,
        4,
        &buckets,
        PreciseSignalFixtureScenario::Base,
    );
    let far = precise_signal_fixture_pixels(
        &device,
        &queue,
        1_u64 << 40,
        4,
        &buckets,
        PreciseSignalFixtureScenario::Base,
    );
    let legacy = precise_signal_legacy_pixels(&device, &queue, &buckets, 4, [8.25, 10.25]);
    assert_eq!(
        near, far,
        "sub-bucket view must be independent of exact origin"
    );
    assert_eq!(
        near, legacy,
        "precise smoothing must retain legacy bucket width"
    );
}

fn precise_signal_fixture_buckets(
    bucket_count: usize,
    band_count: usize,
) -> Vec<crate::runtime::GpuSignalSummaryBucket> {
    (0..bucket_count)
        .flat_map(|bucket| {
            (0..band_count).map(move |band| {
                let seed = ((bucket * 29 + band * 17 + 7) % 61) as f32 / 60.0;
                crate::runtime::GpuSignalSummaryBucket {
                    min: -seed * (0.20 + band as f32 * 0.09),
                    max: seed * (0.36 + band as f32 * 0.15),
                }
            })
        })
        .collect()
}

fn precise_signal_fixture_pixels(
    device: &vello::wgpu::Device,
    queue: &vello::wgpu::Queue,
    first_frame: u64,
    bucket_frames: u32,
    buckets: &[crate::runtime::GpuSignalSummaryBucket],
    scenario: PreciseSignalFixtureScenario,
) -> Vec<u8> {
    let source_frames = first_frame + u64::from(bucket_frames) * 64;
    let window = crate::runtime::GpuSignalSummaryWindow::new(
        source_frames,
        first_frame,
        bucket_frames,
        4,
        buckets,
        4_501,
        1,
    )
    .expect("bounded precise fixture window");
    let mut presentation = precise_signal_fixture_presentation(first_frame, scenario);
    if bucket_frames == 4 {
        presentation.viewport = crate::runtime::GpuSignalViewport::new(
            crate::runtime::GpuSignalPosition::new(first_frame + 8, 0.25)
                .expect("fractional precise start"),
            2.0,
        )
        .expect("sub-bucket viewport");
    }
    let descriptor = window
        .content(&presentation)
        .expect("precise custom shader content");
    let primitive = crate::runtime::PaintPrimitive::GpuSurface(PaintGpuSurface {
        widget_id: 4_501,
        key: 4_501,
        revision: 1,
        rect: Rect::from_min_size(
            Point::new(0.0, 0.0),
            Vector2::new(TARGET_SIZE as f32, TARGET_SIZE as f32),
        ),
        content: descriptor,
        capabilities: GpuSurfaceCapabilities::default(),
        overlays: Vec::new(),
    });
    precise_signal_render_pixels(device, queue, &[primitive], true)
}

fn precise_signal_fixture_presentation(
    first_frame: u64,
    scenario: PreciseSignalFixtureScenario,
) -> crate::runtime::GpuPreciseSignalPresentation {
    let base = crate::runtime::GpuSignalViewport::new(
        crate::runtime::GpuSignalPosition::new(first_frame + 8, 0.25)
            .expect("base fixture position"),
        16.0,
    )
    .expect("base fixture viewport");
    let viewport = match scenario {
        PreciseSignalFixtureScenario::Base
        | PreciseSignalFixtureScenario::SlideForward
        | PreciseSignalFixtureScenario::SlideBackward
        | PreciseSignalFixtureScenario::PartialGain => base,
        PreciseSignalFixtureScenario::AdjacentPan => {
            base.translated_frames(1).expect("adjacent pan")
        }
        PreciseSignalFixtureScenario::FractionalZoom => base
            .zoom_anchored(0.375, 11.5)
            .expect("fractional anchored zoom"),
    };
    let mut presentation = crate::runtime::GpuPreciseSignalPresentation::new(viewport);
    presentation.revision = scenario as u64 + 1;
    match scenario {
        PreciseSignalFixtureScenario::SlideForward => presentation.slide_frames = 1,
        PreciseSignalFixtureScenario::SlideBackward => presentation.slide_frames = -1,
        PreciseSignalFixtureScenario::PartialGain => {
            let selection = crate::runtime::GpuSignalViewport::new(
                crate::runtime::GpuSignalPosition::new(first_frame + 4, 0.25)
                    .expect("partial selection start"),
                10.0,
            )
            .expect("partial selection viewport");
            let mut gain = crate::runtime::GpuPreciseSignalGainPreview::new(selection);
            gain.gain = 0.58;
            gain.fade_in_length = 0.45;
            gain.fade_in_curve = 0.65;
            gain.fade_out_length = 0.35;
            gain.fade_out_curve = 0.30;
            gain.fade_in_extension = 0.25;
            gain.fade_out_extension = 0.30;
            gain.fade_in_outer_gain = 0.35;
            gain.fade_out_outer_gain = 0.45;
            presentation.gain_preview = Some(gain);
        }
        _ => {}
    }
    presentation
}

fn precise_signal_legacy_pixels(
    device: &vello::wgpu::Device,
    queue: &vello::wgpu::Queue,
    buckets: &[crate::runtime::GpuSignalSummaryBucket],
    bucket_frames: usize,
    frame_range: [f32; 2],
) -> Vec<u8> {
    let summary = Arc::new(crate::runtime::GpuSignalSummary {
        frames: bucket_frames * 64,
        band_count: 4,
        levels: vec![crate::runtime::GpuSignalSummaryLevel {
            bucket_frames,
            buckets: Arc::from(buckets),
        }],
    });
    let primitive = crate::runtime::PaintPrimitive::GpuSurface(PaintGpuSurface {
        widget_id: 4_502,
        key: 4_502,
        revision: 1,
        rect: Rect::from_min_size(
            Point::new(0.0, 0.0),
            Vector2::new(TARGET_SIZE as f32, TARGET_SIZE as f32),
        ),
        content: GpuSurfaceContent::SignalSummaryBands {
            frames: bucket_frames * 64,
            band_count: 4,
            frame_range,
            summary,
            gain_preview: None,
            sample_slide_frame_offset: 0,
        },
        capabilities: GpuSurfaceCapabilities::default(),
        overlays: Vec::new(),
    });
    precise_signal_render_pixels(device, queue, &[primitive], false)
}

fn precise_signal_render_pixels(
    device: &vello::wgpu::Device,
    queue: &vello::wgpu::Queue,
    primitives: &[crate::runtime::PaintPrimitive],
    custom: bool,
) -> Vec<u8> {
    let mut renderer = GpuSurfaceRenderer::default();
    let (stats, texture) = render_tile_frame(&mut renderer, device, queue, primitives, &[]);
    if custom {
        assert_eq!(stats.custom_shader.surfaces_rendered, 1);
    } else {
        assert_eq!(stats.signal.body_renders, 1);
    }
    let pixels = readback_rgba(device, queue, &texture);
    renderer.recall_presentation_staging_belt();
    pixels
}
