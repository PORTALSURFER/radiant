//! Real offscreen GPU regression for raw-source replacement at the broker byte cap.
use super::super::adapter::NativeAdapterGeneration;
use super::super::runner_state::NativeTargetGeneration;
use super::super::signal_summary_prepare::{
    SummaryBroker, SummaryRequest, SummaryRequestState, SummaryTargetId,
};
use super::*;
use crate::gui::types::{Point, Rect, Vector2};
use crate::runtime::{GpuSurfaceCapabilities, PaintGpuSurface, PaintPrimitive};
use std::{
    hash::{Hash, Hasher},
    io::Write,
    sync::Arc,
    time::Duration,
};
use vello::wgpu;
const TARGET_SIZE: u32 = 64;
const SIGNAL_KEY: u64 = 1456;
struct RenderedCase {
    stats: GpuSurfaceRenderStats,
    pixels: Vec<u8>,
}
fn signal_surface(content: GpuSurfaceContent, revision: u64) -> PaintGpuSurface {
    signal_surface_at(SIGNAL_KEY, content, revision)
}

fn signal_surface_at(key: u64, content: GpuSurfaceContent, revision: u64) -> PaintGpuSurface {
    PaintGpuSurface {
        widget_id: key,
        key,
        revision,
        rect: Rect::from_min_size(
            Point::new(0.0, 0.0),
            Vector2::new(TARGET_SIZE as f32, TARGET_SIZE as f32),
        ),
        content,
        capabilities: GpuSurfaceCapabilities::default(),
        overlays: Vec::new(),
    }
}

fn render_case(
    renderer: &mut GpuSurfaceRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    primitives: &[PaintPrimitive],
) -> RenderedCase {
    let (texture, view) = render_target(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("radiant_offscreen_signal_comparison"),
    });
    let stats = {
        let mut target = GpuSurfaceRenderTarget {
            device,
            queue,
            encoder: &mut encoder,
            target_view: &view,
            format: wgpu::TextureFormat::Rgba8Unorm,
            size: Vector2::new(TARGET_SIZE as f32, TARGET_SIZE as f32),
            dpi_scale: crate::theme::DpiScale::ONE,
            upload_plan_context: None,
            upload_plan: None,
            collect_upload_plan: false,
        };
        let mut occlusion = SurfaceOcclusionPlan::default();
        occlusion.preprocess(primitives);
        renderer.render(&mut target, primitives, &occlusion, &[])
    };
    renderer.finish_presentation_staging_belt();
    queue.submit(std::iter::once(encoder.finish()));
    let pixels = readback_rgba(device, queue, &texture);
    renderer.recall_presentation_staging_belt();
    assert!(
        stats.render_canvas_upload_plan.is_none(),
        "legacy render must not report an upload plan"
    );
    RenderedCase { stats, pixels }
}

fn render_target(device: &wgpu::Device) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("radiant_offscreen_signal_comparison_target"),
        size: wgpu::Extent3d {
            width: TARGET_SIZE,
            height: TARGET_SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn readback_rgba(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
    let row_bytes = TARGET_SIZE * 4;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("radiant_offscreen_signal_comparison_readback"),
        size: u64::from(row_bytes) * u64::from(TARGET_SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_bytes),
                rows_per_image: Some(TARGET_SIZE),
            },
        },
        wgpu::Extent3d {
            width: TARGET_SIZE,
            height: TARGET_SIZE,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));
    let slice = buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(5)),
        })
        .expect("bounded offscreen comparison readback poll");
    receiver
        .recv_timeout(Duration::from_millis(100))
        .expect("offscreen comparison readback callback")
        .expect("offscreen comparison readback result");
    let pixels = slice.get_mapped_range().to_vec();
    buffer.unmap();
    assert_eq!(pixels.len(), (TARGET_SIZE * TARGET_SIZE * 4) as usize);
    pixels
}

fn native_device() -> (wgpu::Device, wgpu::Queue, wgpu::AdapterInfo) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        ..Default::default()
    }))
    .expect("offscreen comparison requires a WGPU adapter");
    let info = adapter.get_info();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("radiant_offscreen_signal_comparison"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    }))
    .expect("offscreen comparison requires a WGPU device");
    (device, queue, info)
}

