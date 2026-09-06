//! Opt-in native-GPU coverage for the retained custom-shader cache.
//!
//! These tests deliberately create a real offscreen WGPU device. They remain
//! ignored in the normal headless suite and fail explicitly when a requested
//! native adapter is unavailable.

use super::*;
use crate::gui::types::{Point, Rect, Vector2};
use crate::gui_runtime::native_vello::generic_runtime::adapter::NativeAdapterGeneration;
use crate::gui_runtime::native_vello::generic_runtime::closing::NativeLifecycle;
use crate::gui_runtime::native_vello::generic_runtime::native_encode_present::{
    NativeEncodePresentPath, NativeEncodePresentPlanContext,
};
use crate::gui_runtime::native_vello::generic_runtime::native_visual_packet::{
    NativeVisualRequestAdapter, NativeVisualRequestBegin, NativeVisualRequestMailbox,
};
use crate::gui_runtime::native_vello::generic_runtime::runner_state::NativeTargetGeneration;
use crate::runtime::{
    GpuShaderSurfaceDescriptor, GpuSurfaceCapabilities, GpuSurfaceContent, PaintGpuSurface,
};
use std::num::NonZeroU64;
use std::sync::Arc;
use std::time::Duration;
use vello::wgpu;
use winit::window::WindowId;

const TARGET_SIZE: u32 = 64;
const OLD_ASSOCIATIONS: u64 = 1024;
const OLD_KEY_BASE: u64 = 10_000;
const FRESH_KEY: u64 = 99_999;

const RED_SHADER: &str = r#"
@vertex fn vertex_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    return vec4<f32>(positions[index], 0.0, 1.0);
}
@fragment fn fragment_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
"#;

#[test]
#[ignore = "requires native GPU adapter and offscreen WGPU rendering"]
fn shared_pipeline_transition_renders_one_fresh_surface_and_retires_1024_old_associations() {
    let (device, queue) = native_device();
    let descriptor = valid_descriptor();
    let mut renderer = seeded_renderer(&device, &queue, &descriptor);
    let identity = renderer
        .resources
        .custom_shader_pipeline_identity(OLD_KEY_BASE)
        .expect("seed pipeline association")
        .clone();
    for key in OLD_KEY_BASE + 1..OLD_KEY_BASE + OLD_ASSOCIATIONS {
        renderer
            .resources
            .associate_custom_shader_pipeline(key, identity.clone());
    }
    assert_eq!(renderer.resources.custom_shader_pipeline_count(), 1);

    let primitives = vec![PaintPrimitive::GpuSurface(surface(FRESH_KEY, descriptor))];
    let context = upload_plan_context(&device);
    let plan = renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
        context,
        &primitives,
        crate::theme::DpiScale::ONE,
        &[],
    );
    let (texture, view) = render_target(&device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("radiant_native_custom_shader_transition"),
    });
    let mut target = GpuSurfaceRenderTarget {
        device: &device,
        queue: &queue,
        encoder: &mut encoder,
        target_view: &view,
        format: wgpu::TextureFormat::Rgba8Unorm,
        size: Vector2::new(TARGET_SIZE as f32, TARGET_SIZE as f32),
        dpi_scale: crate::theme::DpiScale::ONE,
        upload_plan_context: Some(context),
        upload_plan: Some(plan),
        collect_upload_plan: true,
    };
    let mut occlusion = SurfaceOcclusionPlan::default();
    occlusion.preprocess(&primitives);
    let stats = renderer.render(&mut target, &primitives, &occlusion, &[]);
    renderer.finish_presentation_staging_belt();
    queue.submit(std::iter::once(encoder.finish()));

    assert_eq!(stats.custom_shader.surfaces_rendered, 1);
    assert_eq!(stats.custom_shader.pipeline_rebuilds, 0);
    assert!(
        renderer
            .resources
            .custom_shader_pipeline_identity(FRESH_KEY)
            .is_some()
    );
    assert!(
        renderer
            .resources
            .custom_shader_pipeline_identity(OLD_KEY_BASE)
            .is_none()
    );
    assert_eq!(
        renderer
            .custom_shader_residency_snapshot()
            .pipeline_resident_count,
        1
    );
    assert_red_pixel(&device, &queue, &texture);
    renderer.recall_presentation_staging_belt();
}

