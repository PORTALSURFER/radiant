use super::*;

#[test]
fn spectrogram_example_stays_split_by_model_widget_paint_and_tests() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = example_source(&manifest_dir, "spectrogram", "examples/spectrogram.rs");
    let paint = fs::read_to_string(manifest_dir.join("examples/spectrogram/widget/paint.rs"))
        .expect("spectrogram paint module should be readable");

    for required in [
        "#[path = \"spectrogram/model.rs\"]",
        "#[path = \"spectrogram/widget.rs\"]",
        "#[path = \"spectrogram/tests.rs\"]",
    ] {
        assert!(
            root.contains(required),
            "spectrogram root should delegate `{required}`"
        );
    }
    assert!(
        paint.contains("#[path = \"paint/cells.rs\"]")
            && paint.contains("#[path = \"paint/grid.rs\"]")
            && paint.contains("#[path = \"paint/labels.rs\"]")
            && paint.contains("#[path = \"paint/overlay.rs\"]")
            && paint.contains("#[path = \"paint/primitives.rs\"]"),
        "spectrogram paint root should split cells, grid, labels, overlay, and primitive helpers"
    );

    for path in [
        "examples/spectrogram.rs",
        "examples/spectrogram/model.rs",
        "examples/spectrogram/tests.rs",
        "examples/spectrogram/widget.rs",
        "examples/spectrogram/widget/paint.rs",
        "examples/spectrogram/widget/paint/cells.rs",
        "examples/spectrogram/widget/paint/color.rs",
        "examples/spectrogram/widget/paint/grid.rs",
        "examples/spectrogram/widget/paint/labels.rs",
        "examples/spectrogram/widget/paint/overlay.rs",
        "examples/spectrogram/widget/paint/primitives.rs",
    ] {
        let source = fs::read_to_string(manifest_dir.join(path))
            .unwrap_or_else(|err| panic!("{path} should be readable: {err}"));
        assert!(
            source.lines().count() <= 250,
            "{path} should stay within the example module target"
        );
    }
}

#[test]
fn eq_editor_example_stays_split_by_model_widget_paint_and_tests() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = example_source(&manifest_dir, "eq_editor", "examples/eq_editor.rs");
    let widget = fs::read_to_string(manifest_dir.join("examples/eq_editor/widget.rs"))
        .expect("eq editor widget module should be readable");
    let paint = fs::read_to_string(manifest_dir.join("examples/eq_editor/widget/paint.rs"))
        .expect("eq editor paint module should be readable");

    for required in [
        "#[path = \"eq_editor/model.rs\"]",
        "#[path = \"eq_editor/widget.rs\"]",
        "#[path = \"eq_editor/tests.rs\"]",
    ] {
        assert!(
            root.contains(required),
            "eq editor root should delegate `{required}`"
        );
    }
    assert!(
        widget.contains("#[path = \"widget/input.rs\"]")
            && !widget.contains("fn handle_primary_press(")
            && !widget.contains("fn append_runtime_overlay_paint("),
        "eq editor widget root should keep state and geometry while input and widget trait plumbing stay delegated"
    );
    assert!(
        paint.contains("#[path = \"paint/grid.rs\"]")
            && paint.contains("#[path = \"paint/primitives.rs\"]"),
        "eq editor paint root should delegate grid and primitive helpers"
    );

    for path in [
        "examples/eq_editor.rs",
        "examples/eq_editor/model.rs",
        "examples/eq_editor/tests.rs",
        "examples/eq_editor/widget.rs",
        "examples/eq_editor/widget/geometry.rs",
        "examples/eq_editor/widget/input.rs",
        "examples/eq_editor/widget/paint.rs",
        "examples/eq_editor/widget/paint/grid.rs",
        "examples/eq_editor/widget/paint/primitives.rs",
        "examples/eq_editor/widget/response.rs",
    ] {
        let source = fs::read_to_string(manifest_dir.join(path))
            .unwrap_or_else(|err| panic!("{path} should be readable: {err}"));
        assert!(
            source.lines().count() <= 250,
            "{path} should stay within the example module target"
        );
    }
}
