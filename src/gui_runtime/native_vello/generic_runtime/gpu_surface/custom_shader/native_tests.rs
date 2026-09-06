//! Opt-in native-GPU coverage for the retained custom-shader cache.
//!
//! These tests deliberately create a real offscreen WGPU device. They remain
//! ignored in the normal headless suite and fail explicitly when a requested
//! native adapter is unavailable.

use super::upload_plan::{
    GpuSurfaceRenderCanvasUploadClass, GpuSurfaceRenderCanvasUploadCustomPresentationSource,
};
use super::*;
use crate::gui::repaint::RepaintSignal;
use crate::gui::types::{Point, Rect, Vector2};
use crate::gui_runtime::native_vello::generic_runtime::adapter::NativeAdapterGeneration;
use crate::gui_runtime::native_vello::generic_runtime::closing::NativeLifecycle;
use crate::gui_runtime::native_vello::generic_runtime::custom_shader_prepare::{
    CustomShaderPreparationBroker, CustomShaderPreparationDispatch, CustomShaderPreparationRequest,
    CustomShaderPreparationState, CustomShaderTargetId,
};
use crate::gui_runtime::native_vello::generic_runtime::device::wgpu_device_id;
use crate::gui_runtime::native_vello::generic_runtime::gpu_surface::custom_shader::pipeline::custom_shader_pipeline_key;
use crate::gui_runtime::native_vello::generic_runtime::native_encode_present::{
    NativeEncodePresentPath, NativeEncodePresentPlanContext,
};
use crate::gui_runtime::native_vello::generic_runtime::native_visual_packet::{
    NativeVisualRequestAdapter, NativeVisualRequestBegin, NativeVisualRequestMailbox,
};
use crate::gui_runtime::native_vello::generic_runtime::runner_state::NativeTargetGeneration;
use crate::runtime::{
    GpuShaderPresentationUniformUpdate, GpuShaderSurfaceDescriptor, GpuSurfaceCapabilities,
    GpuSurfaceContent, PaintGpuSurface,
};
use std::num::NonZeroU64;
use std::sync::Arc;
use std::time::Duration;
use vello::wgpu;
use winit::window::WindowId;

#[path = "native_tests/precise_signal.rs"]
mod precise_signal;

const TARGET_SIZE: u32 = 64;
const OLD_ASSOCIATIONS: u64 = 1024;
const OLD_KEY_BASE: u64 = 10_000;
const FRESH_KEY: u64 = 99_999;
const TILE_COUNT: usize = 16;
const TILE_GRID_WIDTH: usize = 4;
const TILE_SIZE: u32 = TARGET_SIZE / TILE_GRID_WIDTH as u32;
const TILE_KEY_BASE: u64 = 200_000;
const TILE_WIDGET_ID: u64 = 33;
const UPDATED_TILE_INDEX: usize = 6;

struct NativePreparationWake;

impl RepaintSignal for NativePreparationWake {
    fn request_repaint(&self) {}
}

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

const TILE_SHADER: &str = r#"
@group(0) @binding(1) var<uniform> base: vec4<f32>;
@group(0) @binding(3) var<uniform> presentation: vec4<f32>;

