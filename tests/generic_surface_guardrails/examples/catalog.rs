use super::*;

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
                "use radiant::prelude::*;",
                "SvgIcon::from_svg(",
                "PaintPrimitive::Svg(",
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
                ".animation(",
                ".on_frame(",
                "retained_canvas(",
                "horizontal_progress_fill_rect(",
                "StatusLineLog",
            ],
        ),
        (
            "animation_showcase",
            vec![
                ".animation(",
                ".on_frame(",
                "AnimationMessage::Frame",
                "examples/animation_showcase/pulse_meter.rs",
                "examples/animation_showcase/pulse_meter/tests.rs",
            ],
        ),
        (
            "background_loading",
            vec![".update_with(", "context.spawn(", "LoadingMessage::Loaded"],
        ),
        (
            "waveform_view",
            vec![
                "GpuSurfaceContent::SignalSummaryBands",
                ".animated_transient_overlay_at(",
                "paint_playhead_overlay(",
                "first_widget_rect(",
                "waveform_playback_uses_paint_only_transient_playhead",
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
            "popup_window",
            vec![
                ".floating_popup()",
                ".popup_policy(",
                "NativePopupOptions::default()",
                "WindowSpec::popup(",
                "workflow-popup",
                "OpenPopup",
                "PopupMessage::Close",
                "popup_view(",
                "examples/popup_window/host.rs",
                "examples/popup_window/model.rs",
                "examples/popup_window/platform.rs",
                "examples/popup_window/policy.rs",
                "hide_after_first_present",
                "park_visible_offscreen_show_path",
                "focus_popup_after_reveal",
                "focus_popup_window",
                "wait_for_visible_popup_window",
                "PopupHosts",
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
                "runtime.borrowed_frame_into(&theme",
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
                "TimelineSurfaceParts",
                "TimelineSurfaceState::from_parts(",
                "TimelineEditPreviewParts",
                "TimelineEditPreview::from_parts(",
                "TimelineMotionState::new(",
                "SignalToolFlags",
                "retained_canvas(",
                "custom_widget_mapped(",
                "prefers_pointer_move_paint_only",
                "append_runtime_overlay_paint",
                "drag_handle_at_point",
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
                "use radiant::prelude::*;",
                "fn append_paint(",
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
            "custom_widget" | "timeline_editor" | "toolbar_icons" | "waveform_view"
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

fn example_source(manifest_dir: &Path, name: &str, path: &str) -> String {
    let root_path = manifest_dir.join(path);
    let mut source = fs::read_to_string(&root_path)
        .unwrap_or_else(|_| panic!("{name} example should be readable"));
    let module_dir = manifest_dir.join("examples").join(name);
    if module_dir.exists() {
        let mut modules = Vec::new();
        collect_example_modules(&module_dir, &mut modules);
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

fn collect_example_modules(dir: &Path, modules: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(dir).unwrap_or_else(|err| panic!("failed to read {}: {err}", dir.display()))
    {
        let path = entry
            .unwrap_or_else(|err| panic!("failed to read entry in {}: {err}", dir.display()))
            .path();
        if path.is_dir() {
            collect_example_modules(&path, modules);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            modules.push(path);
        }
    }
}
