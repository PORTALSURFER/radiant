use super::*;

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
fn native_gpu_surface_interaction_region_model_stays_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let collector = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/runtime_helpers/gpu_surface_regions.rs",
    ))
    .expect("GPU surface interaction region collector should be readable");
    let region = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/runtime_helpers/gpu_surface_regions/region.rs",
    ))
    .expect("GPU surface interaction region model should be readable");
    let tests = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/runtime_helpers/gpu_surface_regions/tests.rs",
    ))
    .expect("GPU surface interaction region tests should be readable");

    assert!(
        collector.contains("mod region;")
            && collector.contains("#[cfg(test)]")
            && collector.contains("mod tests;")
            && collector.contains(
                "pub(in crate::gui_runtime::native_vello) use region::GpuSurfaceInteractionRegion;"
            ),
        "GPU surface interaction collection should delegate region state and capability filtering"
    );
    assert!(
        !collector.contains("struct GpuSurfaceInteractionRegion")
            && !collector.contains("fn from_gpu_surface")
            && region.contains("struct GpuSurfaceInteractionRegion")
            && region.contains("fn from_gpu_surface")
            && region.contains("fn contains"),
        "GPU surface interaction region model and renderability checks should live in runtime_helpers/gpu_surface_regions/region.rs"
    );
    assert!(
        !collector.contains("fn gpu_surface_interaction_region_collection_reuses_existing_buffer")
            && tests
                .contains("fn gpu_surface_interaction_region_collection_reuses_existing_buffer")
            && tests.contains("fn gpu_surface_interaction_regions_skip_opaque_later_panels")
            && tests.contains("fn gpu_surface_interaction_regions_reject_nonfinite_geometry"),
        "GPU surface interaction collector regression tests should stay in runtime_helpers/gpu_surface_regions/tests.rs"
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
fn native_vello_shape_geometry_stays_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let shape = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene/shape.rs"),
    )
    .expect("native Vello shape encoder should be readable");
    let geometry = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene/shape/geometry.rs"),
    )
    .expect("native Vello shape geometry helper should be readable");

    assert!(
        shape.contains("mod geometry;")
            && shape.contains(
                "use geometry::{paintable_stroke_width, polygon_path, polyline_path, to_kurbo_path};"
            ),
        "shape scene encoder should delegate geometry conversion and stroke validation"
    );
    assert!(
        !shape.contains("fn polygon_path")
            && !shape.contains("fn polyline_path")
            && !shape.contains("fn to_kurbo_path")
            && !shape.contains("fn paintable_stroke_width")
            && geometry.contains("fn polygon_path")
            && geometry.contains("fn polyline_path")
            && geometry.contains("fn to_kurbo_path")
            && geometry.contains("fn paintable_stroke_width"),
        "shape path conversion and stroke renderability policy should live in scene/shape/geometry.rs"
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
fn native_external_drag_dropfiles_payload_stays_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let payload = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/external_drag/payload.rs"),
    )
    .expect("native external drag payload module should be readable");
    let dropfiles =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/external_drag/payload/dropfiles.rs",
        ))
        .expect("native external drag DROPFILES payload module should be readable");

    assert!(
        payload.contains("mod dropfiles;")
            && payload.contains("use dropfiles::build_dropfiles_payload;"),
        "external drag payload module should delegate CF_HDROP path serialization"
    );
    assert!(
        !payload.contains("fn encode_drag_paths")
            && !payload.contains("fn dropfiles_header_bytes")
            && !payload.contains("DROPFILES")
            && dropfiles.contains("fn encode_drag_paths")
            && dropfiles.contains("fn dropfiles_header_bytes")
            && dropfiles.contains("DROPFILES"),
        "DROPFILES header and UTF-16 path serialization should live in payload/dropfiles.rs"
    );
}

#[test]
fn native_external_drag_data_object_helpers_stay_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_object = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/external_drag/data_object.rs"),
    )
    .expect("native external drag data object module should be readable");
    let formats =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/external_drag/data_object/formats.rs",
        ))
        .expect("native external drag data object format helper should be readable");
    let medium =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/external_drag/data_object/medium.rs",
        ))
        .expect("native external drag data object medium helper should be readable");

    assert!(
        data_object.contains("mod formats;")
            && data_object.contains("mod medium;")
            && data_object.contains("data_object_format_matches")
            && data_object.contains("drop_effect_from_medium")
            && !data_object.contains("fn is_file_drop_format")
            && !data_object.contains("GlobalLock"),
        "external drag IDataObject implementation should delegate format matching and HGLOBAL effect decoding"
    );
    assert!(
        formats.contains("fn data_object_format_matches")
            && formats.contains("fn is_file_drop_format")
            && formats.contains("fn is_drop_effect_format")
            && formats.contains("fn uses_hglobal_storage")
            && medium.contains("fn drop_effect_from_medium")
            && medium.contains("GlobalLock")
            && medium.contains("GlobalUnlock"),
        "external drag data-object helpers should stay grouped by FORMATETC matching and STGMEDIUM decoding"
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
