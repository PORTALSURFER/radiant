//! Native Vello and renderer-boundary structural guardrails.

use std::{fs, path::PathBuf};

use super::{relative_path, rust_sources_under};

#[test]
fn gui_runtime_public_facade_exports_generic_runtime_entrypoints() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/gui_runtime/mod.rs"))
        .expect("gui_runtime module should be readable");
    let public_exports = source
        .split("pub use native_vello::{")
        .nth(1)
        .and_then(|tail| tail.split("};").next())
        .expect("gui_runtime should have a native_vello public export block");

    for required in [
        "NativeGenericRunReport",
        "NativeGenericRunError",
        "NativeGenericRuntimeArtifacts",
        "NativeStartupTimingArtifact",
        "run_native_vello_runtime",
        "run_native_vello_runtime_with_artifacts",
    ] {
        assert!(
            public_exports.contains(required),
            "radiant::gui_runtime should expose generic runtime API `{required}`"
        );
    }
    assert!(
        source.contains("pub struct RuntimeRunReport<Artifacts, Error = String>"),
        "radiant::gui_runtime should expose a generic runtime report envelope"
    );
}

#[test]
fn public_vector_paint_primitives_do_not_expose_vello_path_types() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/path.rs"))
        .expect("vector paint primitive source should be readable");
    let shape_source =
        fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/shape.rs"))
            .expect("shape paint primitive source should be readable");

    for forbidden in ["vello::kurbo", "BezPath", "pub type PaintTransform"] {
        assert!(
            !source.contains(forbidden),
            "public vector paint primitives should remain backend-neutral; found `{forbidden}`"
        );
    }
    assert!(
        !shape_source.contains("pub struct PaintTransform"),
        "paint shapes should depend on the shared backend-neutral path transform instead of owning it"
    );
    for required in [
        "pub struct PaintPath",
        "pub enum PaintPathCommand",
        "pub struct PaintTransform",
    ] {
        assert!(
            source.contains(required),
            "public vector paint primitives should expose backend-neutral `{required}`"
        );
    }
}

#[test]
fn native_vello_scene_encoder_keeps_custom_surfaces_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scene = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene.rs"),
    )
    .expect("native Vello scene encoder should be readable");
    let custom_surface = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene/custom_surface.rs"),
    )
    .expect("custom surface scene encoder should be readable");

    assert!(
        scene.contains("mod custom_surface;")
            && scene.contains("use custom_surface::encode_custom_surface;"),
        "central scene encoder should delegate retained custom-surface rendering"
    );
    assert!(
        !scene.contains("render_retained_surface(")
            && custom_surface.contains("render_retained_surface(")
            && custom_surface.contains("retained_cache.cached_frame")
            && custom_surface.contains("encode_custom_surface_fallback"),
        "retained custom-surface cache/bridge/fallback logic should stay in the focused custom-surface encoder"
    );
}

#[test]
fn composited_base_frame_cache_avoids_post_mutation_expect() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/composited_base.rs"),
    )
    .expect("composited base presenter should be readable");
    let source = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/composited_base/frame.rs"),
    )
    .expect("composited base frame cache should be readable");
    let ensure_body = source
        .split("pub(super) fn ensure")
        .nth(1)
        .and_then(|tail| tail.split("fn new").next())
        .expect("CompositedBaseFrame::ensure should be present");

    assert!(
        module.contains("mod frame;")
            && module.contains("pub(super) use frame::CompositedBaseFrame;"),
        "composited base presentation should delegate cached texture ownership to the frame module"
    );
    assert!(
        !module.contains("struct CompositedBaseFrame")
            && source.contains("struct CompositedBaseFrame"),
        "cached composited base texture state should stay out of the presenter module"
    );
    assert!(
        ensure_body.contains(".is_some_and(|frame| frame.matches(device, width, height, format))")
            && ensure_body.contains("frame.insert(Self::new(device, width, height, format))"),
        "CompositedBaseFrame::ensure should reuse device-matching frames and install replacements directly"
    );
    assert!(
        !ensure_body.contains(".expect(") && !ensure_body.contains(".unwrap("),
        "CompositedBaseFrame::ensure should not assert the Option state after mutating it"
    );
}

