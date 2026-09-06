//! Opt-in native coverage for persistent custom-shader storage.
use super::*;
use crate::runtime::{
    GpuPersistentStoragePatch, GpuPersistentStorageSnapshot, GpuPersistentStorageStore,
    GpuPersistentStorageTarget, GpuPersistentStorageUpdate,
};
use std::hash::{Hash, Hasher};
use std::io::Write;

const STORAGE_SHADER: &str = r#"
@group(0) @binding(2) var<storage, read> values: array<f32>;
@vertex fn vertex_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
 var p = array<vec2<f32>,3>(vec2(-1.,-1.),vec2(3.,-1.),vec2(-1.,3.)); return vec4(p[index],0.,1.);
}
@fragment fn fragment_main() -> @location(0) vec4<f32> { return vec4(values[0], values[0], values[0], 1.); }
"#;

fn persistent_fixture_device() -> (wgpu::Device, wgpu::Queue, wgpu::AdapterInfo) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        ..Default::default()
    }))
    .expect("persistent storage fixture requires native adapter");
    let info = adapter.get_info();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("radiant_persistent_storage_fixture"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    }))
    .expect("persistent storage fixture requires native device");
    (device, queue, info)
}

fn persistent_descriptor(bytes: &[u8]) -> GpuShaderSurfaceDescriptor {
    GpuShaderSurfaceDescriptor::new("native-persistent-storage")
        .wgsl_source(Arc::<str>::from(STORAGE_SHADER))
        .entry_point("vertex_main")
        .fragment_entry_point("fragment_main")
        .storage_identity(91)
        .storage_revision(7)
        .storage_bytes(bytes)
}

fn bulk_descriptor(bytes: &[u8]) -> GpuShaderSurfaceDescriptor {
    GpuShaderSurfaceDescriptor::new("native-persistent-storage")
        .wgsl_source(Arc::<str>::from(STORAGE_SHADER))
        .entry_point("vertex_main")
        .fragment_entry_point("fragment_main")
        .storage_bytes(bytes)
}

fn ordered_surface(occurrence: usize, descriptor: GpuShaderSurfaceDescriptor) -> PaintGpuSurface {
    let mut surface = surface(FRESH_KEY, descriptor);
    let width = TARGET_SIZE as f32 / 3.0;
    surface.rect = Rect::from_min_size(
        Point::new(width * occurrence as f32, 0.0),
        Vector2::new(width, TARGET_SIZE as f32),
    );
    surface
}

fn persistent_store(bytes: &[u8]) -> (GpuPersistentStorageStore, GpuPersistentStorageTarget) {
    let target = GpuPersistentStorageTarget::new(1_u64, FRESH_KEY, 91, 7);
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
    let target = GpuPersistentStorageTarget::new(1_u64, FRESH_KEY, 91, 7);
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
    assert!(stats.persistent_storage_complete);
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
    assert!(stats.persistent_storage_complete);
    stats
}