#[test]
#[ignore = "requires native GPU adapter and offscreen WGPU rendering"]
fn vetoed_transition_restores_predecessor_resources_without_accumulation() {
    let (device, queue) = native_device();
    let descriptor = valid_descriptor();
    let mut renderer = seeded_renderer(&device, &queue, &descriptor);
    let identity = renderer
        .resources
        .custom_shader_pipeline_identity(OLD_KEY_BASE)
        .expect("seed pipeline association")
        .clone();
    for key in OLD_KEY_BASE + 1..OLD_KEY_BASE + OLD_ASSOCIATIONS {
        renderer
            .resources
            .associate_custom_shader_pipeline(key, identity.clone());
    }
    let old_write_state = renderer
        .resources
        .custom_shader_binding(OLD_KEY_BASE)
        .expect("seed binding")
        .write_state;
    let primitives = vec![PaintPrimitive::GpuSurface(surface(FRESH_KEY, descriptor))];

    for _ in 0..2 {
        let context = upload_plan_context(&device);
        let mut plan = renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
            context,
            &primitives,
            crate::theme::DpiScale::ONE,
            &[],
        );
        // Test-only hook supplied by upload_plan.rs: leave a terminal action
        // after Prune so finish_execution rejects the otherwise valid frame.
        plan.append_action_for_test(GpuSurfaceRenderCanvasUploadAction::BeginFrame);
        let (_texture, view) = render_target(&device);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("radiant_native_custom_shader_veto"),
        });
        let mut target = GpuSurfaceRenderTarget {
            device: &device,
            queue: &queue,
            encoder: &mut encoder,
            target_view: &view,
            format: wgpu::TextureFormat::Rgba8Unorm,
            size: Vector2::new(TARGET_SIZE as f32, TARGET_SIZE as f32),
            dpi_scale: crate::theme::DpiScale::ONE,
            upload_plan_context: Some(context),
            upload_plan: Some(plan),
            collect_upload_plan: true,
        };
        let mut occlusion = SurfaceOcclusionPlan::default();
        occlusion.preprocess(&primitives);
        let _ = renderer.render(&mut target, &primitives, &occlusion, &[]);

        assert!(
            renderer
                .resources
                .custom_shader_pipeline_identity(OLD_KEY_BASE)
                .is_some()
        );
        assert!(renderer.resources.has_custom_shader_binding(OLD_KEY_BASE));
        assert_eq!(
            renderer
                .resources
                .custom_shader_binding(OLD_KEY_BASE)
                .expect("restored seed binding")
                .write_state,
            old_write_state
        );
        assert!(
            renderer
                .resources
                .custom_shader_pipeline_identity(FRESH_KEY)
                .is_none()
        );
        assert!(!renderer.resources.has_custom_shader_binding(FRESH_KEY));
        assert_eq!(renderer.resources.custom_shader_pipeline_count(), 1);
        assert_eq!(renderer.resources.custom_shader_binding_count(), 1);
    }
}

#[test]
#[ignore = "requires native GPU adapter and offscreen WGPU rendering"]
fn planned_invalid_wgsl_replacement_preserves_existing_pipeline_and_binding() {
    assert_invalid_shader_preserves_resources(false);
}

#[test]
#[ignore = "requires native GPU adapter and offscreen WGPU rendering"]
fn legacy_failed_transition_restores_predecessor_resources() {
    assert_invalid_shader_preserves_resources(true);
}

fn assert_invalid_shader_preserves_resources(legacy_transition: bool) {
    let (device, queue) = native_device();
    let descriptor = valid_descriptor();
    let mut renderer = seeded_renderer(&device, &queue, &descriptor);
    let old_identity = renderer
        .resources
        .custom_shader_pipeline_identity(OLD_KEY_BASE)
        .expect("seed pipeline association")
        .clone();
    let old_write_state = renderer
        .resources
        .custom_shader_binding(OLD_KEY_BASE)
        .expect("seed binding")
        .write_state;
    let invalid = GpuShaderSurfaceDescriptor::new("native-shared-red")
        .wgsl_source("this is not WGSL")
        .entry_point("vertex_main")
        .fragment_entry_point("fragment_main");
    let invalid_key = if legacy_transition {
        for key in OLD_KEY_BASE + 1..OLD_KEY_BASE + OLD_ASSOCIATIONS {
            renderer
                .resources
                .associate_custom_shader_pipeline(key, old_identity.clone());
        }
        FRESH_KEY
    } else {
        OLD_KEY_BASE
    };
    let invalid_primitives = vec![PaintPrimitive::GpuSurface(surface(invalid_key, invalid))];
    let context = (!legacy_transition).then(|| upload_plan_context(&device));
    let plan = context.map(|context| {
        renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
            context,
            &invalid_primitives,
            crate::theme::DpiScale::ONE,
            &[],
        )
    });
    let (_texture, view) = render_target(&device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("radiant_native_invalid_custom_shader"),
    });
    let mut target = render_target_for_test(&device, &queue, &mut encoder, &view, context, plan);
    let mut occlusion = SurfaceOcclusionPlan::default();
    occlusion.preprocess(&invalid_primitives);
    let stats = renderer.render(&mut target, &invalid_primitives, &occlusion, &[]);

    assert_eq!(stats.custom_shader.surfaces_rendered, 0);
    assert_eq!(
        renderer
            .resources
            .custom_shader_pipeline_identity(OLD_KEY_BASE),
        Some(&old_identity)
    );
    assert_eq!(
        renderer
            .resources
            .custom_shader_binding(OLD_KEY_BASE)
            .expect("binding survives invalid replacement")
            .write_state,
        old_write_state
    );
    assert_eq!(renderer.resources.custom_shader_pipeline_count(), 1);
    assert_eq!(renderer.resources.custom_shader_binding_count(), 1);
    assert_eq!(stats.custom_shader.failures.shader_module_failures, 1);
}

