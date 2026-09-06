//! Real offscreen GPU regression for raw-source replacement at the broker byte cap.
use super::super::adapter::NativeAdapterGeneration;
use super::super::runner_state::NativeTargetGeneration;
use super::super::signal_summary_prepare::{
    SummaryBroker, SummaryRequest, SummaryRequestState, SummaryTargetId,
};
use super::*;
use crate::gui::types::{Point, Rect, Vector2};
use crate::runtime::{GpuSurfaceCapabilities, PaintGpuSurface, PaintPrimitive};
use std::{sync::Arc, time::Duration};
use vello::wgpu;
const TARGET_SIZE: u32 = 64;
const SIGNAL_KEY: u64 = 1456;
const SIGNAL_WIDGET_ID: u64 = 1456;
struct RenderedCase {
    stats: GpuSurfaceRenderStats,
    pixels: Vec<u8>,
}
fn signal_surface(content: GpuSurfaceContent, revision: u64) -> PaintGpuSurface {
    PaintGpuSurface {
        widget_id: SIGNAL_WIDGET_ID,
        key: SIGNAL_KEY,
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

fn native_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        ..Default::default()
    }))
    .expect("offscreen comparison requires a WGPU adapter");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("radiant_offscreen_signal_comparison"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    }))
    .expect("offscreen comparison requires a WGPU device");
    (device, queue)
}

#[test]
#[ignore = "requires a native offscreen WGPU adapter"]
fn async_signal_replacement_releases_stale_gpu_leases_at_terminal_boundary() {
    let (device, queue) = native_device();
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
        assert!(
            renderer
                .resources
                .signal_summaries
                .contains_key(&SIGNAL_KEY)
        );
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
        assert!(
            !renderer
                .resources
                .signal_summaries
                .contains_key(&SIGNAL_KEY)
        );
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
