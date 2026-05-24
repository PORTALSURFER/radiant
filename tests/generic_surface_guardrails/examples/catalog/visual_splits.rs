use super::*;

#[test]
fn modulation_matrix_example_stays_split_by_widget_paint_and_tests() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module = fs::read_to_string(manifest_dir.join("examples/modulation_matrix/mod.rs"))
        .expect("modulation matrix module root should be readable");
    let widget = fs::read_to_string(manifest_dir.join("examples/modulation_matrix/widget.rs"))
        .expect("modulation matrix widget module should be readable");
    let paint = fs::read_to_string(manifest_dir.join("examples/modulation_matrix/widget_paint.rs"))
        .expect("modulation matrix widget paint module should be readable");

    for required in [
        "mod geometry;",
        "mod model;",
        "mod paint;",
        "mod tests;",
        "mod view;",
        "mod widget;",
        "mod widget_paint;",
    ] {
        assert!(
            module.contains(required),
            "modulation matrix module root should delegate `{required}`"
        );
    }
    assert!(
        widget.contains("#[path = \"widget/input.rs\"]")
            && !widget.contains("fn handle_primary_press(")
            && !widget.contains("fn append_runtime_overlay_paint("),
        "modulation matrix widget root should keep state and geometry while input and widget trait plumbing stay delegated"
    );
    assert!(
        paint.contains("#[path = \"widget_paint/activity.rs\"]")
            && paint.contains("#[path = \"widget_paint/cell.rs\"]")
            && paint.contains("#[path = \"widget_paint/labels.rs\"]")
            && paint.contains("#[path = \"widget_paint/overlay.rs\"]"),
        "modulation matrix paint root should split activity, cell, label, and overlay concerns"
    );

    for path in [
        "examples/modulation_matrix.rs",
        "examples/modulation_matrix/geometry.rs",
        "examples/modulation_matrix/model.rs",
        "examples/modulation_matrix/mod.rs",
        "examples/modulation_matrix/paint.rs",
        "examples/modulation_matrix/tests.rs",
        "examples/modulation_matrix/view.rs",
        "examples/modulation_matrix/widget.rs",
        "examples/modulation_matrix/widget/input.rs",
        "examples/modulation_matrix/widget_paint.rs",
        "examples/modulation_matrix/widget_paint/activity.rs",
        "examples/modulation_matrix/widget_paint/cell.rs",
        "examples/modulation_matrix/widget_paint/labels.rs",
        "examples/modulation_matrix/widget_paint/overlay.rs",
    ] {
        let source = fs::read_to_string(manifest_dir.join(path))
            .unwrap_or_else(|err| panic!("{path} should be readable: {err}"));
        assert!(
            source.lines().count() <= 250,
            "{path} should stay within the example module target"
        );
    }
}