fn native_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        ..Default::default()
    }))
    .expect("native custom-shader test requires a WGPU adapter");
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("radiant_native_custom_shader_test"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    }))
    .expect("native custom-shader test requires a WGPU device")
}

fn seeded_renderer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    descriptor: &GpuShaderSurfaceDescriptor,
) -> GpuSurfaceRenderer {
    let mut renderer = GpuSurfaceRenderer::default();
    let primitives = vec![PaintPrimitive::GpuSurface(surface(
        OLD_KEY_BASE,
        descriptor.clone(),
    ))];
    let (_texture, view) = render_target(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("radiant_native_custom_shader_seed"),
    });
    let mut target = render_target_for_test(device, queue, &mut encoder, &view, None, None);
    let mut occlusion = SurfaceOcclusionPlan::default();
    occlusion.preprocess(&primitives);
    assert_eq!(
        renderer
            .render(&mut target, &primitives, &occlusion, &[])
            .custom_shader
            .surfaces_rendered,
        1
    );
    assert!(renderer.resources.has_custom_shader_binding(OLD_KEY_BASE));
    renderer
}

fn valid_descriptor() -> GpuShaderSurfaceDescriptor {
    GpuShaderSurfaceDescriptor::new("native-shared-red")
        .wgsl_source(Arc::<str>::from(RED_SHADER))
        .entry_point("vertex_main")
        .fragment_entry_point("fragment_main")
        .storage_identity(1)
        .storage_revision(1)
}

fn surface(key: u64, descriptor: GpuShaderSurfaceDescriptor) -> PaintGpuSurface {
    PaintGpuSurface {
        widget_id: 1,
        key,
        revision: 1,
        rect: Rect::from_min_size(
            Point::new(0.0, 0.0),
            Vector2::new(TARGET_SIZE as f32, TARGET_SIZE as f32),
        ),
        content: GpuSurfaceContent::CustomShader {
            descriptor: Arc::new(descriptor),
        },
        capabilities: GpuSurfaceCapabilities::default(),
        overlays: Vec::new(),
    }
}

fn render_target(device: &wgpu::Device) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("radiant_native_custom_shader_target"),
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

fn render_target_for_test<'a>(
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
    context: Option<GpuSurfaceRenderCanvasUploadPlanContext>,
    plan: Option<GpuSurfaceRenderCanvasUploadPlan>,
) -> GpuSurfaceRenderTarget<'a> {
    GpuSurfaceRenderTarget {
        device,
        queue,
        encoder,
        target_view: view,
        format: wgpu::TextureFormat::Rgba8Unorm,
        size: Vector2::new(TARGET_SIZE as f32, TARGET_SIZE as f32),
        dpi_scale: crate::theme::DpiScale::ONE,
        upload_plan_context: context,
        upload_plan: plan,
        collect_upload_plan: context.is_some(),
    }
}

fn upload_plan_context(device: &wgpu::Device) -> GpuSurfaceRenderCanvasUploadPlanContext {
    let mut mailbox = NativeVisualRequestMailbox::new();
    let window_id = WindowId::dummy();
    assert!(mailbox.bind_window(window_id));
    let _ = mailbox
        .enqueue_for_test(crate::gui_runtime::native_vello::generic_runtime::FrameWork::None);
    let packet = match NativeVisualRequestAdapter::begin(&mut mailbox, window_id, true) {
        NativeVisualRequestBegin::Requested(packet) => packet.identity(),
        other => panic!("unexpected packet begin: {other:?}"),
    };
    GpuSurfaceRenderCanvasUploadPlanContext::new(
        NativeEncodePresentPlanContext {
            packet,
            adapter_generation: NativeAdapterGeneration::from_test_serial(1),
            target_generation: NativeTargetGeneration::from_test_serial(1),
            lifecycle: NativeLifecycle::default(),
            path: NativeEncodePresentPath::Composited,
            snapshot_revision: NonZeroU64::MIN,
        },
        NativeAdapterGeneration::from_test_serial(1),
        GpuSurfaceRenderCanvasUploadTarget::new(
            super::wgpu_device_id(device),
            wgpu::TextureFormat::Rgba8Unorm,
            TARGET_SIZE,
            TARGET_SIZE,
        ),
    )
    .expect("valid upload-plan context")
}

fn assert_red_pixel(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) {
    let row_bytes = 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("radiant_native_custom_shader_readback"),
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
        .expect("bounded native custom-shader readback poll");
    receiver
        .recv_timeout(Duration::from_millis(100))
        .expect("native custom-shader readback callback")
        .expect("native custom-shader readback result");
    let mapped = slice.get_mapped_range();
    assert!(
        mapped[0] > 200 && mapped[1] < 20 && mapped[2] < 20,
        "expected rendered red pixel, got {:?}",
        &mapped[..4]
    );
    drop(mapped);
    buffer.unmap();
}