fn summary_target() -> SummaryTargetId {
    SummaryTargetId::new(
        winit::window::WindowId::dummy(),
        NativeAdapterGeneration::from_test_serial(1),
        NativeTargetGeneration::from_test_serial(1),
        SIGNAL_KEY,
    )
    .expect("target")
}

fn run_dispatches(broker: &mut SummaryBroker) {
    while let Some(dispatch) = broker.take_dispatch() {
        dispatch.run();
        broker.drain_completions();
    }
}

fn capture_bounded_signal_fixture(
    adapter: &wgpu::AdapterInfo,
    source: &[f32],
    overview: &RenderedCase,
    tile: &RenderedCase,
    legacy: &RenderedCase,
    capacity: (usize, usize, usize),
) {
    let Some(root) = std::env::var_os("RADIANT_BOUNDED_SIGNAL_OUTPUT_DIR") else {
        return;
    };
    let revision = std::env::var("RADIANT_BOUNDED_SIGNAL_SOURCE_REVISION")
        .expect("native capture requires exact source revision");
    let root = std::path::PathBuf::from(root);
    std::fs::create_dir(&root).expect("capture directory must be fresh");
    let write = |name: &str, bytes: &[u8]| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(name))
            .expect("exclusively create native fixture output");
        file.write_all(bytes).expect("write native fixture output");
    };
    write("overview.rgba", &overview.pixels);
    write("tile.rgba", &tile.pixels);
    write("legacy.rgba", &legacy.pixels);
    let hash = |bytes: &[u8]| {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut state);
        format!("{:#018x}", state.finish())
    };
    let source_bytes: Vec<_> = source
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    let record = serde_json::json!({
        "fixture": "bounded-signal-native-detail", "source_revision": revision,
        "adapter": {"name": adapter.name, "backend": format!("{:?}", adapter.backend),
            "device_type": format!("{:?}", adapter.device_type), "vendor": adapter.vendor, "device": adapter.device},
        "source_frames": source.len(), "source_rootcanhash": hash(&source_bytes),
        "rgba_rootcanhash": {"overview": hash(&overview.pixels),
            "tile": hash(&tile.pixels), "legacy": hash(&legacy.pixels)},
        "immutable_uploads": {"overview": format!("{:?}", overview.stats.render_canvas_uploads.immutable_payload),
            "tile": format!("{:?}", tile.stats.render_canvas_uploads.immutable_payload)},
        "broker_logical_bytes": {"source": capacity.0, "summary": capacity.1, "gpu": capacity.2},
        "measurement": "offscreen native render evidence; no foreground latency or GPU timing claim"
    });
    write(
        "bounded-signal.json",
        &serde_json::to_vec_pretty(&record).expect("serialize capture"),
    );
}

