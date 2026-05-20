use super::*;

#[test]
fn native_vello_scene_texture_rendering_stays_out_of_present_driver() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/generic_runtime.rs"))
            .expect("generic native Vello module should be readable");
    let present = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/present.rs"),
    )
    .expect("present driver should be readable");
    let scene_texture = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene_texture.rs"),
    )
    .expect("scene texture renderer should be readable");

    assert!(
        module.contains("mod scene_texture;")
            && module.contains("use scene_texture::render_scene_texture_if_needed;"),
        "generic runtime should expose the Vello scene texture renderer as a focused module"
    );
    assert!(
        !present.contains("renderer.render_to_texture(")
            && scene_texture.contains("renderer.render_to_texture(")
            && scene_texture.contains("frame.scene_texture_dirty = false")
            && scene_texture.contains("frame.mark_composited_base_dirty();"),
        "present driver should delegate dirty scene texture rendering to the focused scene_texture module"
    );
}

#[test]
fn native_frame_preparation_stays_out_of_present_driver() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/generic_runtime.rs"))
            .expect("generic native Vello module should be readable");
    let present = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/present.rs"),
    )
    .expect("present driver should be readable");
    let frame_prepare = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/frame_prepare.rs"),
    )
    .expect("native frame-preparation module should be readable");

    assert!(
        module.contains("mod frame_prepare;"),
        "generic runtime should expose frame preparation as a focused module"
    );
    assert!(
        present.contains("self.refresh_deferred_surface_if_needed(&mut profile);")
            && present.contains("self.paint_transient_overlays(&mut profile);"),
        "present driver should orchestrate frame preparation without owning its implementation"
    );
    assert!(
        !present.contains("self.core.refresh_surface()")
            && !present.contains("paint_transient_overlay(")
            && frame_prepare.contains("fn refresh_deferred_surface_if_needed")
            && frame_prepare.contains("fn paint_transient_overlays")
            && frame_prepare.contains("collect_gpu_surface_interaction_regions"),
        "deferred model refresh, paint-plan refresh, and transient overlay painting should stay in frame_prepare"
    );
}

#[test]
fn native_timed_frame_drain_does_not_recompute_selected_cadence() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lifecycle = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/lifecycle.rs"),
    )
    .expect("generic native lifecycle should be readable");
    let runner = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/runner.rs"),
    )
    .expect("generic native runner should be readable");

    assert!(
        lifecycle.contains("let cadence = timed_frame_cadence(")
            && lifecycle.contains("TimedFrameCadence::DrainNow { next_wake }")
            && lifecycle.contains("self.drain_timed_frame_now("),
        "native lifecycle should compute timed-frame cadence once and drain directly when due"
    );
    assert!(
        runner.contains("fn drain_timed_frame_now")
            && !runner.contains("fn drain_due_timed_frame")
            && !runner.contains("match timed_frame_cadence("),
        "runner timed-frame drain should not recompute cadence already selected by lifecycle"
    );
}

#[test]
fn native_render_surface_target_size_stays_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/generic_runtime.rs"))
            .expect("generic native Vello module should be readable");
    let present = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/present.rs"),
    )
    .expect("present driver should be readable");
    let composited = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/composited_base.rs"),
    )
    .expect("composited base presenter should be readable");
    let surface_size = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/surface_size.rs"),
    )
    .expect("render surface size module should be readable");

    assert!(
        module.contains("mod surface_size;")
            && module.contains("use surface_size::RenderSurfacePixelSize;"),
        "generic runtime should own render-surface sizing through a focused module"
    );
    assert!(
        present.contains("RenderSurfacePixelSize::from_surface(surface)")
            && composited
                .matches("RenderSurfacePixelSize::from_surface(surface)")
                .count()
                == 2,
        "present and composited-base WGPU targets should use the shared render-surface size helper"
    );
    assert!(
        !present.contains("surface.config.width as f32")
            && !composited.contains("surface.config.width as f32")
            && surface_size.contains("pub(super) struct RenderSurfacePixelSize")
            && surface_size.contains("fn logical_size"),
        "direct WGPU target size conversion should stay centralized instead of repeating raw config casts"
    );
}

