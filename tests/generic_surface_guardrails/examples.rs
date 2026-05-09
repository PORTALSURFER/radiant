use super::*;

#[test]
fn generic_native_example_is_registered_and_uses_application_builders() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .expect("Radiant Cargo.toml should be readable");
    let example = fs::read_to_string(manifest_dir.join("examples/generic_native.rs"))
        .expect("generic_native example should be readable");

    assert!(
        manifest.contains("[[example]]")
            && manifest.contains("name = \"generic_native\"")
            && manifest.contains("path = \"examples/generic_native.rs\""),
        "generic_native should be an explicit checked Cargo example target"
    );

    for required in [
        "use radiant::prelude::*;",
        "radiant::app(DemoState::default())",
        ".size(460, 128)",
        ".min_size(400, 112)",
        ".update_command",
        "Command::message",
        "Command::request_repaint",
        "row([",
        "text(format!(",
        ".preferred_size(260.0, 32.0)",
        "button(\"Increment\")",
        ".message(DemoMessage::ButtonPressed)",
        ".size(112.0, 40.0)",
    ] {
        assert!(
            example.contains(required),
            "generic_native example should exercise `{required}`"
        );
    }
    for deprecated_first_use in [
        "NativeRunOptions",
        "declarative_command_runtime_bridge",
        "run_native_vello_runtime",
        "SurfaceChild",
        "Arc<UiSurface",
    ] {
        assert!(
            !example.contains(deprecated_first_use),
            "generic_native example should not use old first-use boilerplate `{deprecated_first_use}`"
        );
    }
}

#[test]
fn focused_examples_are_registered_and_stay_on_application_builders() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .expect("Radiant Cargo.toml should be readable");

    for (name, required) in [
        (
            "form",
            vec!["text_input(", ".bind(", "toggle(", ".on_change("],
        ),
        (
            "animation_showcase",
            vec![".animation(", ".on_frame(", "AnimationMessage::Frame"],
        ),
        (
            "background_loading",
            vec![".update_with(", "context.spawn(", "LoadingMessage::Loaded"],
        ),
        (
            "typography",
            vec![".wrap()", ".truncate()", ".baseline(", ".fill_width()"],
        ),
        (
            "layout_rows_columns",
            vec!["row([", "column([", ".padding(", ".fill_width()"],
        ),
        (
            "grid_gallery",
            vec![
                "grid_with_gaps(",
                ".wrap()",
                ".fill()",
                "WidgetTone::Accent",
            ],
        ),
        ("list", vec!["list(", "list_row(", ".fill_height()"]),
        (
            "virtualized_list",
            vec!["radiant::app(", "virtual_list(", "list_row(", ".update("],
        ),
        (
            "inspector_panel",
            vec![
                "selectable_property_panel(",
                "PropertyRow::new(",
                ".on_change(",
            ],
        ),
        (
            "context_menu",
            vec!["context_menu_overlay(", "MenuItem::new(", ".danger()"],
        ),
        (
            "styling",
            vec![
                ".primary()",
                ".danger()",
                ".subtle()",
                ".hoverable()",
                "toggle(",
            ],
        ),
        ("scroll", vec!["scroll_column(", ".fill_height()"]),
        (
            "sizing",
            vec![".size(", ".min_size(", ".preferred_size(", ".fill_width()"],
        ),
        (
            "message_routing",
            vec![
                ".update_command",
                "Command::message",
                "Command::request_repaint",
            ],
        ),
        ("keys", vec![".key(", "list_row(", ".reverse()"]),
        (
            "focus_controls",
            vec![
                ".update_with(",
                "context.focus(",
                "text_input(",
                ".message(",
            ],
        ),
        (
            "custom_widget",
            vec![
                "impl Widget for StatusChip",
                "WidgetOutput::custom(",
                "dispatch_input(",
            ],
        ),
    ] {
        let path = format!("examples/{name}.rs");
        let source = fs::read_to_string(manifest_dir.join(&path))
            .unwrap_or_else(|_| panic!("{name} example should be readable"));

        assert!(
            manifest.contains(&format!("name = \"{name}\""))
                && manifest.contains(&format!("path = \"{path}\"")),
            "{name} should be an explicit checked Cargo example target"
        );
        assert!(
            source.contains("use radiant::prelude::*;")
                || source.contains("use radiant::prelude as ui;"),
            "{name} should use the application prelude"
        );
        for required in required {
            assert!(
                source.contains(required),
                "{name} example should exercise `{required}`"
            );
        }
        let mut deprecated_first_use = vec![
            "NativeRunOptions",
            "declarative_command_runtime_bridge",
            "run_native_vello_runtime",
            "SurfaceChild",
            "Arc<UiSurface",
        ];
        if name != "custom_widget" {
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
fn examples_are_checked_portable_sandboxes() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .expect("Radiant Cargo.toml should be readable");
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("Radiant API docs should be readable");

    assert!(
        !manifest.contains("test = false"),
        "maintained examples should participate in `cargo test --examples`"
    );
    assert!(
        docs.contains("cargo test --examples")
            && docs.contains("RADIANT_FOLDER_BROWSER_ROOT")
            && docs.contains("RADIANT_WAVEFORM_PATH"),
        "API docs should describe checked examples and their optional input paths"
    );

    for path in ["examples/folder_browser.rs", "examples/waveform_view.rs"] {
        let source = fs::read_to_string(manifest_dir.join(path))
            .unwrap_or_else(|_| panic!("{path} should be readable"));
        assert!(
            !source.contains("C:\\") && !source.contains("C:/"),
            "{path} should not hardcode local Windows paths"
        );
    }
}
