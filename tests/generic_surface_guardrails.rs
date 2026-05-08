//! Structural guardrails for Radiant's generic public surface.
//!
//! The boundary is proven through package layout, dependency direction, public
//! exports, standalone examples, and behavior tests. These checks avoid token
//! policing so hosts can choose their own domain language outside Radiant.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const DOMAIN_EXTRACTION_INVENTORY: &str = include_str!("../domain_extraction_inventory.tsv");

const GENERIC_SOURCE_ROOTS: &[&str] = &[
    "src/runtime",
    "src/widgets",
    "src/theme.rs",
    "src/gui/automation.rs",
    "src/gui/badge.rs",
    "src/gui/chrome.rs",
    "src/gui/feedback.rs",
    "src/gui/focus.rs",
    "src/gui/form.rs",
    "src/gui/fingerprint.rs",
    "src/gui/frame.rs",
    "src/gui/input.rs",
    "src/gui/invalidation.rs",
    "src/gui/layout_core",
    "src/gui/list.rs",
    "src/gui/paint.rs",
    "src/gui/panel.rs",
    "src/gui/range.rs",
    "src/gui/repaint.rs",
    "src/gui/retained.rs",
    "src/gui/selection.rs",
    "src/gui/shortcuts.rs",
    "src/gui/snapshot.rs",
    "src/gui/svg.rs",
    "src/gui/text_layout.rs",
    "src/gui/types.rs",
    "src/gui/visualization.rs",
];

const EXEMPT_TOP_LEVEL_GUI_FILES: &[&str] = &["src/gui/mod.rs"];

const REQUIRED_BEHAVIOR_TESTS: &[&str] = &[
    "app_runtime_api.rs",
    "generic_surface_guardrails.rs",
    "layout_public_api.rs",
    "runtime_surface_public_api.rs",
    "widgets_primitive_behaviors.rs",
    "widgets_public_api.rs",
];

#[test]
fn top_level_gui_primitives_are_classified_for_boundary_coverage() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let gui_dir = manifest_dir.join("src/gui");
    let mut unclassified = Vec::new();

    let entries = fs::read_dir(&gui_dir)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", gui_dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| panic!("failed to read entry in {}: {err}", gui_dir.display()))
            .path();
        if !path.is_file()
            || path.extension().and_then(|extension| extension.to_str()) != Some("rs")
        {
            continue;
        }

        let relative = relative_path(&manifest_dir, &path);
        if !GENERIC_SOURCE_ROOTS.contains(&relative.as_str())
            && !EXEMPT_TOP_LEVEL_GUI_FILES.contains(&relative.as_str())
        {
            unclassified.push(relative);
        }
    }

    unclassified.sort();
    assert!(
        unclassified.is_empty(),
        "top-level src/gui/*.rs files must be classified for boundary coverage:\n{}",
        unclassified.join("\n")
    );
}

#[test]
fn radiant_manifest_is_independent_of_parent_workspace_crates() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest_path = manifest_dir.join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", manifest_path.display()));
    let uncommented = strip_toml_comments(&manifest);
    let mut violations = Vec::new();

    for (line_index, line) in uncommented.lines().enumerate() {
        let compact = line
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>();
        if compact.contains("path=\"..") || compact.contains("workspace=true") {
            violations.push(format!(
                "Cargo.toml:{} must not depend on parent workspace crates",
                line_index + 1
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Radiant must remain independently buildable:\n{}",
        violations.join("\n")
    );
}

#[test]
fn default_features_stay_empty_for_standalone_builds() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo = fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .expect("Radiant Cargo.toml should be readable");
    let features = cargo
        .split("[features]")
        .nth(1)
        .and_then(|tail| tail.split("\n[").next())
        .expect("Cargo.toml should define a [features] table");

    assert!(
        features.lines().any(|line| line.trim() == "default = []"),
        "Radiant default features must stay empty"
    );
}

#[test]
fn performance_harness_is_registered_and_documented() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .expect("Radiant Cargo.toml should be readable");
    let bench = fs::read_to_string(manifest_dir.join("benches/perf_harness.rs"))
        .expect("perf_harness bench should be readable");
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("docs/API.md should be readable");

    for required in [
        "[[bench]]",
        "name = \"perf_harness\"",
        "path = \"benches/perf_harness.rs\"",
        "harness = false",
    ] {
        assert!(
            manifest.contains(required),
            "Cargo.toml should register perf harness with `{required}`"
        );
    }
    for scenario in [
        "layout_deep_nesting",
        "layout_wrap_1k",
        "layout_virtualized_10k",
        "runtime_surface_large_tree",
        "gpu_signal_summary",
        "gpu_surface_projection",
        "radiant_perf scenario=",
    ] {
        assert!(
            bench.contains(scenario),
            "perf_harness should include `{scenario}`"
        );
    }
    assert!(
        docs.contains("cargo bench --bench perf_harness")
            && docs.contains("does not enforce machine-dependent pass/fail timing thresholds"),
        "docs/API.md should describe how to run and interpret the perf harness"
    );
}

