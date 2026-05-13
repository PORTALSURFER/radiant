use super::*;

#[path = "examples/catalog.rs"]
mod catalog;

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
            && docs.contains("RADIANT_WAVEFORM_PATH")
            && docs.contains("cargo run --example volume_slider")
            && docs.contains("slider(...)"),
        "API docs should describe checked examples, parameter-control examples, and their optional input paths"
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
