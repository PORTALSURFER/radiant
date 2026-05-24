use super::*;

#[path = "catalog/contracts.rs"]
mod contracts;
#[path = "catalog/support.rs"]
mod support;

use contracts::{
    SEPARATELY_COVERED_EXAMPLES, focused_example_contracts, has_focused_example_contract,
};
use support::{example_source, registered_example_names};

#[test]
fn registered_examples_have_complete_guardrail_coverage() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .expect("Radiant Cargo.toml should be readable");
    let mut uncovered = registered_example_names(&manifest)
        .into_iter()
        .filter(|name| !SEPARATELY_COVERED_EXAMPLES.contains(&name.as_str()))
        .filter(|name| !has_focused_example_contract(name))
        .collect::<Vec<_>>();
    uncovered.sort();

    assert!(
        uncovered.is_empty(),
        "registered examples should have focused guardrail coverage or a named separate guardrail:\n{}",
        uncovered.join("\n")
    );
}

#[test]
fn focused_examples_are_registered_and_keep_expected_public_contracts() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .expect("Radiant Cargo.toml should be readable");

    for (name, required) in focused_example_contracts() {
        let path = format!("examples/{name}.rs");
        let source = example_source(&manifest_dir, name, &path);

        assert!(
            manifest.contains(&format!("name = \"{name}\""))
                && manifest.contains(&format!("path = \"{path}\"")),
            "{name} should be an explicit checked Cargo example target"
        );
        assert!(
            name == "paint_helpers"
                || source.contains("use radiant::prelude::*;")
                || source.contains("use radiant::prelude as ui;"),
            "{name} should use the application prelude"
        );
        for required in required {
            if required.contains('/') {
                assert!(
                    manifest_dir.join(required).exists(),
                    "{name} example should keep `{required}`"
                );
            } else {
                assert!(
                    source.contains(required),
                    "{name} example should exercise `{required}`"
                );
            }
        }
        let mut deprecated_first_use = vec![
            "NativeRunOptions",
            "declarative_command_runtime_bridge",
            "run_native_vello_runtime",
            "SurfaceChild",
            "Arc<UiSurface",
        ];
        if !matches!(
            name,
            "custom_widget"
                | "eq_editor"
                | "gpu_surface_stack_overlay"
                | "arrangement_shell"
                | "mixer_console"
                | "modulation_matrix"
                | "piano_roll"
                | "spectrogram"
                | "timeline_editor"
                | "toolbar_icons"
                | "waveform_view"
        ) {
            deprecated_first_use.push("WidgetSizing");
        }

        for deprecated_first_use in deprecated_first_use {
            assert!(
                !source.contains(deprecated_first_use),
                "{name} example should not use old first-use boilerplate `{deprecated_first_use}`"
            );
        }
    }
}

#[test]
fn spectrogram_example_stays_split_by_model_widget_paint_and_tests() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = example_source(&manifest_dir, "spectrogram", "examples/spectrogram.rs");

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

    for path in [
        "examples/spectrogram.rs",
        "examples/spectrogram/model.rs",
        "examples/spectrogram/tests.rs",
        "examples/spectrogram/widget.rs",
        "examples/spectrogram/widget/paint.rs",
        "examples/spectrogram/widget/paint/color.rs",
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

#[test]
fn mixer_console_example_stays_split_by_panel_paint_input_and_tests() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module = fs::read_to_string(manifest_dir.join("examples/mixer_console/mod.rs"))
        .expect("mixer console module root should be readable");
    let panel = fs::read_to_string(manifest_dir.join("examples/mixer_console/panel.rs"))
        .expect("mixer console panel module should be readable");
    let paint = fs::read_to_string(manifest_dir.join("examples/mixer_console/panel_paint.rs"))
        .expect("mixer console paint module should be readable");
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