#[test]
#[ignore = "requires a native offscreen WGPU adapter"]
fn async_signal_replacement_releases_stale_gpu_leases_at_terminal_boundary() {
    let (device, queue, _) = native_device();
    for duplicate in [false, true] {
        let mut broker = SummaryBroker::with_byte_limit_for_test(100);
        let target = SummaryTargetId::new(
            winit::window::WindowId::dummy(),
            NativeAdapterGeneration::from_test_serial(1),
            NativeTargetGeneration::from_test_serial(1),
            SIGNAL_KEY,
        )
        .expect("target");
        let raw = |samples: Arc<[f32]>| GpuSurfaceContent::SignalBands {
            frames: 4,
            band_count: 1,
            frame_range: [0.0, 4.0],
            samples,
        };
        let old = raw(Arc::from([-0.8, 0.8, -0.6, 0.6]));
        let new = raw(Arc::from([-0.2, 0.2, -0.1, 0.1]));
        let mut renderer = GpuSurfaceRenderer::default();
        assert_eq!(
            broker.request(
                target,
                SummaryRequest::from_raw_surface(&old, 1).expect("raw")
            ),
            SummaryRequestState::Pending
        );
        broker.take_dispatch().expect("old dispatch").run();
        broker.drain_completions();
        assert!(renderer.install_prepared_signal_summary(
            SIGNAL_KEY,
            1,
            &old,
            broker.prepared(target).expect("old ready")
        ));
        let old_primitive = PaintPrimitive::GpuSurface(signal_surface(old.clone(), 1));
        let first = render_case(
            &mut renderer,
            &device,
            &queue,
            std::slice::from_ref(&old_primitive),
        );
        assert_eq!(first.stats.signal.summary_builds, 0);
        assert_eq!(first.stats.signal.body_renders, 1);
        assert!(renderer
            .resources
            .signal_summaries
            .contains_key(&SIGNAL_KEY));
        assert!(renderer.resources.signals.get(&SIGNAL_KEY).is_some());
        assert!(renderer.resources.signal_bodies.get(&SIGNAL_KEY).is_some());
        let warm = render_case(
            &mut renderer,
            &device,
            &queue,
            std::slice::from_ref(&old_primitive),
        );
        assert_eq!(warm.stats.signal.body_cache_hits, 1);
        assert_eq!(first.pixels, warm.pixels);
        assert_eq!(
            broker.request(
                target,
                SummaryRequest::from_raw_surface(&new, 2).expect("replacement")
            ),
            SummaryRequestState::WaitingAdmission
        );
        broker.maintain_retired();
        assert!(
            broker.capacity_status().logical_bytes > 0,
            "GPU owners retain old reservation"
        );
        let new_primitive = PaintPrimitive::GpuSurface(signal_surface(new.clone(), 2));
        let primitives = if duplicate {
            vec![old_primitive, new_primitive.clone()]
        } else {
            vec![new_primitive.clone()]
        };
        let pending = render_case(&mut renderer, &device, &queue, &primitives);
        assert_eq!(pending.stats.signal.summary_builds, 0);
        assert!(!renderer
            .resources
            .signal_summaries
            .contains_key(&SIGNAL_KEY));
        assert!(renderer.resources.signals.get(&SIGNAL_KEY).is_none());
        assert!(renderer.resources.signal_bodies.get(&SIGNAL_KEY).is_none());
        broker.maintain_retired();
        assert_eq!(broker.capacity_status().logical_bytes, 0);
        assert_eq!(
            broker.request(
                target,
                SummaryRequest::from_raw_surface(&new, 2).expect("replacement")
            ),
            SummaryRequestState::Pending
        );
        broker.take_dispatch().expect("new dispatch").run();
        assert_eq!(broker.drain_completions(), vec![target]);
        assert!(renderer.install_prepared_signal_summary(
            SIGNAL_KEY,
            2,
            &new,
            broker.prepared(target).expect("new ready")
        ));
        let ready = render_case(&mut renderer, &device, &queue, &[new_primitive]);
        assert_eq!(ready.stats.signal.summary_builds, 0);
        assert_eq!(ready.stats.signal.body_renders, 1);
        assert_ne!(first.pixels, ready.pixels);
        drop(renderer);
        broker.release_target(target);
        broker.maintain_retired();
        assert_eq!(broker.capacity_status().logical_bytes, 0);
    }
}