#[test]
fn native_surface_texture_acquire_stays_with_surface_lifecycle() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let present = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/present.rs"),
    )
    .expect("present driver should be readable");
    let surface = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/surface.rs"),
    )
    .expect("surface lifecycle module should be readable");

    assert!(
        present.contains("self.acquire_present_surface_texture(event_loop, &window)")
            && !present.contains("get_current_texture()")
            && !present.contains("SurfaceError::OutOfMemory"),
        "present driver should delegate WGPU surface texture acquisition and recovery"
    );
    assert!(
        surface.contains("fn acquire_present_surface_texture")
            && surface.contains("get_current_texture()")
            && surface.contains("SurfaceError::Lost | wgpu::SurfaceError::Outdated")
            && surface.contains("SurfaceError::OutOfMemory"),
        "surface texture acquisition and surface-error handling should stay with surface lifecycle"
    );
}

#[test]
fn native_surface_backend_policy_stays_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let surface = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/surface.rs"),
    )
    .expect("surface lifecycle module should be readable");
    let backend = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/surface/backend.rs"),
    )
    .expect("surface backend policy module should be readable");

    assert!(
        surface.contains("mod backend;")
            && surface.contains("render_context_for_options(&self.options)")
            && !surface.contains("fn wgpu_backends")
            && !surface.contains("InstanceDescriptor"),
        "surface lifecycle should delegate explicit WGPU backend policy"
    );
    assert!(
        backend.contains("fn render_context_for_options")
            && backend.contains("fn wgpu_backends")
            && backend.contains("NativeGpuBackend::Auto")
            && backend.contains("wgpu::InstanceDescriptor"),
        "WGPU backend selection and render-context construction should live in surface/backend.rs"
    );
}

#[test]
fn native_window_platform_attributes_stay_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let window = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/window.rs"),
    )
    .expect("native window attribute module should be readable");
    let platform = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/window/platform.rs"),
    )
    .expect("native window platform attribute module should be readable");
    let tests = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests/window_policy.rs"),
    )
    .expect("native window policy tests should be readable");

    assert!(
        window.contains("mod platform;")
            && window.contains("platform::apply_drag_and_drop_attributes")
            && window.contains("platform::apply_popup_attributes")
            && !window.contains("WindowAttributesExtWindows")
            && !window.contains("cfg(target_os"),
        "generic window attributes should delegate platform extension hooks"
    );
    assert!(
        platform.contains("#[cfg(target_os = \"windows\")]")
            && platform.contains("#[cfg(not(target_os = \"windows\"))]")
            && platform.contains("WindowAttributesExtWindows")
            && platform.contains("with_drag_and_drop(true)")
            && platform.contains("with_skip_taskbar(true)"),
        "target-specific window attribute extensions should stay in window/platform.rs"
    );
    assert!(
        tests.contains("generic_native_window_uses_configured_drag_and_drop_policy")
            && tests.contains("generic_native_window_applies_floating_popup_policy"),
        "generic window policy tests should continue covering platform-neutral decisions"
    );
}

#[test]
fn native_vello_present_diagnostics_stay_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let present = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/present.rs"),
    )
    .expect("present driver should be readable");
    let diagnostics = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/present/diagnostics.rs"),
    )
    .expect("present diagnostics helper should be readable");

    assert!(
        present.contains("mod diagnostics;")
            && present.contains("native_frame_diagnostics(")
            && !present.contains("fn native_frame_diagnostics"),
        "present driver should delegate structured frame diagnostics projection"
    );
    assert!(
        diagnostics.contains("fn native_frame_diagnostics")
            && diagnostics.contains("NativeSceneDiagnostics")
            && diagnostics.contains("NativeGpuSurfaceDiagnostics")
            && diagnostics.contains("NativeFrameTimingDiagnostics"),
        "native frame diagnostics projection should live in present/diagnostics.rs"
    );
}

#[test]
fn native_gpu_upload_byte_casts_stay_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/generic_runtime.rs"))
            .expect("generic native Vello module should be readable");
    let upload = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_upload_bytes.rs"),
    )
    .expect("GPU upload byte helper should be readable");
    let encoding = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/encoding.rs"),
    )
    .expect("GPU surface encoding module should be readable");
    let vertex = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/post_gpu_overlay/vertex.rs"),
    )
    .expect("post GPU overlay vertex module should be readable");

    assert!(
        module.contains("mod gpu_upload_bytes;")
            && upload.contains("unsafe trait GpuUploadBytes")
            && upload.contains("from_raw_parts"),
        "generic runtime should own raw WGPU upload byte views in one explicit helper"
    );
    assert!(
        encoding.contains("upload_value_as_bytes")
            && encoding.contains("upload_slice_as_bytes")
            && vertex.contains("upload_slice_as_bytes")
            && !encoding.contains("from_raw_parts")
            && !vertex.contains("from_raw_parts"),
        "renderer upload structs should delegate byte casting instead of duplicating pointer logic"
    );
}