#[test]
fn gpu_surface_render_stats_stay_in_focused_diagnostics_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface.rs"),
    )
    .expect("GPU surface renderer module should be readable");
    let types = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types.rs"),
    )
    .expect("GPU surface type bucket should be readable");
    let stats = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/stats.rs"),
    )
    .expect("GPU surface stats module should be readable");

    assert!(
        module.contains("mod stats;")
            && module.contains("pub(super) use stats::GpuSurfaceRenderStats;"),
        "GPU surface renderer should re-export render stats from the focused stats module"
    );
    assert!(
        !types.contains("struct GpuSurfaceRenderStats")
            && stats.contains("struct GpuSurfaceRenderStats")
            && stats.contains("atlas_texture_uploads")
            && stats.contains("signal_body_encode_elapsed")
            && stats.contains("composite_binding_rebuilds"),
        "render profiling counters should stay out of resource/cache-key type definitions"
    );
}

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
fn native_gpu_surface_wheel_coalescing_stays_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/generic_runtime.rs"))
            .expect("generic native Vello module should be readable");
    let interaction = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface_interaction.rs"),
    )
    .expect("GPU surface interaction module should be readable");
    let wheel = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface_wheel.rs"),
    )
    .expect("GPU surface wheel module should be readable");

    assert!(
        module.contains("mod gpu_surface_wheel;")
            && module.contains("use gpu_surface_wheel::PendingGpuSurfaceWheel;"),
        "generic runtime should keep pending wheel state owned by the wheel module"
    );
    assert!(
        !interaction.contains("struct PendingGpuSurfaceWheel")
            && !interaction.contains("fn flush_pending_gpu_surface_wheel")
            && wheel.contains("struct PendingGpuSurfaceWheel")
            && wheel.contains("fn flush_pending_gpu_surface_wheel")
            && wheel.contains("coalesced_wheel"),
        "wheel coalescing should stay separate from pointer hover overlay interaction"
    );
}

#[test]
fn native_vello_scene_text_run_buffer_stays_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scene = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene.rs"),
    )
    .expect("native Vello scene module should be readable");
    let frame = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene/frame.rs"),
    )
    .expect("retained frame encoder should be readable");
    let text_runs = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene/text_runs.rs"),
    )
    .expect("scene text run buffer module should be readable");

    assert!(
        scene.contains("mod text_runs;")
            && scene.contains(
                "pub(in crate::gui_runtime::native_vello) use text_runs::SceneTextRunBuffer;"
            )
            && scene.contains("use text_runs::flush_text_runs;"),
        "scene module should route reusable text run staging through the focused text_runs module"
    );
    assert!(
        !frame.contains("struct SceneTextRunBuffer")
            && !frame.contains("fn flush_text_runs")
            && text_runs.contains("struct SceneTextRunBuffer")
            && text_runs.contains("fn flush_text_runs")
            && text_runs.contains("INLINE_SCENE_TEXT_RUNS"),
        "retained frame encoding should not own reusable text run staging buffers"
    );
}

#[test]
fn native_text_input_rendering_keeps_utf8_clamping_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module = fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/text_edit.rs"))
        .expect("native text edit module should be readable");
    let state =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/text_edit/state.rs"))
            .expect("native text edit state should be readable");
    let boundary =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/text_edit/boundary.rs"))
            .expect("native text edit boundary helpers should be readable");

    assert!(
        module.contains("mod boundary;") && state.contains("use super::boundary::"),
        "native text-input rendering state should consume UTF-8 boundary policy from a focused module"
    );
    assert!(
        !state.contains("fn clamp_to_char_boundary")
            && boundary.contains("fn clamp_to_char_boundary")
            && !boundary.contains("fn previous_char_boundary")
            && !boundary.contains("fn next_char_boundary"),
        "native text-input rendering should keep only the UTF-8 boundary policy it uses"
    );
}

#[test]
fn native_vello_runtime_does_not_hide_dead_code() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let runtime_dir = manifest_dir.join("src/gui_runtime/native_vello");
    let violations = rust_sources_under(&runtime_dir)
        .into_iter()
        .filter(|path| {
            fs::read_to_string(path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
                .contains("#[allow(dead_code)]")
        })
        .map(|path| relative_path(&manifest_dir, &path))
        .collect::<Vec<_>>();

    assert!(
        violations.is_empty(),
        "native Vello runtime modules should export, test, or remove code instead of hiding dead-code warnings:\n{}",
        violations.join("\n")
    );
}