@vertex fn vertex_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    return vec4<f32>(positions[index], 0.0, 1.0);
}
@fragment fn fragment_main() -> @location(0) vec4<f32> {
    return base + presentation;
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
    assert_eq!(
        stage_custom_shader_preparations(&mut renderer, &device, &primitives),
        1
    );
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
        assert_eq!(
            stage_custom_shader_preparations(&mut renderer, &device, &primitives),
            1
        );
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
    assert_eq!(
        stage_custom_shader_preparations(&mut renderer, &device, &invalid_primitives),
        1
    );
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

#[test]
#[ignore = "requires native GPU adapter and offscreen WGPU rendering"]
fn equivalent_custom_shader_tiles_share_one_pipeline_and_isolate_binding_updates() {
    let (device, queue) = native_device();
    let mut renderer = GpuSurfaceRenderer::default();

    let first_primitives = tile_primitives(None);
    assert_eq!(
        stage_custom_shader_preparations(&mut renderer, &device, &first_primitives),
        1
    );
    let (first_stats, first_texture) =
        render_tile_frame(&mut renderer, &device, &queue, &first_primitives, &[]);
    assert_eq!(first_stats.custom_shader.surfaces_rendered, TILE_COUNT);
    assert_eq!(first_stats.custom_shader.pipeline_rebuilds, 0);
    assert_eq!(first_stats.custom_shader.binding_rebuilds, TILE_COUNT);
    assert_tile_cache_residency(&renderer);
    let first_pixels = readback_rgba(&device, &queue, &first_texture);
    for index in 0..TILE_COUNT {
        assert_tile_color(&first_pixels, index, tile_base_color(index));
    }
    renderer.recall_presentation_staging_belt();
    let first_write_states = (0..TILE_COUNT)
        .map(|index| {
            renderer
                .resources
                .custom_shader_binding(tile_key(index))
                .expect("first-frame tile binding")
                .write_state
        })
        .collect::<Vec<_>>();

    let second_primitives = tile_primitives(Some(UPDATED_TILE_INDEX));
    let update = GpuShaderPresentationUniformUpdate::try_new(
        TILE_WIDGET_ID,
        tile_key(UPDATED_TILE_INDEX),
        91,
        2,
        2,
        rgba_uniform_bytes([0.0, 1.0, 0.0, 0.0]),
    )
    .expect("aligned presentation update");
    let (second_stats, second_texture) = render_tile_frame(
        &mut renderer,
        &device,
        &queue,
        &second_primitives,
        &[update],
    );
    assert_eq!(second_stats.custom_shader.surfaces_rendered, TILE_COUNT);
    assert_eq!(second_stats.custom_shader.pipeline_rebuilds, 0);
    assert_eq!(second_stats.custom_shader.binding_rebuilds, 0);
    assert_eq!(second_stats.custom_shader.static_writes, 1);
    assert_eq!(second_stats.custom_shader.presentation_writes, 2);
    assert_tile_cache_residency(&renderer);
    let second_pixels = readback_rgba(&device, &queue, &second_texture);
    for index in 0..TILE_COUNT {
        let expected = if index == UPDATED_TILE_INDEX {
            [0.0, 1.0, 0.0, 1.0]
        } else {
            tile_base_color(index)
        };
        assert_tile_color(&second_pixels, index, expected);
    }
    let second_write_states = (0..TILE_COUNT)
        .map(|index| {
            renderer
                .resources
                .custom_shader_binding(tile_key(index))
                .expect("second-frame tile binding")
                .write_state
        })
        .collect::<Vec<_>>();
    for index in 0..TILE_COUNT {
        if index == UPDATED_TILE_INDEX {
            assert_ne!(second_write_states[index], first_write_states[index]);
        } else {
            assert_eq!(second_write_states[index], first_write_states[index]);
        }
    }
    renderer.recall_presentation_staging_belt();
}

#[test]
#[ignore = "requires native GPU adapter and worker-prepared WGPU pipeline"]
fn prepared_custom_shader_preflight_preserves_ordered_payload_and_presentation_actions() {
    let (device, _queue) = native_device();
    let mut renderer = GpuSurfaceRenderer::default();
    let primitives = vec![PaintPrimitive::GpuSurface(tile_surface(
        0,
        tile_descriptor([0.2, 0.3, 0.4, 1.0], 1),
    ))];
    assert_eq!(
        stage_custom_shader_preparations(&mut renderer, &device, &primitives),
        1
    );
    let update = GpuShaderPresentationUniformUpdate::try_new(
        TILE_WIDGET_ID,
        tile_key(0),
        91,
        1,
        2,
        rgba_uniform_bytes([0.0, 1.0, 0.0, 0.0]),
    )
    .expect("aligned presentation update");

    let plan = renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
        upload_plan_context(&device),
        &primitives,
        crate::theme::DpiScale::ONE,
        &[update],
    );
    let actions = &plan.actions;
    assert!(matches!(
        actions.first(),
        Some(GpuSurfaceRenderCanvasUploadAction::BeginFrame)
    ));
    let position = |predicate: fn(&GpuSurfaceRenderCanvasUploadAction) -> bool| {
        actions
            .iter()
            .position(predicate)
            .expect("prepared custom shader action")
    };
    let pipeline = position(
        |action| matches!(action, GpuSurfaceRenderCanvasUploadAction::CustomPipeline { key, rebuild: true, .. } if *key == tile_key(0)),
    );
    let binding = position(
        |action| matches!(action, GpuSurfaceRenderCanvasUploadAction::CustomBinding { key, rebuild: true, .. } if *key == tile_key(0)),
    );
    let static_state = position(
        |action| matches!(action, GpuSurfaceRenderCanvasUploadAction::CustomStaticState { key, write: true, .. } if *key == tile_key(0)),
    );
    let initial_presentation = position(
        |action| matches!(action, GpuSurfaceRenderCanvasUploadAction::CustomPresentationState { key, source: GpuSurfaceRenderCanvasUploadCustomPresentationSource::Initial, write: true, .. } if *key == tile_key(0)),
    );
    let update_presentation = position(
        |action| matches!(action, GpuSurfaceRenderCanvasUploadAction::CustomPresentationState { key, source: GpuSurfaceRenderCanvasUploadCustomPresentationSource::Update, revision: 2, write: true, .. } if *key == tile_key(0)),
    );
    let activate = position(
        |action| matches!(action, GpuSurfaceRenderCanvasUploadAction::Activate { key, .. } if *key == tile_key(0)),
    );
    let prune = position(|action| {
        matches!(
            action,
            GpuSurfaceRenderCanvasUploadAction::Prune { clear: false }
        )
    });
    assert!(pipeline < binding);
    assert!(binding < static_state);
    assert!(static_state < initial_presentation);
    assert!(initial_presentation < update_presentation);
    assert!(update_presentation < activate);
    assert!(activate < prune);
    assert!(actions.iter().any(|action| matches!(
        action,
        GpuSurfaceRenderCanvasUploadAction::Upload {
            class: GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
            byte_len: 16,
            ..
        }
    )));
    assert!(actions.iter().any(|action| matches!(
        action,
        GpuSurfaceRenderCanvasUploadAction::Upload {
            class: GpuSurfaceRenderCanvasUploadClass::VolatilePayload,
            byte_len: 16,
            ..
        }
    )));
    assert!(renderer.resources.custom_shader_pipelines_are_empty());
    assert!(renderer.resources.custom_shader_bindings_are_empty());
}

