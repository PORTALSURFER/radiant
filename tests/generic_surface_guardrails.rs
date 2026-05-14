//! Structural guardrails for Radiant's generic public surface.
//!
//! The boundary is proven through package layout, dependency direction, public
//! exports, standalone examples, and behavior tests. These checks avoid token
//! policing so hosts can choose their own domain language outside Radiant.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use radiant::{
    layout::Vector2,
    runtime::{SurfaceNode, UiSurface, WidgetMessageMapper},
    widgets::{TextWidget, WidgetOutput, WidgetSizing},
};

const GENERIC_SOURCE_ROOTS: &[&str] = &[
    "src/runtime",
    "src/widgets",
    "src/theme.rs",
    "src/gui/automation.rs",
    "src/gui/badge.rs",
    "src/gui/chrome.rs",
    "src/gui/feedback.rs",
    "src/gui/focus.rs",
    "src/gui/form.rs",
    "src/gui/fingerprint.rs",
    "src/gui/frame.rs",
    "src/gui/input.rs",
    "src/gui/invalidation.rs",
    "src/gui/layout_core",
    "src/gui/list.rs",
    "src/gui/paint.rs",
    "src/gui/panel.rs",
    "src/gui/range.rs",
    "src/gui/repaint.rs",
    "src/gui/retained.rs",
    "src/gui/selection.rs",
    "src/gui/shortcuts.rs",
    "src/gui/snapshot.rs",
    "src/gui/svg.rs",
    "src/gui/text_layout.rs",
    "src/gui/types.rs",
    "src/gui/visualization.rs",
];

const EXEMPT_TOP_LEVEL_GUI_FILES: &[&str] = &["src/gui/mod.rs"];

const REQUIRED_BEHAVIOR_TESTS: &[&str] = &[
    "app_runtime_api.rs",
    "application_builder_public_api.rs",
    "custom_widget_public_api.rs",
    "generic_surface_guardrails.rs",
    "layout_public_api.rs",
    "runtime_bridge_public_api.rs",
    "runtime_surface_public_api.rs",
    "surface_hover_public_api.rs",
    "surface_node_public_api.rs",
    "surface_scroll_public_api.rs",
    "surface_widget_helpers_public_api.rs",
    "widgets_primitive_behaviors.rs",
    "widgets_public_api.rs",
];

#[cfg(test)]
#[path = "generic_surface_guardrails/docs.rs"]
mod docs;

#[cfg(test)]
#[path = "generic_surface_guardrails/examples.rs"]
mod examples;

#[test]
fn top_level_gui_primitives_are_classified_for_boundary_coverage() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let gui_dir = manifest_dir.join("src/gui");
    let mut unclassified = Vec::new();

    let entries = fs::read_dir(&gui_dir)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", gui_dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| panic!("failed to read entry in {}: {err}", gui_dir.display()))
            .path();
        if !path.is_file()
            || path.extension().and_then(|extension| extension.to_str()) != Some("rs")
        {
            continue;
        }

        let relative = relative_path(&manifest_dir, &path);
        if !GENERIC_SOURCE_ROOTS.contains(&relative.as_str())
            && !EXEMPT_TOP_LEVEL_GUI_FILES.contains(&relative.as_str())
        {
            unclassified.push(relative);
        }
    }

    unclassified.sort();
    assert!(
        unclassified.is_empty(),
        "top-level src/gui/*.rs files must be classified for boundary coverage:\n{}",
        unclassified.join("\n")
    );
}

#[test]
fn radiant_manifest_is_independent_of_parent_workspace_crates() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest_path = manifest_dir.join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", manifest_path.display()));
    let uncommented = strip_toml_comments(&manifest);
    let mut violations = Vec::new();

    for (line_index, line) in uncommented.lines().enumerate() {
        let compact = line
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>();
        if compact.contains("path=\"..") || compact.contains("workspace=true") {
            violations.push(format!(
                "Cargo.toml:{} must not depend on parent workspace crates",
                line_index + 1
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Radiant must remain independently buildable:\n{}",
        violations.join("\n")
    );
}

#[test]
fn default_features_stay_empty_for_standalone_builds() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo = fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .expect("Radiant Cargo.toml should be readable");
    let features = cargo
        .split("[features]")
        .nth(1)
        .and_then(|tail| tail.split("\n[").next())
        .expect("Cargo.toml should define a [features] table");

    assert!(
        features.lines().any(|line| line.trim() == "default = []"),
        "Radiant default features must stay empty"
    );
}

#[test]
fn public_module_tree_exposes_one_progressive_api_surface() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("Radiant lib.rs should be readable");
    let public_modules = public_module_names(&lib);
    let expected = BTreeSet::from([
        "gui".to_owned(),
        "gui_runtime".to_owned(),
        "layout".to_owned(),
        "prelude".to_owned(),
        "runtime".to_owned(),
        "theme".to_owned(),
        "widgets".to_owned(),
    ]);

    assert_eq!(
        public_modules, expected,
        "Radiant's crate root should expose only generic public modules"
    );
    assert!(
        !manifest_dir.join("src/compat.rs").exists()
            && rust_sources_under(&manifest_dir.join("src/compat")).is_empty(),
        "compatibility adapter source files belong outside the generic Radiant crate"
    );
}

