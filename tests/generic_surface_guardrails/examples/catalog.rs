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