#[test]
#[ignore = "requires native GPU adapter and worker-prepared WGPU pipelines"]
fn ordered_duplicate_shader_occurrences_release_all_committed_broker_interests() {
    let (device, queue) = native_device();
    let mut second = valid_descriptor();
    second.shader_key.push_str("-second");
    let primitives = vec![
        PaintPrimitive::GpuSurface(surface(FRESH_KEY, valid_descriptor())),
        PaintPrimitive::GpuSurface(surface(FRESH_KEY, second)),
    ];
    let mut renderer = GpuSurfaceRenderer::default();
    let (count, mut broker) =
        stage_custom_shader_preparations_retained(&mut renderer, &device, &primitives);
    assert_eq!(count, 2);
    assert_eq!(broker.capacity_status().entries, 2);
    let (texture, view) = render_target(&device);
    let stats = render_prepared_frame(&mut renderer, &device, &queue, &view, &primitives);
    assert_eq!(stats.custom_shader.surfaces_rendered, 2);
    assert_eq!(stats.custom_shader.pipeline_rebuilds, 0);
    let receipts = renderer.take_committed_custom_shader_targets();
    assert_eq!(receipts.len(), 2);
    for target in receipts {
        broker.consume_target(target);
    }
    broker.maintain_retired();
    assert_eq!(broker.capacity_status().entries, 0);
    assert_eq!(broker.capacity_status().interests, 0);
    assert_eq!(broker.capacity_status().key_text_bytes, 0);
    assert_red_pixel(&device, &queue, &texture);
}