#[test]
#[ignore = "requires a native offscreen WGPU adapter"]
fn bounded_detail_matches_legacy_pixels_and_reuses_its_tile_page() {
    let (device, queue, adapter) = native_device();
    const FRAMES: usize = 65_536;
    let samples: Arc<[f32]> = Arc::from(
        (0..FRAMES)
            .map(|frame| ((frame % 251) as f32 / 125.0) - 1.0)
            .collect::<Vec<_>>(),
    );
    let raw = |range| GpuSurfaceContent::SignalBands {
        frames: FRAMES,
        band_count: 1,
        frame_range: range,
        samples: Arc::clone(&samples),
    };
    let close = raw([32_768.0, 32_800.0]);
    let target = summary_target();
    let mut broker = SummaryBroker::with_byte_limit_for_test(16 * 1024 * 1024);
    assert_eq!(
        broker.request(
            target,
            SummaryRequest::from_raw_surface(&close, 1).expect("close source")
        ),
        SummaryRequestState::Pending
    );
    let overview_dispatch = broker.take_dispatch().expect("overview dispatch");
    overview_dispatch.run();
    assert_eq!(broker.drain_completions(), vec![target]);
    let overview_prepared = broker.prepared(target).expect("overview prepared");
    assert!(overview_prepared.tile().is_none());
    let overview_asset = overview_prepared.asset_key();

    let mut renderer = GpuSurfaceRenderer::default();
    assert!(renderer.install_prepared_signal_summary(SIGNAL_KEY, 1, &close, overview_prepared));
    let close_primitive = PaintPrimitive::GpuSurface(signal_surface(close.clone(), 1));
    let overview = render_case(
        &mut renderer,
        &device,
        &queue,
        std::slice::from_ref(&close_primitive),
    );

    run_dispatches(&mut broker);
    let tile_prepared = broker.prepared(target).expect("tile prepared");
    assert!(tile_prepared.tile().is_some());
    assert_ne!(overview_asset, tile_prepared.asset_key());
    assert!(renderer.install_prepared_signal_summary(SIGNAL_KEY, 1, &close, tile_prepared));
    let tile = render_case(
        &mut renderer,
        &device,
        &queue,
        std::slice::from_ref(&close_primitive),
    );
    let tile_buffer_key = renderer
        .resources
        .signals
        .get(&SIGNAL_KEY)
        .expect("tile buffer")
        .cache_key;

    let legacy = GpuSurfaceContent::SignalSummaryBands {
        frames: FRAMES,
        band_count: 1,
        frame_range: [32_768.0, 32_800.0],
        summary: Arc::new(crate::runtime::GpuSignalSummary::from_interleaved_samples(
            &samples, FRAMES, 1,
        )),
        gain_preview: None,
        sample_slide_frame_offset: 0,
    };
    let mut legacy_renderer = GpuSurfaceRenderer::default();
    let legacy_primitive = PaintPrimitive::GpuSurface(signal_surface_at(SIGNAL_KEY + 1, legacy, 1));
    let legacy = render_case(&mut legacy_renderer, &device, &queue, &[legacy_primitive]);
    assert_eq!(
        tile.pixels, legacy.pixels,
        "completed precise tile must match full summary"
    );

    let pan = raw([32_772.0, 32_804.0]);
    assert_eq!(
        broker.request(
            target,
            SummaryRequest::from_raw_surface(&pan, 1).expect("pan source")
        ),
        SummaryRequestState::Ready
    );
    run_dispatches(&mut broker);
    let pan_prepared = broker.prepared(target).expect("pan tile prepared");
    assert_eq!(
        pan_prepared.asset_key(),
        renderer.resources.signal_summaries[&SIGNAL_KEY]
            .prepared
            .asset_key()
    );
    assert!(renderer.install_prepared_signal_summary(SIGNAL_KEY, 1, &pan, pan_prepared));
    let pan_primitive = PaintPrimitive::GpuSurface(signal_surface(pan, 1));
    let pan_render = render_case(&mut renderer, &device, &queue, &[pan_primitive]);
    assert_eq!(
        renderer.resources.signals[&SIGNAL_KEY].cache_key,
        tile_buffer_key
    );
    assert_eq!(
        pan_render
            .stats
            .render_canvas_uploads
            .immutable_payload
            .operations,
        Some(0)
    );

    let end = raw([(FRAMES - 40) as f32, (FRAMES - 8) as f32]);
    assert_eq!(
        broker.request(
            target,
            SummaryRequest::from_raw_surface(&end, 1).expect("end source")
        ),
        SummaryRequestState::Ready
    );
    run_dispatches(&mut broker);
    let end_prepared = broker.prepared(target).expect("end tile");
    assert!(renderer.install_prepared_signal_summary(SIGNAL_KEY, 1, &end, end_prepared));
    let end_tile = render_case(
        &mut renderer,
        &device,
        &queue,
        &[PaintPrimitive::GpuSurface(signal_surface(end.clone(), 1))],
    );
    let end_legacy = GpuSurfaceContent::SignalSummaryBands {
        frames: FRAMES,
        band_count: 1,
        frame_range: [(FRAMES - 40) as f32, (FRAMES - 8) as f32],
        summary: Arc::new(crate::runtime::GpuSignalSummary::from_interleaved_samples(
            &samples, FRAMES, 1,
        )),
        gain_preview: None,
        sample_slide_frame_offset: 0,
    };
    let end_reference = render_case(
        &mut legacy_renderer,
        &device,
        &queue,
        &[PaintPrimitive::GpuSurface(signal_surface_at(
            SIGNAL_KEY + 2,
            end_legacy,
            1,
        ))],
    );
    assert_eq!(
        end_tile.pixels, end_reference.pixels,
        "near-end tile must retain legacy clamping"
    );

    let capacity = broker.capacity_status();
    capture_bounded_signal_fixture(
        &adapter,
        &samples,
        &overview,
        &tile,
        &legacy,
        (
            capacity.source_logical_bytes,
            capacity.summary_logical_bytes,
            capacity.gpu_logical_bytes,
        ),
    );
}

