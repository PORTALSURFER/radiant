use super::*;

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
    let tests = fs::read_to_string(manifest_dir.join("src/gui/svg/tests.rs"))
        .expect("generic SVG root tests should be readable");
    let icon = fs::read_to_string(manifest_dir.join("src/gui/svg/icon.rs"))
        .expect("generic SVG icon module should be readable");
    let model = fs::read_to_string(manifest_dir.join("src/gui/svg/model.rs"))
        .expect("generic SVG model module should be readable");
    let parser = fs::read_to_string(manifest_dir.join("src/gui/svg/parser.rs"))
        .expect("generic SVG parser module should be readable");
    let parser_numbers = fs::read_to_string(manifest_dir.join("src/gui/svg/parser/numbers.rs"))
        .expect("generic SVG parser number module should be readable");
    let parser_transform = fs::read_to_string(manifest_dir.join("src/gui/svg/parser/transform.rs"))
        .expect("generic SVG parser transform module should be readable");
    let hit_test = fs::read_to_string(manifest_dir.join("src/gui/svg/hit_test.rs"))
        .expect("generic SVG hit-test module should be readable");

    for required in [
        "mod hit_test;",
        "mod icon;",
        "mod model;",
        "mod parser;",
        "pub use hit_test::point_in_svg_shapes;",
        "pub use model::{SvgDocument, SvgShape};",
        "pub use parser::parse_svg_document;",
    ] {
        assert!(
            root.contains(required),
            "generic SVG root should keep public API re-exports while delegating `{required}`"
        );
    }
    assert!(
        root.contains("pub use icon::")
            && root.contains("SvgIcon")
            && root.contains("SvgIconTintCache")
            && root.contains("svg_with_current_color"),
        "generic SVG root should expose icon facade items from the icon module"
    );
    assert!(
        root.contains("#[path = \"svg/tests.rs\"]")
            && !root.contains("fn svg_icon_appends_retained_svg_primitive")
            && !root.contains("fn svg_subset_parser_supports_evenodd_cutouts"),
        "generic SVG root behavior tests should stay delegated"
    );
    assert!(
        tests.contains("fn svg_icon_appends_retained_svg_primitive")
            && tests.contains("fn svg_icon_try_from_svg_reports_parse_errors")
            && tests.contains("fn svg_subset_parser_supports_evenodd_cutouts")
            && tests.contains("fn svg_subset_parser_applies_group_transforms"),
        "generic SVG root behavior coverage should live in svg/tests.rs"
    );
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
            && parser.contains("mod numbers;")
            && parser.contains("mod transform;")
            && !parser.contains("fn parse_transform_list")
            && !parser.contains("fn parse_number_list"),
        "SVG document traversal should live in the parser root while helper parsing stays delegated"
    );
    assert!(
        parser_numbers.contains("fn parse_number_list")
            && parser_numbers.contains("fn parse_number"),
        "SVG numeric list parsing should live in parser/numbers.rs"
    );
    assert!(
        parser_transform.contains("fn parse_transform_list")
            && parser_transform.contains("fn parse_single_transform")
            && parser_transform.contains("parse_number_list(args)"),
        "SVG transform parsing should live in parser/transform.rs"
    );
    assert!(
        hit_test.contains("pub fn point_in_svg_shapes")
            && hit_test.contains("fn point_in_svg_shape"),
        "SVG shape hit-testing should live in the hit-test module"
    );
}

#[test]
fn native_vello_svg_encoder_transforms_svg_content_not_only_clip() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let encoder = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/scene/svg.rs"),
    )
    .expect("native Vello SVG encoder should be readable");

    assert!(
        encoder.contains("let mut svg_scene = Scene::new();")
            && encoder.contains("vello_svg::append_tree_with(&mut svg_scene")
            && encoder.contains("scene.push_clip_layer(Fill::NonZero, transform, &source_bounds);")
            && encoder.contains("scene.append(&svg_scene, Some(transform));"),
        "Vello layer transforms only affect the clip path; SVG content must be appended with the destination transform"
    );
    assert!(
        !encoder.contains("scene.push_layer("),
        "SVG encoding should not rely on push_layer transform for drawing commands"
    );
}