#[test]
fn public_module_tree_exposes_one_progressive_api_surface() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("Radiant lib.rs should be readable");
    let public_modules = public_module_names(&lib);
    let expected = BTreeSet::from([
        "gui".to_owned(),
        "gui_runtime".to_owned(),
        "layout".to_owned(),
        "prelude".to_owned(),
        "runtime".to_owned(),
        "theme".to_owned(),
        "widgets".to_owned(),
    ]);

    assert_eq!(
        public_modules, expected,
        "Radiant's crate root should expose only generic public modules"
    );
    assert!(
        !manifest_dir.join("src/compat.rs").exists()
            && rust_sources_under(&manifest_dir.join("src/compat")).is_empty(),
        "compatibility adapter source files belong outside the generic Radiant crate"
    );
}

#[test]
fn behavior_test_suite_is_explicit_and_local_to_generic_surfaces() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tests_dir = manifest_dir.join("tests");
    let mut test_files = fs::read_dir(&tests_dir)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", tests_dir.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|err| {
                    panic!("failed to read entry in {}: {err}", tests_dir.display())
                })
                .path()
        })
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("rs"))
        .map(|path| {
            path.file_name()
                .and_then(|file_name| file_name.to_str())
                .expect("test file should have utf-8 name")
                .to_owned()
        })
        .collect::<Vec<_>>();
    test_files.sort();

    assert_eq!(
        test_files, REQUIRED_BEHAVIOR_TESTS,
        "Radiant integration tests should stay focused on generic layout, runtime, widget, and boundary behavior"
    );
    assert!(
        !tests_dir.join("shots").exists()
            && !manifest_dir
                .join("src/gui_runtime/native_vello/tests")
                .exists()
            && !manifest_dir
                .join("src/gui_runtime/native_vello/tests.rs")
                .exists(),
        "renderer snapshots and backend fixture trees should live with their owning host or backend suite"
    );
}

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
            && manifest.contains("path = \"examples/generic_native.rs\"")
            && manifest.contains("test = false"),
        "generic_native should be an explicit standalone Cargo example target"
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
            "layout_rows_columns",
            vec!["row([", "column([", ".padding(", ".fill_width()"],
        ),
        ("list", vec!["list(", "list_row(", ".fill_height()"]),
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
            "custom_widget",
            vec![
                "impl Widget for StatusChip",
                "custom_widget(",
                "WidgetOutput::custom(",
            ],
        ),
    ] {
        let path = format!("examples/{name}.rs");
        let source = fs::read_to_string(manifest_dir.join(&path))
            .unwrap_or_else(|_| panic!("{name} example should be readable"));

        assert!(
            manifest.contains(&format!("name = \"{name}\""))
                && manifest.contains(&format!("path = \"{path}\""))
                && manifest.contains("test = false"),
            "{name} should be an explicit standalone Cargo example target"
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
fn runtime_widgets_are_open_trait_objects_not_closed_builtin_enums() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let surface = fs::read_to_string(manifest_dir.join("src/runtime/surface.rs"))
        .expect("runtime surface should be readable");

    for closed_runtime_shape in [
        "enum RuntimeWidget",
        "enum WidgetMessageMapper",
        "WidgetMessageMapper::None",
        "ButtonWidget",
        "ToggleWidget",
        "TextInputWidget",
        "ScrollbarWidget",
        "DragHandleWidget",
        "ListItemWidget",
        "SelectableWidget",
        "BadgeWidget",
        "CardWidget",
        "ImageWidget",
        "CanvasWidget",
        "pub fn button(",
        "pub fn button_mapped(",
        "pub fn text_input(",
        "pub fn text_input_mapped(",
        "pub fn toggle(",
        "pub fn toggle_mapped(",
        "pub fn scrollbar(",
        "pub fn scrollbar_mapped(",
        "pub fn drag_handle_mapped(",
        "pub fn list_item(",
        "pub fn list_item_mapped(",
        "pub fn selectable(",
        "pub fn selectable_mapped(",
        "pub fn canvas(",
        "pub fn canvas_mapped(",
    ] {
        assert!(
            !surface.contains(closed_runtime_shape),
            "runtime widget routing should not reintroduce closed builtin-only dispatch `{closed_runtime_shape}`"
        );
    }

    for open_runtime_shape in [
        "Box<dyn Widget>",
        "WidgetMessageMapper<Message> {",
        "pub fn typed<",
        "pub fn dynamic(",
    ] {
        assert!(
            surface.contains(open_runtime_shape),
            "runtime widget routing should preserve open widget API support `{open_runtime_shape}`"
        );
    }
}

#[test]
fn widget_common_does_not_carry_hardcoded_widget_taxonomies() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let widgets_dir = manifest_dir.join("src/widgets");
    let interaction =
        fs::read_to_string(widgets_dir.join("interaction.rs")).expect("interaction.rs is readable");
    let primitive_support = fs::read_to_string(widgets_dir.join("primitives/support.rs"))
        .expect("primitive widget support is readable");
    let mut violations = Vec::new();

    for forbidden in [
        "pub enum WidgetOutput",
        "WidgetOutput::Button",
        "WidgetOutput::Badge",
        "WidgetOutput::ListItem",
        "WidgetOutput::Selectable",
        "WidgetOutput::Toggle",
        "WidgetOutput::TextInput",
        "WidgetOutput::Scrollbar",
        "WidgetOutput::DragHandle",
        "WidgetOutput::Canvas",
    ] {
        if interaction.contains(forbidden) {
            violations.push(format!("src/widgets/interaction.rs contains `{forbidden}`"));
        }
    }

    for source_path in rust_sources_under(&widgets_dir) {
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
        let relative = relative_path(&manifest_dir, &source_path);
        for forbidden in [
            "enum WidgetKind",
            "WidgetKind::",
            "enum WidgetMessageKind",
            "WidgetMessageKind::",
            "enum WidgetSpec",
            "WidgetSpec::",
            "emitted_messages",
        ] {
            if source.contains(forbidden) {
                violations.push(format!("{relative} contains `{forbidden}`"));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "widget core should not carry hardcoded widget taxonomies:\n{}",
        violations.join("\n")
    );

    assert!(
        interaction.contains("pub struct WidgetOutput")
            && interaction.contains("pub fn typed<T>")
            && interaction.contains("pub fn typed_ref<T"),
        "widget outputs should stay open typed payloads instead of a closed builtin enum"
    );

    for forbidden in [
        "pub struct TextWidget",
        "pub struct ButtonWidget",
        "pub struct ToggleWidget",
        "pub struct TextInputWidget",
        "pub struct ScrollbarWidget",
        "pub struct DragHandleWidget",
        "pub struct ListItemWidget",
        "pub struct SelectableWidget",
        "pub struct BadgeWidget",
        "pub struct CardWidget",
        "pub struct ImageWidget",
        "pub struct CanvasWidget",
        "impl Widget for",
    ] {
        assert!(
            !primitive_support.contains(forbidden),
            "primitive support should not own concrete widget types or trait impls: `{forbidden}`"
        );
    }
}

#[test]
fn runtime_does_not_export_per_widget_paint_catalogs() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let runtime_dir = manifest_dir.join("src/runtime");
    let mut violations = Vec::new();

    for source_path in rust_sources_under(&runtime_dir) {
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
        let relative = relative_path(&manifest_dir, &source_path);
        for forbidden in [
            "push_text_widget_paint",
            "push_button_widget_paint",
            "push_toggle_widget_paint",
            "push_text_input_widget_paint",
            "push_scrollbar_widget_paint",
            "push_drag_handle_widget_paint",
            "push_list_item_widget_paint",
            "push_selectable_widget_paint",
            "push_badge_widget_paint",
            "push_card_widget_paint",
            "push_image_widget_paint",
            "push_canvas_widget_paint",
        ] {
            if source.contains(forbidden) {
                violations.push(format!("{relative} contains `{forbidden}`"));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "runtime paint should not own or export per-widget paint catalogs:\n{}",
        violations.join("\n")
    );
}

#[test]
fn application_view_nodes_do_not_carry_hardcoded_widget_variants() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let application_entry = fs::read_to_string(manifest_dir.join("src/application.rs"))
        .expect("application builder source should be readable");
    let mut application = application_entry;
    for source_path in rust_sources_under(&manifest_dir.join("src/application")) {
        application.push('\n');
        application.push_str(
            &fs::read_to_string(&source_path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display())),
        );
    }

    for forbidden in [
        "ViewNodeKind::Text",
        "ViewNodeKind::ButtonMapped",
        "ViewNodeKind::DragHandle",
        "ViewNodeKind::Toggle",
        "ViewNodeKind::TextInput",
        "ViewNodeKind::CustomWidget",
    ] {
        assert!(
            !application.contains(forbidden),
            "application builder should lower widgets through the WidgetView API, not hardcoded variant `{forbidden}`"
        );
    }

    for required in [
        "pub trait WidgetView<Message>",
        "ViewNodeKind::Widget",
        "fn into_surface_node(",
        "pub struct MappedWidget",
        "pub fn widget<Message>",
    ] {
        assert!(
            application.contains(required),
            "application builder should preserve generic widget lowering `{required}`"
        );
    }
}

#[test]
fn gui_runtime_public_facade_exports_generic_runtime_entrypoints() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/gui_runtime/mod.rs"))
        .expect("gui_runtime module should be readable");
    let public_exports = source
        .split("pub use native_vello::{")
        .nth(1)
        .and_then(|tail| tail.split("};").next())
        .expect("gui_runtime should have a native_vello public export block");

    for required in [
        "NativeGenericRunReport",
        "NativeGenericRuntimeArtifacts",
        "NativeStartupTimingArtifact",
        "run_native_vello_runtime",
        "run_native_vello_runtime_with_artifacts",
    ] {
        assert!(
            public_exports.contains(required),
            "radiant::gui_runtime should expose generic runtime API `{required}`"
        );
    }
    assert!(
        source.contains("pub struct RuntimeRunReport<Artifacts>"),
        "radiant::gui_runtime should expose a generic runtime report envelope"
    );
}