#[test]
fn behavior_test_suite_is_explicit_and_local_to_generic_surfaces() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tests_dir = manifest_dir.join("tests");
    let mut test_files = fs::read_dir(&tests_dir)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", tests_dir.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|err| {
                    panic!("failed to read entry in {}: {err}", tests_dir.display())
                })
                .path()
        })
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("rs"))
        .map(|path| {
            path.file_name()
                .and_then(|file_name| file_name.to_str())
                .expect("test file should have utf-8 name")
                .to_owned()
        })
        .collect::<Vec<_>>();
    test_files.sort();

    assert_eq!(
        test_files, REQUIRED_BEHAVIOR_TESTS,
        "Radiant integration tests should stay focused on generic layout, runtime, widget, and boundary behavior"
    );
    assert!(
        !tests_dir.join("shots").exists()
            && !manifest_dir
                .join("src/gui_runtime/native_vello/tests")
                .exists()
            && !manifest_dir
                .join("src/gui_runtime/native_vello/tests.rs")
                .exists(),
        "renderer snapshots and backend fixture trees should live with their owning host or backend suite"
    );
}

#[test]
fn runtime_widgets_accept_boxed_widgets_and_dynamic_messages() {
    #[derive(Clone, Debug, PartialEq)]
    struct CustomPayload(&'static str);

    #[derive(Debug, PartialEq)]
    enum HostMessage {
        Custom(&'static str),
    }

    let widget_id = 91;
    let boxed_widget = Box::new(TextWidget::new(
        widget_id,
        "boxed",
        WidgetSizing::fixed(Vector2::new(80.0, 24.0)),
    ));
    let surface = UiSurface::new(SurfaceNode::custom_widget_box(
        boxed_widget,
        WidgetMessageMapper::dynamic(|output| {
            output
                .typed_ref::<CustomPayload>()
                .map(|payload| HostMessage::Custom(payload.0))
        }),
    ));

    assert!(surface.find_widget(widget_id).is_some());
    assert_eq!(
        surface.dispatch_widget_output(widget_id, WidgetOutput::custom(CustomPayload("open"))),
        Some(HostMessage::Custom("open"))
    );
}

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
        ensure_body.contains(".is_some_and(|frame| frame.matches(width, height, format))")
            && ensure_body.contains("frame.insert(Self::new(device, width, height, format))"),
        "CompositedBaseFrame::ensure should reuse matching frames and install replacements directly"
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
fn native_text_edit_utf8_boundary_policy_stays_in_focused_module() {
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
        module.contains("mod boundary;") && state.contains("use super::boundary::{"),
        "native text editor state should consume UTF-8 boundary policy from a focused module"
    );
    assert!(
        !state.contains("fn clamp_to_char_boundary")
            && boundary.contains("fn clamp_to_char_boundary")
            && boundary.contains("fn previous_char_boundary")
            && boundary.contains("fn next_char_boundary"),
        "UTF-8 boundary navigation should stay separate from mutable editor state"
    );
}

fn public_module_names(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub mod "))
        .filter_map(|tail| tail.split([';', '{']).next())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}

fn relative_path(manifest_dir: &Path, path: &Path) -> String {
    path.strip_prefix(manifest_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn rust_sources_under(path: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    if !path.exists() {
        return sources;
    }
    if path.is_dir() {
        for entry in fs::read_dir(path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
        {
            let entry = entry
                .unwrap_or_else(|err| panic!("failed to read entry in {}: {err}", path.display()))
                .path();
            sources.extend(rust_sources_under(&entry));
        }
    } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
        sources.push(path.to_owned());
    }
    sources
}

fn strip_toml_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split_once('#').map_or(line, |(before, _)| before))
        .collect::<Vec<_>>()
        .join("\n")
}
