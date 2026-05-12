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
            "volume_slider",
            vec![
                "slider(",
                ".primary()",
                ".on_change(",
                "checkbox(",
                "TextAlign::Right",
            ],
        ),
        (
            "sample_source_list",
            vec![
                "list(",
                "list_row_id(",
                "selectable(",
                "button(\"+\")",
                "button(\"-\")",
                "selected_id",
            ],
        ),
        (
            "toolbar_icons",
            vec![
                "rasterize_svg_icon(",
                "custom_widget_mapped(",
                "IconToggleButton::new(",
                "ToolMessage::Toggle(",
            ],
        ),
        (
            "status_bar",
            vec![
                "use radiant::prelude::*;",
                "status_bar(",
                "StatusMessage::ActionPressed",
                "StatusMessage::AutosaveChanged",
                "StatusSegments::primary(",
                "context.spawn(",
                "StatusMessage::WorkerFinished",
            ],
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
            "busy_progress",
            vec![
                "context.spawn(",
                ".animation(",
                ".on_frame(",
                "retained_canvas(",
                "horizontal_progress_fill_rect(",
            ],
        ),
        (
            "multi_window_manifest",
            vec![
                "radiant::window(",
                ".spec(",
                "WindowSpec::new(",
                "build_window_views()",
            ],
        ),
        (
            "typography",
            vec![
                ".wrap()",
                ".truncate()",
                ".baseline(",
                ".align_text(",
                "TextAlign::Center",
            ],
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
        (
            "widget_gallery",
            vec!["badge(", "selectable(", "card()", "stack(["],
        ),
        ("list", vec!["list(", "list_row(", ".fill_height()"]),
        (
            "virtualized_list",
            vec![
                "radiant::app(",
                "virtual_list_window(",
                ".on_scroll(",
                "selectable(",
                ".update(",
            ],
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
            "layout_diagnostics",
            vec![
                "LayoutDebugOptions::all_enabled()",
                "LayoutDiagnosticCode::InvalidScrollOffsetClamped",
                "DebugPrimitiveKind::NodeBounds",
                "layout_tree_with_state(",
            ],
        ),
        (
            "rendering_benchmark",
            vec![
                "surface.frame(",
                "paint_plan.stats()",
                "radiant_rendering_benchmark",
            ],
        ),
        (
            "host_surface_frame",
            vec![
                "SurfaceRuntime::new(",
                "dispatch_event(Event::PointerMove",
                "runtime.borrowed_frame(&theme)",
                "paint_plan.stats()",
                "radiant_host_surface_frame",
            ],
        ),
        (
            "split_workspace",
            vec![
                "SplitPaneSidebarState",
                "SplitPaneSlot",
                "assign_selected_to",
                "pane_view(",
            ],
        ),
        (
            "node_editor",
            vec![
                "retained_canvas(",
                "drop_marker(",
                "drag_handle()",
                "selectable(",
            ],
        ),
        (
            "timeline_editor",
            vec![
                "TimelineSurfaceState::new(",
                "TimelineMotionState::new(",
                "retained_canvas(",
                "custom_widget_mapped(",
            ],
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
        (
            "theme_playground",
            vec![
                "ThemeTokens::dark_for_viewport_width(",
                "effective_ui_scale(",
                "resolve_widget_visual_tokens(",
                "radiant_theme_playground",
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
                ".shortcuts(",
                "ShortcutResolution::action",
                ".update_with(",
                "context.focus(",
                "text_input(",
                ".message(",
            ],
        ),
        (
            "plugin_panel",
            vec![
                "grid_with_gaps(",
                "toggle(",
                "parameter_tile(",
                "WidgetProminence::Subtle",
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
        let source = example_source(&manifest_dir, name, &path);

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
        if name != "custom_widget" && name != "timeline_editor" && name != "toolbar_icons" {
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

fn example_source(manifest_dir: &Path, name: &str, path: &str) -> String {
    let root_path = manifest_dir.join(path);
    let mut source = fs::read_to_string(&root_path)
        .unwrap_or_else(|_| panic!("{name} example should be readable"));
    let module_dir = manifest_dir.join("examples").join(name);
    if module_dir.exists() {
        let mut modules = fs::read_dir(&module_dir)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", module_dir.display()))
            .map(|entry| {
                entry
                    .unwrap_or_else(|err| {
                        panic!("failed to read entry in {}: {err}", module_dir.display())
                    })
                    .path()
            })
            .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("rs"))
            .collect::<Vec<_>>();
        modules.sort();
        for module in modules {
            source.push('\n');
            source.push_str(&fs::read_to_string(&module).unwrap_or_else(|err| {
                panic!("failed to read example module {}: {err}", module.display())
            }));
        }
    }
    source
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