#[test]
#[ignore = "requires native GPU adapter and a pending worker-prepared WGPU pipeline"]
fn pending_replacement_preserves_cached_pipeline_while_cached_surface_renders() {
    let (device, queue) = native_device();
    let mut renderer = GpuSurfaceRenderer::default();
    let cached = PaintPrimitive::GpuSurface(surface(OLD_KEY_BASE, valid_descriptor()));
    stage_custom_shader_preparations(&mut renderer, &device, std::slice::from_ref(&cached));
    let (texture, view) = render_target(&device);
    let initial = render_prepared_frame(
        &mut renderer,
        &device,
        &queue,
        &view,
        std::slice::from_ref(&cached),
    );
    assert_eq!(initial.custom_shader.surfaces_rendered, 1);
    let mut replacement_descriptor = valid_descriptor();
    replacement_descriptor.shader_key.push_str("-pending");
    let replacement = PaintPrimitive::GpuSurface(surface(FRESH_KEY, replacement_descriptor));
    let (_broker, _dispatch) =
        begin_pending_custom_shader_preparation(&device, std::slice::from_ref(&replacement));
    renderer.replace_custom_shader_preparations(Vec::new());
    let primitives = [cached, replacement];
    let stats = render_prepared_frame(&mut renderer, &device, &queue, &view, &primitives);
    assert_eq!(stats.custom_shader.surfaces_rendered, 1);
    assert_eq!(stats.custom_shader.failures.surfaces_failed, 1);
    assert_eq!(stats.custom_shader.pipeline_rebuilds, 0);
    assert!(
        renderer
            .resources
            .custom_shader_pipeline_identity(OLD_KEY_BASE)
            .is_some()
    );
    assert!(renderer.resources.has_custom_shader_binding(OLD_KEY_BASE));
    assert!(
        renderer
            .resources
            .custom_shader_pipeline_identity(FRESH_KEY)
            .is_none()
    );
    assert_red_pixel(&device, &queue, &texture);
}

fn render_prepared_frame(
    renderer: &mut GpuSurfaceRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    view: &wgpu::TextureView,
    primitives: &[PaintPrimitive],
) -> GpuSurfaceRenderStats {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("radiant_native_prepared_shader_frame"),
    });
    let stats = {
        let mut target = render_target_for_test(device, queue, &mut encoder, view, None, None);
        let mut occlusion = SurfaceOcclusionPlan::default();
        occlusion.preprocess(primitives);
        renderer.render(&mut target, primitives, &occlusion, &[])
    };
    renderer.finish_presentation_staging_belt();
    queue.submit(std::iter::once(encoder.finish()));
    renderer.recall_presentation_staging_belt();
    stats
}

fn tile_primitives(updated_tile: Option<usize>) -> Vec<PaintPrimitive> {
    (0..TILE_COUNT)
        .map(|index| {
            let updated = updated_tile == Some(index);
            PaintPrimitive::GpuSurface(tile_surface(
                index,
                tile_descriptor(
                    if updated {
                        [0.0, 0.0, 0.0, 1.0]
                    } else {
                        tile_base_color(index)
                    },
                    if updated { 2 } else { 1 },
                ),
            ))
        })
        .collect()
}

fn tile_descriptor(base: [f32; 4], storage_revision: u64) -> GpuShaderSurfaceDescriptor {
    GpuShaderSurfaceDescriptor::new("native-shared-tile")
        .wgsl_source(Arc::<str>::from(TILE_SHADER))
        .entry_point("vertex_main")
        .fragment_entry_point("fragment_main")
        .uniform_bytes(rgba_uniform_bytes(base))
        .presentation_uniform(rgba_uniform_bytes([0.0; 4]), 1)
        .storage_identity(91)
        .storage_revision(storage_revision)
}

