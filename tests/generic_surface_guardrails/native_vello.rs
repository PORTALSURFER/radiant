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
    let gui_svg_source = fs::read_to_string(manifest_dir.join("src/gui/svg.rs"))
        .expect("generic SVG icon source should be readable");
    let gui_svg_parser = fs::read_to_string(manifest_dir.join("src/gui/svg/parser.rs"))
        .expect("generic SVG parser source should be readable");
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
        !gui_svg_source.contains("vello::kurbo") && !gui_svg_parser.contains("vello::kurbo"),
        "generic SVG icon parsing should not reach through the native Vello facade for geometry"
    );
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
fn gui_svg_keeps_icon_parser_model_and_hit_testing_in_focused_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/svg.rs"))
        .expect("generic SVG module should be readable");
    let icon = fs::read_to_string(manifest_dir.join("src/gui/svg/icon.rs"))
        .expect("generic SVG icon module should be readable");
    let model = fs::read_to_string(manifest_dir.join("src/gui/svg/model.rs"))
        .expect("generic SVG model module should be readable");
    let parser = fs::read_to_string(manifest_dir.join("src/gui/svg/parser.rs"))
        .expect("generic SVG parser module should be readable");
    let hit_test = fs::read_to_string(manifest_dir.join("src/gui/svg/hit_test.rs"))
        .expect("generic SVG hit-test module should be readable");

    for required in [
        "mod hit_test;",
        "mod icon;",
        "mod model;",
        "mod parser;",
        "pub use hit_test::point_in_svg_shapes;",
        "pub use icon::SvgIcon;",
        "pub use model::{SvgDocument, SvgShape};",
        "pub use parser::parse_svg_document;",
    ] {
        assert!(
            root.contains(required),
            "generic SVG root should keep public API re-exports while delegating `{required}`"
        );
    }
    assert!(
        icon.contains("pub struct SvgIcon") && icon.contains("PaintSvgDocument::try_from_svg"),
        "SVG icon retained-paint wrapper should live in the icon module"
    );
    assert!(
        model.contains("pub struct SvgDocument")
            && model.contains("pub struct SvgShape")
            && model.contains("enum SvgFillRule"),
        "SVG parsed document and shape state should live in the model module"
    );
    assert!(
        parser.contains("pub fn parse_svg_document")
            && parser.contains("fn collect_shapes")
            && parser.contains("fn parse_transform_list"),
        "SVG subset parsing should live in the parser module"
    );
    assert!(
        hit_test.contains("pub fn point_in_svg_shapes")
            && hit_test.contains("fn point_in_svg_shape"),
        "SVG shape hit-testing should live in the hit-test module"
    );
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
fn post_gpu_overlay_vertex_buffer_upload_is_non_panicking() {
    let source = fs::read_to_string(
        "src/gui_runtime/native_vello/generic_runtime/post_gpu_overlay/buffer.rs",
    )
    .expect("post GPU overlay vertex buffer should be readable");

    assert!(
        !source.contains(".expect(") && !source.contains(".unwrap("),
        "post GPU overlay vertex buffer upload should handle missing cached buffers without panicking"
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
fn native_gpu_surface_resource_lifecycle_stays_with_resource_cache() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let renderer = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface.rs"),
    )
    .expect("GPU surface renderer module should be readable");
    let resources = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources.rs"),
    )
    .expect("GPU surface resources module should be readable");
    let cache = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/cache.rs"),
    )
    .expect("GPU surface resource cache module should be readable");

    assert!(
        resources.contains("mod cache;")
            && resources.contains("pub(super) use cache::GpuSurfaceResourceCache;")
            && renderer.contains("resources: GpuSurfaceResourceCache"),
        "GPU surface renderer should store retained WGPU resources through a focused resource cache"
    );
    assert!(
        cache.contains("struct GpuSurfaceResourceCache")
            && cache.contains("fn prune_inactive")
            && cache.contains("fn clear")
            && !renderer.contains("textures: HashMap")
            && !renderer.contains("signal_summaries: HashMap"),
        "resource-map lifecycle should live with the resource cache, not top-level renderer fields"
    );
}

#[test]
fn native_gpu_surface_overlay_uniforms_stay_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let renderer = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface.rs"),
    )
    .expect("GPU surface renderer module should be readable");
    let passes = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/passes.rs"),
    )
    .expect("GPU surface pass module should be readable");
    let overlays = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/overlays.rs"),
    )
    .expect("GPU surface overlay uniform module should be readable");

    assert!(
        renderer.contains("mod overlays;") && renderer.contains("use overlays::*;"),
        "GPU surface renderer should route overlay uniform packing through a focused module"
    );
    assert!(
        !passes.contains("fn vertical_overlays")
            && !passes.contains("fn normalized_ratio")
            && overlays.contains("fn vertical_overlays")
            && overlays.contains("fn normalized_ratio")
            && overlays.contains("fn rgba_to_float"),
        "overlay uniform packing should not live with WGPU render-pass and scissor setup"
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
            && text_runs.contains("INLINE_SCENE_TEXT_RUNS")
            && !text_runs.contains("unsafe")
            && !text_runs.contains("ManuallyDrop")
            && !text_runs.contains("fn rebind"),
        "retained frame encoding should not own reusable text run staging buffers"
    );
}