#[test]
fn api_docs_describe_the_structural_boundary_strategy() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("docs/API.md should be readable");
    let normalized_docs = docs.split_whitespace().collect::<Vec<_>>().join(" ");

    for required in [
        "# Radiant Core API",
        "Dependency Boundary",
        "host -> Radiant, never Radiant -> host",
        "Boundary tests prove that dependency direction, public exports, examples, and",
        "they intentionally avoid enforcing product",
        "Radiant now exposes only generic GUI and native runtime APIs",
        "Radiant exposes one public API with progressive control",
        "Application builders and explicit runtime objects are part of the same API surface",
        "same model with more explicit control",
        "Radiant's application API is designed to be easy to read without hiding the runtime model",
        "View, Element, And Widget",
        "VirtualListWindow",
        "virtual_list_view_start_after_scroll_delta",
        "SignalChromeState",
        "SignalToolState",
        "SignalRasterPreview",
        "TimelineViewport",
        "TimelineTransportState",
        "TimelineEditPreview",
        "TimelineFeedbackEvents",
        "TimelinePresentationState",
        "TimelineSurfaceState",
        "TimelineMotionState",
        "UiSurface",
        "SurfaceNode",
        "WidgetId",
        "Command<Message>",
        "Soft-Deprecated First-Use Boilerplate",
        "not a Rust `#[deprecated]` attribute on the explicit control objects",
        "RuntimeRunReport<Artifacts>",
        "RuntimeBridge",
        "ThemeTokens",
        "SurfacePaintPlan",
        "InvalidationMask",
        "RetainedSegmentMask",
        "VisualSnapshot",
    ] {
        assert!(
            normalized_docs.contains(required),
            "docs/API.md should document `{required}`"
        );
    }
}

