use super::*;

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
fn native_vello_scene_encoders_use_explicit_imports() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let encoder_paths = [
        "src/gui_runtime/native_vello/generic_runtime/scene/frame.rs",
        "src/gui_runtime/native_vello/generic_runtime/scene/image.rs",
        "src/gui_runtime/native_vello/generic_runtime/scene/shape.rs",
        "src/gui_runtime/native_vello/generic_runtime/scene/shape/geometry.rs",
        "src/gui_runtime/native_vello/generic_runtime/scene/svg.rs",
    ];

    for path in encoder_paths {
        let source = fs::read_to_string(manifest_dir.join(path))
            .unwrap_or_else(|err| panic!("{path} should be readable: {err}"));
        assert!(
            !source.contains("use crate::gui_runtime::native_vello::*;"),
            "{path} should import the native Vello dependencies it actually uses"
        );
    }
}