#[test]
#[ignore = "requires a native offscreen WGPU adapter"]
fn prepared_signal_budget_denial_keeps_old_resources_until_renderer_drops() {
    let (device, queue, _) = native_device();
    let raw = |samples: Arc<[f32]>| GpuSurfaceContent::SignalBands {
        frames: 4,
        band_count: 1,
        frame_range: [0.0, 4.0],
        samples,
    };
    let old = raw(Arc::from([-0.9, 0.9, -0.8, 0.8]));
    let new = raw(Arc::from([0.05, 0.1, 0.15, 0.2]));

    // Measure the complete first native residency (including any cache owners)
    // with the same adapter and geometry. The production budget remains immutable.
    let first_used = {
        let target = summary_target();
        let mut broker = SummaryBroker::with_byte_limit_for_test(1024 * 1024);
        broker.set_gpu_budget_limit_for_test(1024 * 1024);
        assert_eq!(
            broker.request(
                target,
                SummaryRequest::from_raw_surface(&old, 1).expect("old")
            ),
            SummaryRequestState::Pending
        );
        run_dispatches(&mut broker);
        let mut renderer = GpuSurfaceRenderer::default();
        assert!(renderer.install_prepared_signal_summary(
            SIGNAL_KEY,
            1,
            &old,
            broker.prepared(target).expect("calibration prepared")
        ));
        let _ = render_case(
            &mut renderer,
            &device,
            &queue,
            &[PaintPrimitive::GpuSurface(signal_surface(old.clone(), 1))],
        );
        let used = broker.gpu_logical_bytes_for_test();
        assert!(used > 0);
        drop(renderer);
        assert_eq!(broker.gpu_logical_bytes_for_test(), 0);
        used
    };

    let target = summary_target();
    let mut broker = SummaryBroker::with_byte_limit_for_test(1024 * 1024);
    broker.set_gpu_budget_limit_for_test(first_used + 1);
    assert_eq!(
        broker.request(
            target,
            SummaryRequest::from_raw_surface(&old, 1).expect("old")
        ),
        SummaryRequestState::Pending
    );
    run_dispatches(&mut broker);
    let mut renderer = GpuSurfaceRenderer::default();
    assert!(renderer.install_prepared_signal_summary(
        SIGNAL_KEY,
        1,
        &old,
        broker.prepared(target).expect("old prepared")
    ));
    let old_render = render_case(
        &mut renderer,
        &device,
        &queue,
        &[PaintPrimitive::GpuSurface(signal_surface(old.clone(), 1))],
    );
    assert_eq!(broker.gpu_logical_bytes_for_test(), first_used);

    assert_eq!(
        broker.request(
            target,
            SummaryRequest::from_raw_surface(&new, 2).expect("new")
        ),
        SummaryRequestState::Pending
    );
    run_dispatches(&mut broker);
    assert!(renderer.install_prepared_signal_summary(
        SIGNAL_KEY,
        2,
        &new,
        broker.prepared(target).expect("new prepared")
    ));
    let denied = render_case(
        &mut renderer,
        &device,
        &queue,
        &[PaintPrimitive::GpuSurface(signal_surface(new, 2))],
    );
    assert_eq!(
        denied.stats.signal.body_renders, 0,
        "denied replacement must remain incomplete"
    );
    assert_ne!(
        denied.pixels, old_render.pixels,
        "denied replacement must not draw the stale body"
    );
    assert_eq!(
        broker.gpu_logical_bytes_for_test(),
        first_used,
        "old resources retain their charge until renderer drop"
    );
    drop(renderer);
    assert_eq!(broker.gpu_logical_bytes_for_test(), 0);
}
