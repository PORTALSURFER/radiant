//! Opt-in native coverage for persistent custom-shader storage.
use super::*;
use crate::runtime::{
    GpuPersistentStoragePatch, GpuPersistentStorageSnapshot, GpuPersistentStorageStore,
    GpuPersistentStorageTarget, GpuPersistentStorageUpdate,
};

const STORAGE_SHADER: &str = r#"
@group(0) @binding(2) var<storage, read> values: array<f32>;
@vertex fn vertex_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
 var p = array<vec2<f32>,3>(vec2(-1.,-1.),vec2(3.,-1.),vec2(-1.,3.)); return vec4(p[index],0.,1.);
}
@fragment fn fragment_main() -> @location(0) vec4<f32> { return vec4(values[0], values[0], values[0], 1.); }
"#;

fn persistent_descriptor(bytes: &[u8]) -> GpuShaderSurfaceDescriptor {
    GpuShaderSurfaceDescriptor::new("native-persistent-storage")
        .wgsl_source(Arc::<str>::from(STORAGE_SHADER))
        .entry_point("vertex_main")
        .fragment_entry_point("fragment_main")
        .storage_identity(91)
        .storage_revision(7)
        .storage_bytes(bytes)
}

fn persistent_store(bytes: &[u8]) -> (GpuPersistentStorageStore, GpuPersistentStorageTarget) {
    let target = GpuPersistentStorageTarget::new(1.into(), FRESH_KEY, 91, 7);
    let mut store = GpuPersistentStorageStore::default();
    store
        .apply(GpuPersistentStorageUpdate::Snapshot(
            GpuPersistentStorageSnapshot::new(target, 4, bytes.len(), bytes.len(), 1, bytes)
                .expect("snapshot"),
        ))
        .expect("admit snapshot");
    (store, target)
}

fn persistent_store_with_logical_prefix(
    capacity: usize,
    initial_bytes: &[u8],
) -> (GpuPersistentStorageStore, GpuPersistentStorageTarget) {
    let target = GpuPersistentStorageTarget::new(1.into(), FRESH_KEY, 91, 7);
    let mut store = GpuPersistentStorageStore::default();
    store
        .apply(GpuPersistentStorageUpdate::Snapshot(
            GpuPersistentStorageSnapshot::new(
                target,
                4,
                capacity,
                initial_bytes.len(),
                1,
                initial_bytes,
            )
            .expect("snapshot"),
        ))
        .expect("admit snapshot");
    (store, target)
}

