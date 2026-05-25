use super::*;

#[test]
fn mixer_console_example_stays_split_by_panel_paint_input_and_tests() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module = fs::read_to_string(manifest_dir.join("examples/mixer_console/mod.rs"))
        .expect("mixer console module root should be readable");
    let panel = fs::read_to_string(manifest_dir.join("examples/mixer_console/panel.rs"))
        .expect("mixer console panel module should be readable");
    let paint = fs::read_to_string(manifest_dir.join("examples/mixer_console/panel_paint.rs"))
        .expect("mixer console paint module should be readable");
    let strip =
        fs::read_to_string(manifest_dir.join("examples/mixer_console/panel_paint/strip.rs"))
            .expect("mixer console strip paint module should be readable");
    let tests = fs::read_to_string(manifest_dir.join("examples/mixer_console/tests.rs"))
        .expect("mixer console tests module should be readable");

    for required in ["mod model;", "mod panel;", "mod panel_paint;", "mod tests;"] {
        assert!(
            module.contains(required),
            "mixer console module root should delegate `{required}`"
        );
    }
    assert!(
        panel.contains("#[path = \"panel/geometry.rs\"]")
            && panel.contains("#[path = \"panel/input.rs\"]")
            && panel.contains("#[path = \"panel/interaction.rs\"]"),
        "mixer console panel should split geometry, input, and interaction helpers"
    );
    assert!(
        paint.contains("#[path = \"panel_paint/fader.rs\"]")
            && paint.contains("#[path = \"panel_paint/meter.rs\"]")
            && paint.contains("#[path = \"panel_paint/overlay.rs\"]")
            && paint.contains("#[path = \"panel_paint/strip.rs\"]"),
        "mixer console paint should split fader, meter, overlay, and strip concerns"
    );
    assert!(
        strip.contains("#[path = \"strip/controls.rs\"]")
            && strip.contains("#[path = \"strip/footer.rs\"]")
            && strip.contains("#[path = \"strip/sends.rs\"]")
            && strip.contains("#[path = \"strip/shell.rs\"]")
            && strip.contains("#[path = \"strip/style.rs\"]"),
        "mixer console strip paint should split controls, footer, sends, shell, and style concerns"
    );
    assert!(
        tests.contains("#[path = \"tests/model_behavior.rs\"]")
            && tests.contains("#[path = \"tests/panel_interaction.rs\"]")
            && tests.contains("#[path = \"tests/panel_paint.rs\"]")
            && tests.contains("#[path = \"tests/runtime.rs\"]"),
        "mixer console tests should stay grouped by behavior, panel paint, panel input, and runtime concerns"
    );

    for path in [
        "examples/mixer_console.rs",
        "examples/mixer_console/model.rs",
        "examples/mixer_console/panel.rs",
        "examples/mixer_console/panel/geometry.rs",
        "examples/mixer_console/panel/input.rs",
        "examples/mixer_console/panel/interaction.rs",
        "examples/mixer_console/panel_paint.rs",
        "examples/mixer_console/panel_paint/fader.rs",
        "examples/mixer_console/panel_paint/meter.rs",
        "examples/mixer_console/panel_paint/overlay.rs",
        "examples/mixer_console/panel_paint/strip.rs",
        "examples/mixer_console/panel_paint/strip/controls.rs",
        "examples/mixer_console/panel_paint/strip/footer.rs",
        "examples/mixer_console/panel_paint/strip/sends.rs",
        "examples/mixer_console/panel_paint/strip/shell.rs",
        "examples/mixer_console/panel_paint/strip/style.rs",
        "examples/mixer_console/paint.rs",
        "examples/mixer_console/tests.rs",
        "examples/mixer_console/tests/model_behavior.rs",
        "examples/mixer_console/tests/panel_interaction.rs",
        "examples/mixer_console/tests/panel_paint.rs",
        "examples/mixer_console/tests/runtime.rs",
        "examples/mixer_console/view.rs",
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
    let model = fs::read_to_string(manifest_dir.join("examples/piano_roll/model.rs"))
        .expect("piano roll model module should be readable");
    assert!(
        model.contains("#[path = \"model/editing.rs\"]")
            && model.contains("#[path = \"model/note.rs\"]")
            && model.contains("#[path = \"model/viewport.rs\"]")
            && model.contains("pub(crate) use note::PianoNote")
            && model.contains("pub(crate) use viewport::PianoRollViewport"),
        "piano roll model root should delegate editing, note, and viewport domains while preserving its public model boundary"
    );
    assert!(
        widget.contains("#[path = \"widget/input.rs\"]")
            && widget.contains("#[path = \"widget/navigation_input.rs\"]")
            && widget.contains("#[path = \"widget/pointer_input.rs\"]")
            && widget.contains("#[path = \"widget/velocity_input.rs\"]")
            && !widget.contains("fn handle_primary_press(")
            && !widget.contains("fn append_runtime_overlay_paint("),
        "piano roll widget root should keep state and geometry while input, navigation input, pointer input, velocity input, and widget trait plumbing stay delegated"
    );
    assert!(
        paint.contains("#[path = \"widget_paint/grid.rs\"]")
            && paint.contains("#[path = \"widget_paint/keyboard.rs\"]")
            && paint.contains("#[path = \"widget_paint/note.rs\"]")
            && paint.contains("#[path = \"widget_paint/overlay.rs\"]")
            && paint.contains("#[path = \"widget_paint/velocity.rs\"]")
            && !paint.contains("#[cfg(any())]")
            && !paint.contains("fn append_key_row(")
            && !paint.contains("fn append_velocity_pillar("),
        "piano roll paint root should split grid, keyboard, note, overlay, and velocity concerns"
    );
    let tests = fs::read_to_string(manifest_dir.join("examples/piano_roll/tests.rs"))
        .expect("piano roll tests module should be readable");
    assert!(
        tests.contains("#[path = \"tests/model_behavior.rs\"]")
            && tests.contains("#[path = \"tests/paint_static.rs\"]")
            && tests.contains("#[path = \"tests/note_drag.rs\"]")
            && tests.contains("#[path = \"tests/selection.rs\"]")
            && tests.contains("#[path = \"tests/wheel_navigation.rs\"]")
            && tests.contains("#[path = \"tests/pan_navigation.rs\"]")
            && tests.contains("#[path = \"tests/marquee_stress.rs\"]")
            && tests.contains("#[path = \"tests/marquee_selection.rs\"]")
            && tests.contains("#[path = \"tests/velocity_paint.rs\"]")
            && tests.contains("#[path = \"tests/velocity_drag.rs\"]")
            && tests.contains("#[path = \"tests/hover_overlay.rs\"]")
            && tests.contains("#[path = \"tests/keyboard_hover.rs\"]")
            && tests.contains("#[path = \"tests/runtime.rs\"]"),
        "piano roll tests should stay grouped by model, paint, input, navigation, marquee, velocity, hover, keyboard, and runtime concerns"
    );

    for path in [
        "examples/piano_roll.rs",
        "examples/piano_roll/drag.rs",
        "examples/piano_roll/geometry.rs",
        "examples/piano_roll/model.rs",
        "examples/piano_roll/model/editing.rs",
        "examples/piano_roll/model/note.rs",
        "examples/piano_roll/model/viewport.rs",
        "examples/piano_roll/mod.rs",
        "examples/piano_roll/paint.rs",
        "examples/piano_roll/tests.rs",
        "examples/piano_roll/tests/hover_overlay.rs",
        "examples/piano_roll/tests/keyboard_hover.rs",
        "examples/piano_roll/tests/marquee_selection.rs",
        "examples/piano_roll/tests/marquee_stress.rs",
        "examples/piano_roll/tests/model_behavior.rs",
        "examples/piano_roll/tests/note_drag.rs",
        "examples/piano_roll/tests/paint_static.rs",
        "examples/piano_roll/tests/pan_navigation.rs",
        "examples/piano_roll/tests/runtime.rs",
        "examples/piano_roll/tests/selection.rs",
        "examples/piano_roll/tests/velocity_drag.rs",
        "examples/piano_roll/tests/velocity_paint.rs",
        "examples/piano_roll/tests/wheel_navigation.rs",
        "examples/piano_roll/view.rs",
        "examples/piano_roll/widget.rs",
        "examples/piano_roll/widget/input.rs",
        "examples/piano_roll/widget/navigation_input.rs",
        "examples/piano_roll/widget/pointer_input.rs",
        "examples/piano_roll/widget/velocity_input.rs",
        "examples/piano_roll/widget_paint.rs",
        "examples/piano_roll/widget_paint/grid.rs",
        "examples/piano_roll/widget_paint/keyboard.rs",
        "examples/piano_roll/widget_paint/note.rs",
        "examples/piano_roll/widget_paint/overlay.rs",
        "examples/piano_roll/widget_paint/velocity.rs",
    ] {
        let source = fs::read_to_string(manifest_dir.join(path))
            .unwrap_or_else(|err| panic!("{path} should be readable: {err}"));
        assert!(
            source.lines().count() <= 250,
            "{path} should stay within the example module target"
        );
    }
}
