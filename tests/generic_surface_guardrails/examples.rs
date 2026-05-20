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

    let undocumented = registered_example_names(&manifest)
        .into_iter()
        .filter(|name| !docs.contains(&format!("cargo run --example {name}")))
        .collect::<Vec<_>>();
    assert!(
        undocumented.is_empty(),
        "registered examples should be discoverable in docs/API.md:\n{}",
        undocumented.join("\n")
    );

    assert_no_local_windows_paths_in_examples(&manifest_dir);
}

#[test]
fn api_docs_map_examples_to_target_areas() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .expect("Radiant Cargo.toml should be readable");
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("Radiant API docs should be readable");
    let registered_examples = registered_example_names(&manifest);

    for required in [
        "| Target area | Focused examples |",
        "| First-use application API | `hello_world`, `generic_native`, `counter` |",
        "| State, commands, and background work |",
        "| Custom widgets and retained GPU surfaces |",
        "| Advanced creative-tool surfaces |",
        "| Text, diagnostics, and performance inspection |",
        "| Window and host integration |",
    ] {
        assert!(
            docs.contains(required),
            "API docs should map maintained examples to target capability area `{required}`"
        );
    }

    for example in [
        "hello_world",
        "generic_native",
        "counter",
        "todo_list",
        "message_routing",
        "background_loading",
        "status_bar",
        "layout_rows_columns",
        "grid_gallery",
        "scroll",
        "sizing",
        "virtualized_list",
        "styling",
        "theme_playground",
        "widget_gallery",
        "toolbar_icons",
        "focus_controls",
        "context_menu",
        "tree_and_details",
        "folder_browser",
        "custom_widget",
        "gpu_surface",
        "custom_shader_surface",
        "gpu_surface_stack_overlay",
        "waveform_view",
        "node_editor",
        "timeline_editor",
        "plugin_panel",
        "split_workspace",
        "typography",
        "layout_diagnostics",
        "rendering_benchmark",
        "host_surface_frame",
        "multi_window_manifest",
        "popup_window",
    ] {
        assert!(
            registered_examples.iter().any(|name| name == example),
            "target-area map should only name registered examples: {example}"
        );
        assert!(
            docs.contains(&format!("`{example}`")),
            "target-area map should include `{example}`"
        );
    }
}

#[test]
fn examples_do_not_hide_dead_code() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let example_dir = manifest_dir.join("examples");
    let violations = rust_sources_under(&example_dir)
        .into_iter()
        .filter(|path| {
            fs::read_to_string(path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
                .contains("allow(dead_code)")
        })
        .map(|path| relative_path(&manifest_dir, &path))
        .collect::<Vec<_>>();

    assert!(
        violations.is_empty(),
        "maintained examples should compile, cfg, test, or remove example code instead of hiding dead-code warnings:\n{}",
        violations.join("\n")
    );
}

fn assert_no_local_windows_paths_in_examples(manifest_dir: &Path) {
    let example_dir = manifest_dir.join("examples");
    let violations = rust_sources_under(&example_dir)
        .into_iter()
        .filter(|path| {
            let source = fs::read_to_string(path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            source.contains("C:\\") || source.contains("C:/")
        })
        .map(|path| relative_path(manifest_dir, &path))
        .collect::<Vec<_>>();

    assert!(
        violations.is_empty(),
        "maintained examples should use arguments, environment variables, or temp paths instead of hardcoded local Windows paths:\n{}",
        violations.join("\n")
    );
}

fn registered_example_names(manifest: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_example = false;

    for line in manifest.lines().map(str::trim) {
        match line {
            "[[example]]" => in_example = true,
            line if line.starts_with("[[") || line.starts_with('[') => in_example = false,
            line if in_example && line.starts_with("name = ") => {
                if let Some(name) = quoted_toml_value(line) {
                    names.push(name.to_owned());
                }
            }
            _ => {}
        }
    }

    names
}

fn quoted_toml_value(line: &str) -> Option<&str> {
    line.split_once('"')
        .and_then(|(_, tail)| tail.split_once('"'))
        .map(|(value, _)| value)
}