fn tile_surface(index: usize, descriptor: GpuShaderSurfaceDescriptor) -> PaintGpuSurface {
    let x = (index % TILE_GRID_WIDTH) as f32 * TILE_SIZE as f32;
    let y = (index / TILE_GRID_WIDTH) as f32 * TILE_SIZE as f32;
    PaintGpuSurface {
        widget_id: TILE_WIDGET_ID,
        key: tile_key(index),
        revision: 1,
        rect: Rect::from_min_size(
            Point::new(x, y),
            Vector2::new(TILE_SIZE as f32, TILE_SIZE as f32),
        ),
        content: GpuSurfaceContent::CustomShader {
            descriptor: Arc::new(descriptor),
        },
        capabilities: GpuSurfaceCapabilities::default(),
        overlays: Vec::new(),
    }
}

fn tile_key(index: usize) -> u64 {
    TILE_KEY_BASE + index as u64
}

fn tile_base_color(index: usize) -> [f32; 4] {
    [
        0.2 + 0.1 * (index % TILE_GRID_WIDTH) as f32,
        0.2 + 0.1 * (index / TILE_GRID_WIDTH) as f32,
        0.4,
        1.0,
    ]
}

fn rgba_uniform_bytes(rgba: [f32; 4]) -> [u8; 16] {
    let mut bytes = [0; 16];
    for (index, value) in rgba.into_iter().enumerate() {
        bytes[index * 4..(index + 1) * 4].copy_from_slice(&value.to_ne_bytes());
    }
    bytes
}

fn assert_tile_cache_residency(renderer: &GpuSurfaceRenderer) {
    assert_eq!(renderer.resources.custom_shader_pipeline_count(), 1);
    assert_eq!(renderer.resources.custom_shader_binding_count(), TILE_COUNT);
    let residency = renderer.custom_shader_residency_snapshot();
    assert_eq!(residency.pipeline_resident_count, 1);
    assert_eq!(residency.binding_resident_count, TILE_COUNT);
    assert_eq!(
        residency.app_uniform_logical_bytes,
        Some((TILE_COUNT * 16) as u64)
    );
    assert_eq!(
        residency.presentation_uniform_logical_bytes,
        Some((TILE_COUNT * 16) as u64)
    );
}

fn render_tile_frame(
    renderer: &mut GpuSurfaceRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    primitives: &[PaintPrimitive],
    presentation_updates: &[GpuShaderPresentationUniformUpdate],
) -> (GpuSurfaceRenderStats, wgpu::Texture) {
    let (texture, view) = render_target(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("radiant_native_custom_shader_tiles"),
    });
    let stats = {
        let mut target = render_target_for_test(device, queue, &mut encoder, &view, None, None);
        let mut occlusion = SurfaceOcclusionPlan::default();
        occlusion.preprocess(primitives);
        renderer.render(&mut target, primitives, &occlusion, presentation_updates)
    };
    renderer.finish_presentation_staging_belt();
    queue.submit(std::iter::once(encoder.finish()));
    (stats, texture)
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
    assert_eq!(
        stage_custom_shader_preparations(&mut renderer, device, &primitives),
        1
    );
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

/// Exercises the same broker handoff as the native runner: request every
/// surface's immutable pipeline, execute its worker preparation, drain the
/// terminal receipt, and stage the resulting candidate before preflight.
/// The broker coalesces equivalent tile requests, so the returned count is
/// the number of actual pipeline preparations rather than surface interests.
fn stage_custom_shader_preparations(
    renderer: &mut GpuSurfaceRenderer,
    device: &wgpu::Device,
    primitives: &[PaintPrimitive],
) -> usize {
    stage_custom_shader_preparations_retained(renderer, device, primitives).0
}