#[test]
fn native_vello_plain_text_encoding_stays_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let scene = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene.rs"),
    )
    .expect("native Vello scene module should be readable");
    let text = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene/text.rs"),
    )
    .expect("plain text scene encoder should be readable");

    assert!(
        scene.contains("mod text;") && scene.contains("use text::encode_text;"),
        "central scene encoder should delegate plain text primitive translation"
    );
    assert!(
        !scene.contains("PaintTextAlign::Left => TextAlign::Left")
            && text.contains("fn encode_text")
            && text.contains("PaintTextAlign::Left => TextAlign::Left")
            && text.contains("baseline.unwrap_or(text.font_size)")
            && text.contains("text.rect.width().max(0.0)"),
        "plain text alignment, baseline, and width mapping should stay in the focused text encoder"
    );
}

#[test]
fn native_vello_text_input_geometry_stays_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let text_input = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene/text_input.rs"),
    )
    .expect("native Vello text-input encoder should be readable");
    let geometry = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/scene/text_input/geometry.rs"),
    )
    .expect("native Vello text-input geometry module should be readable");

    assert!(
        text_input.contains("mod geometry;")
            && text_input.contains(
                "use geometry::{caret_size, selection_rect, text_input_geometry_is_renderable};"
            )
            && text_input.contains("fn encode_text_input")
            && text_input.contains("fn draw_text_input_text")
            && text_input.contains("fn encode_block_caret"),
        "text-input encoder should own scene orchestration while delegating geometry helpers"
    );
    assert!(
        !text_input.contains("fn caret_size")
            && !text_input.contains("fn selection_rect")
            && !text_input.contains("fn text_input_geometry_is_renderable")
            && geometry.contains("fn caret_size")
            && geometry.contains("fn selection_rect")
            && geometry.contains("fn text_input_geometry_is_renderable"),
        "text-input caret sizing, selection rectangles, and renderability checks should live in scene/text_input/geometry.rs"
    );
}

#[test]
fn native_vello_text_renderer_keeps_models_and_renderability_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/text_renderer.rs"))
            .expect("native Vello text renderer should be readable");
    let model = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/text_renderer/model.rs"),
    )
    .expect("native Vello text renderer model module should be readable");
    let renderability = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/text_renderer/renderability.rs"),
    )
    .expect("native Vello text renderer renderability module should be readable");

    for required in [
        "mod model;",
        "mod renderability;",
        "pub(in crate::gui_runtime::native_vello) use model::{",
        "pub(in crate::gui_runtime::native_vello) use renderability::font_size_is_renderable;",
        "use renderability::text_run_is_renderable;",
    ] {
        assert!(
            root.contains(required),
            "native Vello text renderer root should delegate `{required}`"
        );
    }
    assert!(
        !root.contains("pub(in crate::gui_runtime::native_vello) struct SceneTextRun")
            && !root.contains("pub(in crate::gui_runtime::native_vello) struct TextLayout")
            && !root.contains("fn text_run_is_renderable"),
        "native Vello text renderer root should orchestrate rendering without owning text models or renderability policy"
    );
    assert!(
        model.contains("pub(in crate::gui_runtime::native_vello) struct SceneTextRun")
            && model.contains("impl From<&TextRun> for SceneTextRun")
            && model.contains("pub(in crate::gui_runtime::native_vello) struct TextLayout")
            && model.contains("pub(in crate::gui_runtime::native_vello) struct TextCursorStop")
            && model.contains("pub(in crate::gui_runtime::native_vello) struct TextLayoutKey")
            && model.contains("pub(in crate::gui_runtime::native_vello) struct LoadedFont"),
        "native Vello text renderer data models should live in text_renderer/model.rs"
    );
    assert!(
        renderability.contains("fn text_run_is_renderable")
            && renderability.contains("fn font_size_is_renderable")
            && renderability.contains("max_width.is_finite() && max_width > 0.0"),
        "native Vello text renderability policy should live in text_renderer/renderability.rs"
    );
}

#[test]
fn native_vello_scene_geometry_uses_explicit_kurbo_dependency() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let shape = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene/shape.rs"),
    )
    .expect("native Vello shape encoder should be readable");
    let svg = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene/svg.rs"),
    )
    .expect("native Vello SVG encoder should be readable");

    assert!(
        shape.contains("use kurbo::Stroke;") && !shape.contains("vello::kurbo::Stroke"),
        "shape scene encoding should use Radiant's explicit kurbo dependency for stroke geometry"
    );
    assert!(
        svg.contains("use kurbo::Rect as KurboRect;") && !svg.contains("vello::kurbo::Rect"),
        "SVG scene encoding should use Radiant's explicit kurbo dependency for source bounds"
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
fn native_text_field_layout_keeps_cursor_stop_windowing_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let layout =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/text_edit/layout.rs"))
            .expect("native text edit layout module should be readable");
    let cursor_stops = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/text_edit/layout/cursor_stops.rs"),
    )
    .expect("native text edit cursor-stop helpers should be readable");

    assert!(
        layout.contains("mod cursor_stops;")
            && layout.contains("use cursor_stops::{")
            && layout
                .contains("pub(in crate::gui_runtime::native_vello) struct TextFieldLayoutState")
            && layout
                .contains("pub(in crate::gui_runtime::native_vello) fn build_text_field_layout"),
        "native text-field layout root should own the layout state and delegate cursor-stop windowing"
    );
    assert!(
        !layout.contains("fn finite_stop_x")
            && !layout.contains("fn stop_local_x")
            && !layout.contains("fn visible_end_stop_index")
            && cursor_stops.contains("fn cursor_stop_x")
            && cursor_stops.contains("fn visible_end_stop_index")
            && cursor_stops.contains("fn build_visible_cursor_stops")
            && cursor_stops.contains("fn finite_stop_x")
            && cursor_stops.contains("fn stop_local_x"),
        "cursor-stop lookup, sanitization, and visible-window helpers should live in text_edit/layout/cursor_stops.rs"
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