#[test]
fn api_docs_soft_deprecate_only_first_use_runtime_boilerplate() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("docs/API.md should be readable");
    let runtime = fs::read_to_string(manifest_dir.join("src/runtime/mod.rs"))
        .expect("runtime module should be readable");

    for first_use_boilerplate in [
        "constructing `NativeRunOptions` directly for a hello-world app",
        "hand-writing a closure bridge before the app has meaningful state",
        "wrapping one label in `Arc<UiSurface<_>>`",
        "manually composing `SurfaceNode`, `SurfaceChild`, explicit numeric IDs, and",
    ] {
        assert!(
            docs.contains(first_use_boilerplate),
            "docs/API.md should soft-deprecate `{first_use_boilerplate}` for first-use application code"
        );
    }

    for explicit_control in [
        "The `radiant::runtime` module",
        "`RuntimeBridge`",
        "`UiSurface`",
        "`SurfaceNode`",
        "`SurfaceChild`",
        "`NativeRunOptions`",
        "`WidgetSizing`",
        "remain supported and non-deprecated for hosts",
    ] {
        assert!(
            docs.contains(explicit_control),
            "docs/API.md should preserve explicit-control API guidance for `{explicit_control}`"
        );
    }
    assert!(
        !runtime.contains("#[deprecated"),
        "radiant::runtime should remain supported, not a blanket-deprecated module"
    );
}