fn assert_cpu_reference_gray(pixels: &[u8], value: f32) {
    assert_eq!(
        pixels.len(),
        TARGET_SIZE as usize * TARGET_SIZE as usize * 4
    );
    for pixel in pixels.chunks_exact(4) {
        assert_color(pixel, [value, value, value, 1.0]);
    }
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

fn record_native_evidence(
    stages: &[(&str, &[u8])],
    upload_bytes: &[Option<u64>],
    adapter: &wgpu::AdapterInfo,
) {
    let Some(root) = std::env::var_os("RADIANT_PERSISTENT_STORAGE_OUTPUT_DIR") else {
        return;
    };
    let source_revision = std::env::var("RADIANT_PERSISTENT_STORAGE_SOURCE_REVISION")
        .expect("RADIANT_PERSISTENT_STORAGE_SOURCE_REVISION is required with output capture");
    std::fs::create_dir_all(&root).expect("create persistent storage output directory");
    let root = std::path::PathBuf::from(root);
    let mut stage_records = Vec::new();
    for (name, pixels) in stages {
        let path = root.join(format!("persistent-storage-{name}.rgba"));
        let mut rgba = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .expect("exclusively create persistent-storage RGBA evidence");
        rgba.write_all(pixels)
            .expect("write persistent-storage RGBA evidence");
        stage_records.push(serde_json::json!({
            "name": name,
            "rgba_file": path.file_name().and_then(|value| value.to_str()),
            "rgba_default_hasher": hash_bytes(pixels),
        }));
    }
    let record = serde_json::json!({
        "source_revision": source_revision,
        "shader_default_hasher": hash_bytes(STORAGE_SHADER.as_bytes()),
        "target_size": TARGET_SIZE,
        "storage_capacity_bytes": 65_536,
        "upload_logical_bytes": upload_bytes,
        "adapter": adapter.name.as_str(),
        "backend": format!("{:?}", adapter.backend),
        "device_type": format!("{:?}", adapter.device_type),
        "driver": adapter.driver.as_str(),
        "driver_info": adapter.driver_info.as_str(),
        "device_setup": "native WGPU adapter with default limits",
        "stages": stage_records,
    });
    let path = root.join("persistent-storage-native-evidence.json");
    let mut json = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .expect("exclusively create persistent-storage metadata");
    json.write_all(&serde_json::to_vec_pretty(&record).expect("serialize persistent evidence"))
        .expect("write persistent-storage metadata");
}

#[test]
#[ignore = "requires native GPU adapter and offscreen WGPU rendering"]
fn persistent_storage_uploads_full_then_ranges_and_replays_after_recovery() {
    let (device, queue, adapter) = persistent_fixture_device();
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
    assert_cpu_reference_gray(&initial_pixels, 0.25);
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
    assert_cpu_reference_gray(&warm_pixels, 0.75);
    renderer.commit_pending_persistent_storage();

    let (unchanged, unchanged_pixels) =
        persistent_render(&mut renderer, &device, &queue, &primitives, &store);
    assert_eq!(
        unchanged
            .render_canvas_uploads
            .immutable_payload
            .logical_bytes,
        Some(0)
    );
    assert_eq!(unchanged_pixels, warm_pixels);

    drop(renderer);
    let (recovery_device, recovery_queue, _recovery_adapter) = persistent_fixture_device();
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
    record_native_evidence(
        &[
            ("initial", &initial_pixels),
            ("warm", &warm_pixels),
            ("recovered", &pixels),
        ],
        &[
            initial
                .render_canvas_uploads
                .immutable_payload
                .logical_bytes,
            warm.render_canvas_uploads.immutable_payload.logical_bytes,
            unchanged
                .render_canvas_uploads
                .immutable_payload
                .logical_bytes,
            after_recovery
                .render_canvas_uploads
                .immutable_payload
                .logical_bytes,
        ],
        &adapter,
    );
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
        Some(65_536),
        "a conservative abort forgets touched GPU state so it replays the full shadow"
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

#[test]
#[ignore = "requires native GPU adapter and offscreen WGPU rendering"]
fn persistent_storage_switches_through_bulk_and_recovers_after_submitted_abort() {
    let (device, queue) = native_device();
    let mut persistent_bytes = vec![0_u8; 65_536];
    persistent_bytes[..4].copy_from_slice(&0.25_f32.to_le_bytes());
    let (mut store, target) = persistent_store(&persistent_bytes);
    let persistent_primitives = vec![PaintPrimitive::GpuSurface(surface(
        FRESH_KEY,
        persistent_descriptor(&persistent_bytes),
    ))];
    let mut renderer = GpuSurfaceRenderer::default();
    assert_eq!(
        stage_custom_shader_preparations(&mut renderer, &device, &persistent_primitives),
        1
    );
    let (initial, _) = persistent_render(
        &mut renderer,
        &device,
        &queue,
        &persistent_primitives,
        &store,
    );
    assert_eq!(
        initial
            .render_canvas_uploads
            .immutable_payload
            .logical_bytes,
        Some(65_536)
    );
    renderer.commit_pending_persistent_storage();

    let mut bulk_bytes = persistent_bytes.clone();
    bulk_bytes[..4].copy_from_slice(&0.75_f32.to_le_bytes());
    let ordered_primitives = vec![
        PaintPrimitive::GpuSurface(ordered_surface(0, persistent_descriptor(&persistent_bytes))),
        PaintPrimitive::GpuSurface(ordered_surface(1, bulk_descriptor(&bulk_bytes))),
        PaintPrimitive::GpuSurface(ordered_surface(2, persistent_descriptor(&persistent_bytes))),
    ];
    assert_eq!(
        stage_custom_shader_preparations(&mut renderer, &device, &ordered_primitives),
        1
    );
    let (ordered, ordered_pixels) =
        persistent_render(&mut renderer, &device, &queue, &ordered_primitives, &store);
    assert_eq!(
        ordered.custom_shader.surfaces_rendered, 3,
        "the three same-key ordered occurrences must not be occlusion-skipped"
    );
    assert!(
        ordered
            .render_canvas_uploads
            .immutable_payload
            .logical_bytes
            .is_some_and(|bytes| bytes >= 65_536),
        "the final persistent occurrence replays after the intervening bulk reset"
    );
    assert_cpu_reference_gray(&ordered_pixels, 0.25);
    renderer.commit_pending_persistent_storage();

    store
        .apply(GpuPersistentStorageUpdate::Patch(
            GpuPersistentStoragePatch::replace(target, 1, 2, 0, 0.75_f32.to_le_bytes())
                .expect("post-submit patch"),
        ))
        .expect("apply post-submit patch");
    let (submitted, submitted_pixels) = persistent_render(
        &mut renderer,
        &device,
        &queue,
        &persistent_primitives,
        &store,
    );
    assert_eq!(
        submitted
            .render_canvas_uploads
            .immutable_payload
            .logical_bytes,
        Some(4)
    );
    assert_cpu_reference_gray(&submitted_pixels, 0.75);
    // This models a native veto after queue submission: the actual GPU write
    // is unknowable to the retained cursor, so recovery must start from the
    // complete CPU shadow instead of trusting the staged revision.
    renderer.abort_pending_persistent_storage();
    let (same_device_retry, retry_pixels) = persistent_render(
        &mut renderer,
        &device,
        &queue,
        &persistent_primitives,
        &store,
    );
    assert_eq!(
        same_device_retry
            .render_canvas_uploads
            .immutable_payload
            .logical_bytes,
        Some(65_536),
        "a submitted-veto cursor invalidation forces a full same-device replay"
    );
    assert_cpu_reference_gray(&retry_pixels, 0.75);
    renderer.commit_pending_persistent_storage();
    drop(renderer);

    let (recovery_device, recovery_queue) = native_device();
    let mut recovered = GpuSurfaceRenderer::default();
    assert_eq!(
        stage_custom_shader_preparations(&mut recovered, &recovery_device, &persistent_primitives),
        1
    );
    let (recovered_stats, recovered_pixels) = persistent_render(
        &mut recovered,
        &recovery_device,
        &recovery_queue,
        &persistent_primitives,
        &store,
    );
    assert_eq!(
        recovered_stats
            .render_canvas_uploads
            .immutable_payload
            .logical_bytes,
        Some(65_536)
    );
    assert_cpu_reference_gray(&recovered_pixels, 0.75);
}