fn persistent_render(
    renderer: &mut GpuSurfaceRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    primitives: &[PaintPrimitive],
    store: &GpuPersistentStorageStore,
) -> (GpuSurfaceRenderStats, Vec<u8>) {
    let context = upload_plan_context(device);
    let plan = renderer.preflight_render_canvas_upload_plan_with_persistent_storage(
        context,
        primitives,
        crate::theme::DpiScale::ONE,
        &[],
        store,
    );
    let (texture, view) = render_target(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let mut target = render_target_for_test(
        device,
        queue,
        &mut encoder,
        &view,
        Some(context),
        Some(plan),
    );
    let mut occlusion = SurfaceOcclusionPlan::default();
    occlusion.preprocess(primitives);
    let stats =
        renderer.render_with_persistent_storage(&mut target, primitives, &occlusion, &[], store);
    renderer.finish_presentation_staging_belt();
    queue.submit(std::iter::once(encoder.finish()));
    let pixels = readback_rgba(device, queue, &texture);
    renderer.recall_presentation_staging_belt();
    (stats, pixels)
}

fn persistent_stage_without_submission(
    renderer: &mut GpuSurfaceRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    primitives: &[PaintPrimitive],
    store: &GpuPersistentStorageStore,
) -> GpuSurfaceRenderStats {
    let context = upload_plan_context(device);
    let plan = renderer.preflight_render_canvas_upload_plan_with_persistent_storage(
        context,
        primitives,
        crate::theme::DpiScale::ONE,
        &[],
        store,
    );
    let (_texture, view) = render_target(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let stats = {
        let mut target = render_target_for_test(
            device,
            queue,
            &mut encoder,
            &view,
            Some(context),
            Some(plan),
        );
        let mut occlusion = SurfaceOcclusionPlan::default();
        occlusion.preprocess(primitives);
        renderer.render_with_persistent_storage(&mut target, primitives, &occlusion, &[], store)
    };
    renderer.finish_presentation_staging_belt();
    drop(encoder.finish());
    stats
}

#[test]
#[ignore = "requires native GPU adapter and offscreen WGPU rendering"]
fn persistent_storage_uploads_full_then_ranges_and_replays_after_recovery() {
    let (device, queue) = native_device();
    let mut bytes = vec![0_u8; 65_536];
    bytes[..4].copy_from_slice(&0.25_f32.to_le_bytes());
    let (mut store, storage_target) = persistent_store(&bytes);
    let primitives = vec![PaintPrimitive::GpuSurface(surface(
        FRESH_KEY,
        persistent_descriptor(&bytes),
    ))];
    let mut renderer = GpuSurfaceRenderer::default();
    assert_eq!(
        stage_custom_shader_preparations(&mut renderer, &device, &primitives),
        1
    );
    let (initial, initial_pixels) =
        persistent_render(&mut renderer, &device, &queue, &primitives, &store);
    assert_eq!(
        initial
            .render_canvas_uploads
            .immutable_payload
            .logical_bytes,
        Some(65_536)
    );
    renderer.commit_pending_persistent_storage();

    store
        .apply(GpuPersistentStorageUpdate::Patch(
            GpuPersistentStoragePatch::replace(storage_target, 1, 2, 0, 0.75_f32.to_le_bytes())
                .expect("patch"),
        ))
        .expect("apply patch");
    let (warm, warm_pixels) =
        persistent_render(&mut renderer, &device, &queue, &primitives, &store);
    assert_eq!(
        warm.render_canvas_uploads.immutable_payload.logical_bytes,
        Some(4)
    );
    assert_ne!(initial_pixels, warm_pixels);
    renderer.commit_pending_persistent_storage();

    let (recovery_device, recovery_queue) = native_device();
    drop(renderer);
    let mut recovered = GpuSurfaceRenderer::default();
    assert_eq!(
        stage_custom_shader_preparations(&mut recovered, &recovery_device, &primitives),
        1
    );
    let (after_recovery, pixels) = persistent_render(
        &mut recovered,
        &recovery_device,
        &recovery_queue,
        &primitives,
        &store,
    );
    assert_eq!(
        after_recovery
            .render_canvas_uploads
            .immutable_payload
            .logical_bytes,
        Some(65_536)
    );
    assert_eq!(pixels, warm_pixels);
}

#[test]
#[ignore = "requires native GPU adapter and offscreen WGPU rendering"]
fn persistent_storage_coalesces_replays_after_abort_and_invalidates_released_incarnation() {
    let (device, queue) = native_device();
    let mut descriptor_bytes = vec![0_u8; 65_536];
    descriptor_bytes[..4].copy_from_slice(&0.25_f32.to_le_bytes());
    let (mut store, target) =
        persistent_store_with_logical_prefix(descriptor_bytes.len(), &descriptor_bytes[..4]);
    let primitives = vec![PaintPrimitive::GpuSurface(surface(
        FRESH_KEY,
        persistent_descriptor(&descriptor_bytes),
    ))];
    let mut renderer = GpuSurfaceRenderer::default();
    assert_eq!(
        stage_custom_shader_preparations(&mut renderer, &device, &primitives),
        1
    );

    let (initial, _) = persistent_render(&mut renderer, &device, &queue, &primitives, &store);
    assert_eq!(
        initial
            .render_canvas_uploads
            .immutable_payload
            .logical_bytes,
        Some(65_536)
    );
    renderer.commit_pending_persistent_storage();

    store
        .apply(GpuPersistentStorageUpdate::Patch(
            GpuPersistentStoragePatch::append(target, 1, 2, 0_u32.to_le_bytes()).expect("append"),
        ))
        .expect("apply append");
    store
        .apply(GpuPersistentStorageUpdate::Patch(
            GpuPersistentStoragePatch::replace(target, 2, 3, 0, 0.5_f32.to_le_bytes())
                .expect("first overlapping replacement"),
        ))
        .expect("apply replacement");
    store
        .apply(GpuPersistentStorageUpdate::Patch(
            GpuPersistentStoragePatch::replace(target, 3, 4, 0, 0.75_f32.to_le_bytes())
                .expect("second overlapping replacement"),
        ))
        .expect("apply replacement");
    let (coalesced, pixels) =
        persistent_render(&mut renderer, &device, &queue, &primitives, &store);
    assert_eq!(
        coalesced
            .render_canvas_uploads
            .immutable_payload
            .logical_bytes,
        Some(8),
        "append and repeated replacement are uploaded as their coalesced ranges"
    );
    assert_color(&pixels[..4], [0.75, 0.75, 0.75, 1.0]);
    renderer.commit_pending_persistent_storage();

    store
        .apply(GpuPersistentStorageUpdate::Patch(
            GpuPersistentStoragePatch::replace(target, 4, 5, 0, 0.25_f32.to_le_bytes())
                .expect("retry replacement"),
        ))
        .expect("apply retry replacement");
    let vetoed =
        persistent_stage_without_submission(&mut renderer, &device, &queue, &primitives, &store);
    assert_eq!(
        vetoed.render_canvas_uploads.immutable_payload.logical_bytes,
        Some(4)
    );
    renderer.abort_pending_persistent_storage();
    let (replayed, replayed_pixels) =
        persistent_render(&mut renderer, &device, &queue, &primitives, &store);
    assert_eq!(
        replayed
            .render_canvas_uploads
            .immutable_payload
            .logical_bytes,
        Some(4),
        "an unsubmitted encoder must leave the cursor retryable"
    );
    assert_color(&replayed_pixels[..4], [0.25, 0.25, 0.25, 1.0]);
    renderer.commit_pending_persistent_storage();

    store
        .apply(GpuPersistentStorageUpdate::Release(target))
        .expect("release exact target");
    store
        .apply(GpuPersistentStorageUpdate::Snapshot(
            GpuPersistentStorageSnapshot::new(
                target,
                4,
                descriptor_bytes.len(),
                4,
                1,
                0.75_f32.to_le_bytes(),
            )
            .expect("same-fence replacement snapshot"),
        ))
        .expect("restore target with a fresh incarnation");
    let (reincarnated, reincarnated_pixels) =
        persistent_render(&mut renderer, &device, &queue, &primitives, &store);
    assert_eq!(
        reincarnated
            .render_canvas_uploads
            .immutable_payload
            .logical_bytes,
        Some(65_536),
        "a released same-fence snapshot must not reuse a prior GPU cursor"
    );
    assert_color(&reincarnated_pixels[..4], [0.75, 0.75, 0.75, 1.0]);
}
