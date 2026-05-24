use super::*;

#[test]
fn piano_roll_example_stays_split_by_widget_input_paint_and_tests() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module = fs::read_to_string(manifest_dir.join("examples/piano_roll/mod.rs"))
        .expect("piano roll module root should be readable");
    let widget = fs::read_to_string(manifest_dir.join("examples/piano_roll/widget.rs"))
        .expect("piano roll widget module should be readable");
    let paint = fs::read_to_string(manifest_dir.join("examples/piano_roll/widget_paint.rs"))
        .expect("piano roll paint module should be readable");

    for required in [
        "mod drag;",
        "mod geometry;",
        "mod model;",
        "mod widget;",
        "mod widget_paint;",
    ] {
        assert!(
            module.contains(required),
            "piano roll module root should delegate `{required}`"
        );
    }
    assert!(
        widget.contains("#[path = \"widget/input.rs\"]")
            && !widget.contains("fn handle_primary_press(")
            && !widget.contains("fn append_runtime_overlay_paint("),
        "piano roll widget root should keep state and geometry while input and widget trait plumbing stay delegated"
    );
    assert!(
        paint.contains("#[path = \"widget_paint/grid.rs\"]")
            && paint.contains("#[path = \"widget_paint/keyboard.rs\"]")
            && paint.contains("#[path = \"widget_paint/note.rs\"]")
            && paint.contains("#[path = \"widget_paint/overlay.rs\"]"),
        "piano roll paint root should split grid, keyboard, note, and overlay concerns"
    );

    for path in [
        "examples/piano_roll.rs",
        "examples/piano_roll/drag.rs",
        "examples/piano_roll/geometry.rs",
        "examples/piano_roll/model.rs",
        "examples/piano_roll/mod.rs",
        "examples/piano_roll/paint.rs",
        "examples/piano_roll/tests.rs",
        "examples/piano_roll/view.rs",
        "examples/piano_roll/widget.rs",
        "examples/piano_roll/widget/input.rs",
        "examples/piano_roll/widget_paint.rs",
        "examples/piano_roll/widget_paint/grid.rs",
        "examples/piano_roll/widget_paint/keyboard.rs",
        "examples/piano_roll/widget_paint/note.rs",
        "examples/piano_roll/widget_paint/overlay.rs",
    ] {
        let source = fs::read_to_string(manifest_dir.join(path))
            .unwrap_or_else(|err| panic!("{path} should be readable: {err}"));
        assert!(
            source.lines().count() <= 250,
            "{path} should stay within the example module target"
        );
    }
}