fn stage_custom_shader_preparations_retained(
    renderer: &mut GpuSurfaceRenderer,
    device: &wgpu::Device,
    primitives: &[PaintPrimitive],
) -> (usize, CustomShaderPreparationBroker) {
    let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
    let target_generation = NativeTargetGeneration::from_test_serial(1);
    let mut broker = CustomShaderPreparationBroker::new(Arc::new(NativePreparationWake));
    let mut requests = Vec::new();

    for (primitive_index, primitive) in primitives.iter().enumerate() {
        let PaintPrimitive::GpuSurface(surface) = primitive else {
            continue;
        };
        let GpuSurfaceContent::CustomShader { descriptor } = &surface.content else {
            continue;
        };
        let key =
            custom_shader_pipeline_key(descriptor).expect("native descriptor has a pipeline key");
        let target = CustomShaderTargetId::new_for_occurrence(
            WindowId::dummy(),
            adapter_generation,
            target_generation,
            surface.key,
            primitive_index,
        )
        .expect("test target serial");
        let request = CustomShaderPreparationRequest::new(
            device.clone(),
            wgpu_device_id(device),
            adapter_generation,
            wgpu::TextureFormat::Rgba8Unorm,
            key,
        );
        assert!(matches!(
            broker.request(target, request.clone()),
            CustomShaderPreparationState::Pending | CustomShaderPreparationState::Ready
        ));
        requests.push((target, request));
    }

    let mut preparations = 0;
    while let Some(dispatch) = broker.take_dispatch() {
        preparations += 1;
        dispatch.run();
        broker.drain_completions();
    }
    let installs = requests
        .into_iter()
        .map(|(target, request)| {
            let prepared = broker.prepared(target);
            let failure = broker.failure(target);
            assert!(
                prepared.is_some() || failure.is_some(),
                "native preparation reached a terminal state"
            );
            (target, request, prepared, failure)
        })
        .collect();
    renderer.replace_custom_shader_preparations(installs);
    (preparations, broker)
}

/// Retains a real active broker dispatch. The caller deliberately does not run
/// it, which keeps the replacement genuinely pending without injecting a fake
/// pipeline candidate into the renderer.
fn begin_pending_custom_shader_preparation(
    device: &wgpu::Device,
    primitives: &[PaintPrimitive],
) -> (
    CustomShaderPreparationBroker,
    CustomShaderPreparationDispatch,
) {
    let adapter_generation = NativeAdapterGeneration::from_test_serial(1);
    let target_generation = NativeTargetGeneration::from_test_serial(1);
    let mut broker = CustomShaderPreparationBroker::new(Arc::new(NativePreparationWake));
    let PaintPrimitive::GpuSurface(surface) = primitives
        .first()
        .expect("replacement custom shader primitive")
    else {
        unreachable!("replacement fixture contains a GPU surface")
    };
    let GpuSurfaceContent::CustomShader { descriptor } = &surface.content else {
        unreachable!("replacement fixture contains a custom shader")
    };
    let request = CustomShaderPreparationRequest::new(
        device.clone(),
        wgpu_device_id(device),
        adapter_generation,
        wgpu::TextureFormat::Rgba8Unorm,
        custom_shader_pipeline_key(descriptor).expect("native descriptor has a pipeline key"),
    );
    let target = CustomShaderTargetId::new(
        WindowId::dummy(),
        adapter_generation,
        target_generation,
        surface.key,
    )
    .expect("test target serial");
    assert_eq!(
        broker.request(target, request),
        CustomShaderPreparationState::Pending
    );
    let dispatch = broker
        .take_dispatch()
        .expect("pending native worker dispatch");
    (broker, dispatch)
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
    let pixels = readback_rgba(device, queue, texture);
    assert_color(&pixels[..4], [1.0, 0.0, 0.0, 1.0]);
}

fn assert_tile_color(pixels: &[u8], index: usize, expected: [f32; 4]) {
    let x = (index % TILE_GRID_WIDTH) as u32 * TILE_SIZE + TILE_SIZE / 2;
    let y = (index / TILE_GRID_WIDTH) as u32 * TILE_SIZE + TILE_SIZE / 2;
    let offset = y as usize * 256 + x as usize * 4;
    assert_color(&pixels[offset..offset + 4], expected);
}

fn assert_color(pixel: &[u8], expected: [f32; 4]) {
    let expected = expected.map(|component| (component * 255.0).round() as i16);
    let actual: Vec<_> = pixel[..4].iter().copied().map(i16::from).collect();
    assert!(
        actual
            .iter()
            .zip(expected)
            .all(|(actual, expected)| (actual - expected).abs() <= 8),
        "expected {expected:?}, got {actual:?}"
    );
}

fn readback_rgba(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
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
    let pixels = mapped.to_vec();
    drop(mapped);
    buffer.unmap();
    pixels
}
