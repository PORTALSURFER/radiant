use super::*;

const SEPARATELY_COVERED_EXAMPLES: &[&str] = &["generic_native", "hello_world"];

const FOCUSED_EXAMPLE_CONTRACTS: &[(&str, &[&str])] = &[
    ("counter", &["radiant::app(", ".on_click(", ".primary()"]),
    (
        "todo_list",
        &[
            "use radiant::prelude as ui;",
            "text_input(",
            ".bind_submit(",
            "drag_handle()",
            "drop_marker(",
            "overlay_panel(",
            "scroll(",
        ],
    ),
    ("form", &["text_input(", ".bind(", "toggle(", ".on_change("]),
    (
        "volume_slider",
        &[
            "slider(",
            ".primary()",
            ".on_change(",
            "checkbox(",
            "TextAlign::Right",
        ],
    ),
    (
        "sample_source_list",
        &[
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
        &[
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
        &[
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
        &[
            ".animation(",
            ".on_frame(",
            "AnimationMessage::Frame",
            "examples/animation_showcase/pulse_meter.rs",
            "examples/animation_showcase/pulse_meter/tests.rs",
        ],
    ),
    (
        "background_loading",
        &[
            ".update_with(",
            "context.spawn_resource(",
            "ResourceCompletion",
            "LoadingMessage::Loaded",
        ],
    ),
    (
        "folder_browser",
        &[
            "use radiant::prelude as ui;",
            "RADIANT_FOLDER_BROWSER_ROOT",
            "BrowserState::from_root(",
            "view::project_surface",
            "examples/folder_browser/actions.rs",
            "examples/folder_browser/file_view.rs",
            "examples/folder_browser/storage.rs",
            "examples/folder_browser/tests.rs",
        ],
    ),
    (
        "waveform_view",
        &[
            "GpuSurfaceContent::SignalSummaryBands",
            ".animated_transient_overlay_at(",
            "paint_playhead_overlay(",
            "first_widget_rect(",
            "waveform_playback_uses_paint_only_transient_playhead",
        ],
    ),
    (
        "multi_window_manifest",
        &[
            "radiant::window(",
            ".spec(",
            "WindowSpec::new(",
            "build_window_views()",
        ],
    ),
    (
        "popup_window",
        &[
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
        &[
            ".wrap()",
            ".truncate()",
            ".baseline(",
            ".align_text(",
            "TextAlign::Center",
        ],
    ),
    (
        "layout_rows_columns",
        &["row([", "column([", ".padding(", ".fill_width()"],
    ),
    (
        "grid_gallery",
        &[
            "grid_with_gaps(",
            ".wrap()",
            ".fill()",
            "WidgetTone::Accent",
        ],
    ),
    (
        "widget_gallery",
        &["badge(", "selectable(", "card()", "stack(["],
    ),
    ("list", &["list(", "list_row(", ".fill_height()"]),
    (
        "tree_and_details",
        &[
            "tree_list_with_drag(",
            "selectable_sortable_details_list(",
            "DetailsColumn::fixed(",
            "DetailsSort::new(",
            "DragHandleMessage",
        ],
    ),
    (
        "virtualized_list",
        &[
            "radiant::app(",
            "virtual_list_window(",
            ".on_scroll(",
            "selectable(",
            ".update(",
        ],
    ),
    (
        "inspector_panel",
        &[
            "selectable_property_panel(",
            "PropertyRow::new(",
            ".on_change(",
        ],
    ),
    (
        "context_menu",
        &["context_menu_overlay(", "MenuItem::new(", ".danger()"],
    ),
    (
        "paint_helpers",
        &[
            "use radiant::gui::{",
            "border_fill_rects(",
            "text_field_paint(",
            "TextFieldPaint",
            "BorderSides::ALL",
        ],
    ),
    (
        "layout_diagnostics",
        &[
            "LayoutDebugOptions::all_enabled()",
            "LayoutDiagnosticCode::InvalidScrollOffsetClamped",
            "DebugPrimitiveKind::NodeBounds",
            "layout_tree_with_state(",
        ],
    ),
    (
        "rendering_benchmark",
        &[
            "surface.frame(",
            "paint_plan.stats()",
            "radiant_rendering_benchmark",
        ],
    ),
    (
        "host_surface_frame",
        &[
            "SurfaceRuntime::new(",
            "dispatch_event(Event::PointerMove",
            "runtime.borrowed_frame_into(&theme",
            "paint_plan.stats()",
            "radiant_host_surface_frame",
        ],
    ),
    (
        "passive_widgets",
        &[
            "radiant::window(",
            "passive_button(",
            "passive_toggle(",
            "passive_text_input(",
            "canvas()",
            "spacer()",
        ],
    ),
    (
        "split_workspace",
        &[
            "SplitPaneSidebarState",
            "SplitPaneSlot",
            "assign_selected_to",
            "pane_view(",
        ],
    ),
    (
        "node_editor",
        &[
            "retained_canvas(",
            "drop_marker(",
            "drag_handle()",
            "selectable(",
        ],
    ),
    (
        "timeline_editor",
        &[
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
        &[
            ".primary()",
            ".danger()",
            ".subtle()",
            ".hoverable()",
            "toggle(",
        ],
    ),
    (
        "theme_playground",
        &[
            "ThemeTokens::dark_for_viewport_width(",
            "effective_ui_scale(",
            "resolve_widget_visual_tokens(",
            "radiant_theme_playground",
        ],
    ),
    ("scroll", &["scroll_column(", ".fill_height()"]),
    (
        "sizing",
        &[".size(", ".min_size(", ".preferred_size(", ".fill_width()"],
    ),
    (
        "message_routing",
        &[
            ".update_command",
            "Command::message",
            "Command::request_repaint",
        ],
    ),
    ("keys", &[".key(", "list_row(", ".reverse()"]),
    (
        "focus_controls",
        &[
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
        &[
            "grid_with_gaps(",
            "toggle(",
            "parameter_tile(",
            "WidgetProminence::Subtle",
        ],
    ),
    (
        "custom_widget",
        &[
            "use radiant::prelude::*;",
            "fn append_paint(",
            "impl Widget for StatusChip",
            "WidgetOutput::custom(",
            "dispatch_input(",
        ],
    ),
    (
        "gpu_surface",
        &[
            "gpu_surface_input(",
            "GpuSurfaceContent::RgbaAtlas",
            "OnceLock<Arc<ImageRgba>>",
            "gpu_surface_example_lowers_to_retained_gpu_primitive",
            "gpu_surface_example_routes_input_to_state",
        ],
    ),
    (
        "gpu_surface_stack_overlay",
        &[
            "gpu_surface(",
            "custom_widget_mapped(",
            ".animated_transient_overlay_at(",
            "SurfacePaintPlan",
            "SelectionOverlay",
            "examples/gpu_surface_stack_overlay/tests.rs",
        ],
    ),
];

#[test]
fn registered_examples_have_complete_guardrail_coverage() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .expect("Radiant Cargo.toml should be readable");
    let mut uncovered = registered_example_names(&manifest)
        .into_iter()
        .filter(|name| !SEPARATELY_COVERED_EXAMPLES.contains(&name.as_str()))
        .filter(|name| {
            !FOCUSED_EXAMPLE_CONTRACTS
                .iter()
                .any(|(contract_name, _)| *contract_name == name)
        })
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

    for (name, required) in FOCUSED_EXAMPLE_CONTRACTS {
        let path = format!("examples/{name}.rs");
        let source = example_source(&manifest_dir, name, &path);

        assert!(
            manifest.contains(&format!("name = \"{name}\""))
                && manifest.contains(&format!("path = \"{path}\"")),
            "{name} should be an explicit checked Cargo example target"
        );
        assert!(
            *name == "paint_helpers"
                || source.contains("use radiant::prelude::*;")
                || source.contains("use radiant::prelude as ui;"),
            "{name} should use the application prelude"
        );
        for required in *required {
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
            *name,
            "custom_widget"
                | "gpu_surface_stack_overlay"
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

fn registered_example_names(manifest: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_example = false;

    for line in manifest.lines().map(str::trim) {
        match line {
            "[[example]]" => in_example = true,
            line if line.starts_with("[[") || line.starts_with('[') => in_example = false,
            line if in_example && line.starts_with("name = ") => {
                let Some(name) = quoted_toml_value(line) else {
                    continue;
                };
                names.push(name.to_owned());
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