#[test]
fn domain_extraction_inventory_is_closed_out() {
    let rules = parse_extraction_inventory();
    assert!(
        rules.is_empty(),
        "domain extraction inventory should have no active migration rules"
    );
    assert!(
        DOMAIN_EXTRACTION_INVENTORY.contains("no longer an active")
            && DOMAIN_EXTRACTION_INVENTORY.contains("migration backlog"),
        "domain extraction inventory should be retained only as a final boundary note"
    );
}

#[allow(dead_code)]
#[derive(Debug)]
struct ExtractionRule {
    pattern: String,
    disposition: String,
    owner: String,
}

fn public_module_names(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub mod "))
        .filter_map(|tail| tail.split([';', '{']).next())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}

fn relative_path(manifest_dir: &Path, path: &Path) -> String {
    path.strip_prefix(manifest_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn rust_sources_under(path: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    if !path.exists() {
        return sources;
    }
    if path.is_dir() {
        for entry in fs::read_dir(path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
        {
            let entry = entry
                .unwrap_or_else(|err| panic!("failed to read entry in {}: {err}", path.display()))
                .path();
            sources.extend(rust_sources_under(&entry));
        }
    } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
        sources.push(path.to_owned());
    }
    sources
}

fn strip_toml_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split_once('#').map_or(line, |(before, _)| before))
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_extraction_inventory() -> Vec<ExtractionRule> {
    let mut rules = Vec::new();
    for (line_index, line) in DOMAIN_EXTRACTION_INVENTORY.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("pattern\t") {
            continue;
        }
        let columns = line.split('\t').collect::<Vec<_>>();
        assert_eq!(
            columns.len(),
            4,
            "domain extraction inventory line {} should have four tab-separated columns",
            line_index + 1
        );
        rules.push(ExtractionRule {
            pattern: columns[0].to_owned(),
            disposition: columns[1].to_owned(),
            owner: columns[2].to_owned(),
        });
    }
    rules
}
