//! Guardrails for the generic Radiant public surface.
//!
//! Generic modules are allowed to use backend-neutral Radiant primitives only:
//! `radiant::layout`, `radiant::widgets`, `radiant::runtime`, `radiant::theme`,
//! and the shared non-shell `gui` primitives those APIs expose. The current
//! The legacy host shell remains a transitional compatibility exception under
//! `compat::legacy_shell`, `gui::native_shell`, and the native Vello
//! compatibility runtime.

use std::{
    collections::BTreeMap,
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

const COMPAT_INTEGRATION_TESTS: &[&str] = &[];

const FORBIDDEN_GENERIC_TOKENS: &[&str] = &[
    "crate::app",
    "crate::{app",
    "crate::compat_app_contract",
    "crate::{compat_app_contract",
    "crate::compat::legacy_shell",
    "crate::{compat::legacy_shell",
    "compat::legacy_shell",
    "crate::gui::native_shell",
    "crate::{gui::native_shell",
    "gui::native_shell",
    "crate::gui_runtime::native_vello",
    "crate::{gui_runtime::native_vello",
    "gui_runtime::native_vello",
    "native_shell",
    "AppModel",
    "UiAction",
];

const FORBIDDEN_GENERIC_TEST_TOKENS: &[&str] = &[
    "radiant::compat::legacy_shell",
    "radiant::{compat::legacy_shell",
    "compat::legacy_shell",
    concat!("Sem", "pal"),
    concat!("sem", "pal"),
    "capture_gui_automation_snapshot",
    "capture_native_shell_shot_snapshot",
];

const DOMAIN_SCAN_ROOTS: &[&str] = &["src", "tests", "examples"];

const DOMAIN_SCAN_EXEMPT_FILES: &[&str] = &[
    "tests/generic_surface_guardrails.rs",
    "tests/generic_extraction_ownership.rs",
];

const HOST_PRODUCT_NAME_SCAN_ROOTS: &[&str] = &["src", "docs", "examples"];

const DOMAIN_TERMS: &[&str] = &[
    "AppModel",
    "UiAction",
    concat!("Sem", "pal"),
    concat!("sem", "pal"),
    "sample",
    "Sample",
    "browser",
    "Browser",
    "audio",
    "Audio",
    "waveform",
    "Waveform",
    "tag",
    "Tag",
    "collection",
    "Collection",
    "library",
    "Library",
    "source",
    "Source",
    "folder",
    "Folder",
    "BPM",
    "bpm",
    "slice",
    "Slice",
    "loop",
    "Loop",
    "one-shot",
    "One-shot",
    "oneshot",
    "Oneshot",
];

const DOC_PRODUCT_DOMAIN_TERMS: &[&str] = &[
    "sample",
    "Sample",
    "browser",
    "Browser",
    "audio",
    "Audio",
    "waveform",
    "Waveform",
    "tag",
    "Tag",
    "collection",
    "Collection",
];

const INVENTORY_DISPOSITIONS: &[&str] = &[
    "move_to_host",
    "generalize_in_radiant",
    "remove_compat_export",
    "split_generic_from_compat",
    "generic_wording_cleanup",
];

const INVENTORY_OWNERS: &[&str] = &["host_app", "radiant_boundary"];

#[test]
fn generic_sources_do_not_import_legacy_shell_contracts() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();

    for root in GENERIC_SOURCE_ROOTS {
        collect_violations(&manifest_dir.join(root), &manifest_dir, &mut violations);
    }

    assert!(
        violations.is_empty(),
        "generic Radiant modules must stay independent from host compatibility contracts; \
         move transitional shell code under compat::legacy_shell, gui::native_shell, or gui_runtime/native_vello:\n{}",
        violations.join("\n")
    );
}

#[test]
fn localized_native_shell_surfaces_do_not_import_parent_host_sources() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let native_shell_mod = fs::read_to_string(manifest_dir.join("src/gui/native_shell/mod.rs"))
        .expect("native shell module");

    for module in [
        "browser_chrome_surface",
        "status_surface",
        "style/chrome",
        "style/palette",
        "sidebar_surface",
        "signal_header_surface",
        "top_bar_surface",
        "waveform_toolbar_surface",
    ] {
        let module_file = manifest_dir.join(format!("src/gui/native_shell/{module}.rs"));
        assert!(
            module_file.exists(),
            "{module} should be localized inside Radiant rather than imported from host parent sources"
        );
        assert!(
            !native_shell_mod.contains(&format!("app_core/native_shell/composition/{module}.rs")),
            "{module} must stay a local Radiant module until the remaining compatibility shell is retired"
        );
    }

    for path in [
        "src/gui/native_shell/browser_chrome_surface_helpers.rs",
        "src/gui/native_shell/sidebar_surface_helpers.rs",
    ] {
        assert!(
            manifest_dir.join(path).exists(),
            "{path} should be localized with its surface module inside Radiant"
        );
    }

    let style_mod = fs::read_to_string(manifest_dir.join("src/gui/native_shell/style/mod.rs"))
        .expect("native shell style module");
    assert!(
        !style_mod.contains("app_core/native_shell/composition/style/"),
        "native shell style modules must stay local to Radiant while the compatibility shell remains"
    );
    assert!(
        !native_shell_mod.contains("app_core/native_shell/composition/tests/"),
        "host native-shell composition fixtures must stay out of Radiant native_shell"
    );

    let layout_adapter =
        fs::read_to_string(manifest_dir.join("src/gui/native_shell/layout_adapter.rs"))
            .expect("native shell layout adapter");
    assert!(
        !layout_adapter.contains("app_core/native_shell/composition/layout_adapter/"),
        "native shell layout-adapter modules must stay local to Radiant while the compatibility shell remains"
    );
    for path in [
        "src/gui/native_shell/layout_adapter/controls_tests.rs",
        "src/gui/native_shell/layout_adapter/sidebar_header/tests.rs",
        "src/gui/native_shell/layout_adapter/waveform_annotations/tests.rs",
    ] {
        assert!(
            !manifest_dir.join(path).exists(),
            "{path} is a host composition fixture and must stay out of Radiant"
        );
    }

    let toolbar_helpers =
        fs::read_to_string(manifest_dir.join("src/gui/native_shell/state/toolbar_helpers.rs"))
            .expect("native shell toolbar helper facade");
    assert!(
        !toolbar_helpers.contains("app_core/native_shell/composition/state/toolbar_helpers/"),
        "native shell toolbar helpers must stay local to Radiant while the compatibility shell remains"
    );

    let hit_testing =
        fs::read_to_string(manifest_dir.join("src/gui/native_shell/state/hit_testing/mod.rs"))
            .expect("native shell hit-testing facade");
    assert!(
        !hit_testing.contains("app_core/native_shell/composition/state/hit_testing/"),
        "native shell hit-testing helpers must stay local to Radiant while the compatibility shell remains"
    );

    let frame_build =
        fs::read_to_string(manifest_dir.join("src/gui/native_shell/state/frame_build.rs"))
            .expect("native shell frame-build facade");
    assert!(
        !frame_build.contains("app_core/native_shell/composition/state/frame_build/"),
        "native shell frame-build helpers must stay local to Radiant while the compatibility shell remains"
    );

    let state_facade = fs::read_to_string(manifest_dir.join("src/gui/native_shell/state.rs"))
        .expect("native shell state facade");
    for module in [
        "automation",
        "browser_rows",
        "motion_overlay",
        "options_panel",
        "overlays",
        "svg_icons",
        "text_fields",
        "waveform_segments",
    ] {
        assert!(
            !state_facade.contains(&format!("app_core/native_shell/composition/state/{module}")),
            "{module} must stay local to Radiant while the compatibility shell remains"
        );
    }
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/tests")
            .exists(),
        "host native-shell state fixtures must stay out of Radiant native_shell"
    );
}

#[test]
fn localized_legacy_shell_text_entry_does_not_import_parent_host_sources() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let text_entry_facade = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/legacy_shell_text_entry.rs"),
    )
    .expect("legacy shell text-entry facade");
    assert!(
        !text_entry_facade.contains("app_core/native_shell/composition/runtime/text_entry"),
        "legacy shell text-entry helpers must stay local to Radiant until the remaining compatibility runtime is retired"
    );
    for path in [
        "src/gui_runtime/native_vello/legacy_shell_text_entry/text_entry/mod.rs",
        "src/gui_runtime/native_vello/legacy_shell_text_entry/text_entry/pointer.rs",
        "src/gui_runtime/native_vello/legacy_shell_text_entry/text_entry/state.rs",
    ] {
        assert!(
            manifest_dir.join(path).exists(),
            "{path} should be localized inside Radiant rather than imported from host parent sources"
        );
    }
}

#[test]
fn top_level_gui_primitives_are_classified_for_generic_import_guard() {
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

        let relative = path
            .strip_prefix(&manifest_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if !GENERIC_SOURCE_ROOTS.contains(&relative.as_str())
            && !EXEMPT_TOP_LEVEL_GUI_FILES.contains(&relative.as_str())
        {
            unclassified.push(relative);
        }
    }

    unclassified.sort();
    assert!(
        unclassified.is_empty(),
        "top-level src/gui/*.rs files must be classified so generic primitives are covered by \
         the host import guard, or explicitly exempted as transitional compat/docs files:\n{}",
        unclassified.join("\n")
    );
}

#[test]
fn radiant_manifest_does_not_depend_on_host_product_or_parent_workspace() {
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
            .collect::<String>()
            .to_ascii_lowercase();
        if compact.contains(host_product_slug()) {
            violations.push(format!(
                "Cargo.toml:{} must not name a host-product dependency",
                line_index + 1
            ));
        }
        if compact.contains("path=\"..") || compact.contains("workspace=true") {
            violations.push(format!(
                "Cargo.toml:{} must not depend on parent workspace crates",
                line_index + 1
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Radiant must remain independently buildable with dependency direction host -> Radiant:\n{}",
        violations.join("\n")
    );
}

#[test]
fn generic_integration_tests_do_not_reintroduce_legacy_shell_fixtures() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tests_dir = manifest_dir.join("tests");
    let mut violations = Vec::new();

    assert!(
        !tests_dir.join("shots").exists(),
        "host visual snapshot fixtures belong in the host app test tree, not Radiant tests/shots"
    );

    let entries = fs::read_dir(&tests_dir)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", tests_dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| panic!("failed to read entry in {}: {err}", tests_dir.display()))
            .path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        if path
            .file_name()
            .and_then(|file_name| file_name.to_str())
            .is_some_and(|file_name| {
                file_name == "generic_surface_guardrails.rs"
                    || COMPAT_INTEGRATION_TESTS.contains(&file_name)
            })
        {
            continue;
        }
        collect_token_violations(
            &path,
            &manifest_dir,
            FORBIDDEN_GENERIC_TEST_TOKENS,
            &mut violations,
        );
    }

    assert!(
        violations.is_empty(),
        "generic Radiant integration tests must stay neutral; keep host shell coverage in \
         host-owned tests or the explicit compat tests:\n{}",
        violations.join("\n")
    );
}

#[test]
fn generic_native_example_stays_product_neutral_and_runtime_backed() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest_path = manifest_dir.join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", manifest_path.display()));
    let example_path = manifest_dir.join("examples/generic_native.rs");
    let source = fs::read_to_string(&example_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", example_path.display()));
    let uncommented = strip_rust_comments(&source);

    assert!(
        manifest.contains("[[example]]")
            && manifest.contains("name = \"generic_native\"")
            && manifest.contains("path = \"examples/generic_native.rs\"")
            && manifest.contains("test = false"),
        "generic_native should be an explicit standalone Cargo example target that can be checked without running the GUI"
    );

    for forbidden in FORBIDDEN_GENERIC_TEST_TOKENS {
        assert!(
            !uncommented.contains(forbidden),
            "generic_native example must not depend on host compatibility fixtures, found `{forbidden}`"
        );
    }
    for required in [
        "declarative_command_runtime_bridge",
        "Command::message",
        "Command::request_repaint",
        "run_native_vello_runtime",
        "UiSurface",
        "SurfaceNode::row",
        "SurfaceNode::text",
        "SurfaceNode::button",
        "SurfaceChild::fill",
    ] {
        assert!(
            uncommented.contains(required),
            "generic_native example should exercise the generic runtime/widget API via `{required}`"
        );
    }
}

#[test]
fn native_runtime_flags_use_radiant_names() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let font_source =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/text_renderer/font.rs"))
            .expect("native font source should be readable");

    assert!(
        font_source.contains("RADIANT_NATIVE_FONT_PATH"),
        "native font override should use a Radiant-owned runtime flag"
    );
    assert!(
        !font_source.contains(concat!("SEM", "PAL_NATIVE_FONT_PATH")),
        "Radiant runtime code must not expose host-product-named runtime flags"
    );
}

#[test]
fn gui_runtime_public_facade_exports_generic_runtime_only() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module_path = manifest_dir.join("src/gui_runtime/mod.rs");
    let source = fs::read_to_string(&module_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", module_path.display()));
    let public_exports = source
        .split("pub use native_vello::{")
        .nth(1)
        .and_then(|tail| tail.split("};").next())
        .expect("gui_runtime should have a native_vello public export block");

    for forbidden in [
        "NativeRunReport",
        "NativeRuntimeArtifacts",
        "capture_gui_automation_snapshot",
        "capture_native_shell_shot_snapshot",
        "run_native_vello_app",
        "run_native_vello_app_declarative",
        "run_native_vello_app_declarative_with_artifacts",
        "run_native_vello_app_with_artifacts",
        "run_native_vello_preview",
    ] {
        assert!(
            !public_exports.contains(forbidden),
            "radiant::gui_runtime must stay generic-only; expose `{forbidden}` through compat::legacy_native_vello until the compatibility runtime is removed"
        );
    }
    for required in [
        "NativeGenericRunReport",
        "NativeGenericRuntimeArtifacts",
        "NativeStartupTimingArtifact",
        "run_native_vello_runtime",
        "run_native_vello_runtime_with_artifacts",
    ] {
        assert!(
            public_exports.contains(required),
            "radiant::gui_runtime should continue exposing generic runtime API `{required}`"
        );
    }
    assert!(
        source.contains("pub struct RuntimeRunReport<Artifacts>"),
        "radiant::gui_runtime should expose a generic runtime report envelope"
    );
    let gui_mod = fs::read_to_string(manifest_dir.join("src/gui/mod.rs"))
        .expect("gui module should be readable");
    assert!(
        gui_mod.contains("pub mod snapshot;"),
        "radiant::gui should expose generic visual snapshot primitives"
    );
}

#[test]
fn legacy_shell_compatibility_is_not_enabled_by_default() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo_path = manifest_dir.join("Cargo.toml");
    let cargo = fs::read_to_string(&cargo_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", cargo_path.display()));
    assert!(
        cargo.contains("default = []"),
        "Radiant default features must stay empty so standalone builds do not compile compatibility shell code"
    );
    assert!(
        cargo.contains("legacy-shell = []"),
        "Radiant must expose an explicit legacy-shell feature for current host compatibility"
    );
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("Radiant lib.rs should be readable");
    assert!(
        lib.contains("#[cfg(feature = \"legacy-shell\")]\npub mod compat;"),
        "Radiant default builds should not expose the transitional compat namespace"
    );
}

#[test]
fn legacy_shell_namespace_does_not_reexport_generic_runtime_types() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let compat = fs::read_to_string(manifest_dir.join("src/compat.rs"))
        .expect("compat facade should be readable");

    assert!(
        !compat.contains("pub use crate::gui_runtime"),
        "compat::legacy_shell must not re-export generic runtime types; host adapters should import them from radiant::gui_runtime"
    );
    for forbidden in [
        "NativeRunOptions",
        "NativeStartupTimingArtifact",
        "WindowIconRgba",
    ] {
        assert!(
            !compat.contains(forbidden),
            "generic runtime type `{forbidden}` belongs under radiant::gui_runtime, not compat::legacy_shell"
        );
    }
}

#[test]
fn legacy_shell_sources_are_feature_gated() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(
        !manifest_dir.join("src/app").exists(),
        "legacy shell compatibility contracts must live under src/compat/legacy_shell, not a top-level src/app module"
    );
    assert!(
        !manifest_dir
            .join("src/gui_runtime/native_vello/shell_snapshot.rs")
            .exists(),
        "host shell snapshot capture must not live in the generic native Vello runtime tree"
    );
    assert!(
        !manifest_dir
            .join("src/compat/legacy_shell/shell_snapshot.rs")
            .exists(),
        "host shell snapshot capture belongs in the consuming application, not the Radiant compatibility facade"
    );
    assert!(
        !manifest_dir
            .join("src/gui_runtime/native_vello/text_bpm.rs")
            .exists()
            && !manifest_dir
                .join("src/gui_runtime/native_vello/text_bpm")
                .exists(),
        "legacy shell BPM/text-entry helpers belong with host composition, not the generic native Vello runtime tree"
    );
    assert!(
        !manifest_dir
            .join("src/compat/legacy_shell/native_vello_text_bpm")
            .exists(),
        "legacy shell BPM/text-entry helpers belong with host composition, not Radiant compatibility contracts"
    );
    assert!(
        !manifest_dir
            .join("src/compat/legacy_shell/automation.rs")
            .exists(),
        "generic automation DTOs belong in gui::automation and should be re-exported directly when compatibility needs them"
    );
    assert!(
        !manifest_dir
            .join("src/compat/legacy_shell/runtime_artifacts.rs")
            .exists(),
        "legacy runtime artifact wrappers should not live in a separate compatibility module"
    );
    assert!(
        !manifest_dir
            .join("src/compat/legacy_shell/browser.rs")
            .exists(),
        "browser/list/map compatibility aliases should be re-exported directly from legacy_shell or moved to generic primitives"
    );
    assert!(
        !manifest_dir
            .join("src/compat/legacy_shell/native_vello.rs")
            .exists(),
        "legacy native Vello runtime entrypoints should live under compat::legacy_native_vello, not the model/action shell facade"
    );
    for path in [
        "src/compat/legacy_shell/actions/mod.rs",
        "src/compat/legacy_shell/aliases.rs",
        "src/compat/legacy_shell/bridge.rs",
        "src/compat/legacy_shell/dirty_segments.rs",
        "src/compat/legacy_shell/motion.rs",
        "src/compat/legacy_native_vello.rs",
        "src/compat/legacy_shell/shell.rs",
        "src/compat/legacy_shell/sources.rs",
        "src/compat/legacy_shell/waveform.rs",
    ] {
        let source = fs::read_to_string(manifest_dir.join(path))
            .unwrap_or_else(|error| panic!("{path} should be readable: {error}"));
        assert!(
            !source.contains("app_core/native_shell/composition/runtime"),
            "{path} must remain localized in Radiant instead of path-importing host runtime sources"
        );
    }
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/browser_chrome_surface_tests.rs")
            .exists(),
        "host browser chrome fixtures belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/sidebar_surface_tests.rs")
            .exists(),
        "host sidebar fixtures belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/layout_adapter/controls_tests.rs")
            .exists(),
        "host shell control layout fixtures belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/tests/folder_visibility_toggle.rs")
            .exists(),
        "host native-shell state fixtures belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/tests")
            .exists(),
        "host native-shell state test harness belongs with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir.join("src/gui/native_shell/tests").exists(),
        "host native-shell composition tests belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir.join("assets/icons").exists(),
        "host native-shell icon assets belong with host composition assets, not Radiant"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/ownership_inventory.tsv")
            .exists(),
        "host native-shell ownership inventory belongs with host composition tests, not Radiant"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/svg_icons/parser.rs")
            .exists()
            && !manifest_dir
                .join("src/gui/native_shell/state/svg_icons")
                .exists(),
        "generic SVG parsing belongs under Radiant gui primitives, not the host native_shell compatibility tree"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/tests/status_bar_progress.rs")
            .exists(),
        "host status/progress state fixtures belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/tests/browser_scrollbars.rs")
            .exists()
            && !manifest_dir
                .join("src/gui/native_shell/state/tests/browser_scrollbars")
                .exists(),
        "host browser-scrollbar state fixtures belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/tests/sidebar.rs")
            .exists()
            && !manifest_dir
                .join("src/gui/native_shell/state/tests/sidebar")
                .exists(),
        "host sidebar state fixtures belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/tests/browser_toolbar.rs")
            .exists()
            && !manifest_dir
                .join("src/gui/native_shell/state/tests/browser_toolbar")
                .exists(),
        "host browser-toolbar state fixtures belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/tests/overlay_controls.rs")
            .exists()
            && !manifest_dir
                .join("src/gui/native_shell/state/tests/overlay_controls")
                .exists(),
        "host overlay-control state fixtures belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/tests/frame_build.rs")
            .exists()
            && !manifest_dir
                .join("src/gui/native_shell/state/tests/frame_build")
                .exists(),
        "host frame-build state fixtures belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/tests/overlays.rs")
            .exists()
            && !manifest_dir
                .join("src/gui/native_shell/state/tests/overlays")
                .exists(),
        "host overlay state fixtures belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/tests/waveform_selection.rs")
            .exists()
            && !manifest_dir
                .join("src/gui/native_shell/state/tests/waveform_selection")
                .exists(),
        "host waveform-selection state fixtures belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/tests/playhead_trail_render.rs")
            .exists()
            && !manifest_dir
                .join("src/gui/native_shell/state/tests/playhead_trail_state.rs")
                .exists(),
        "host playhead-trail state fixtures belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/tests/waveform_slices.rs")
            .exists(),
        "host waveform-slice state fixtures belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/tests/selection_states.rs")
            .exists(),
        "host selection-state fixtures belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/tests/browser_rows")
            .exists(),
        "host browser-row fixtures belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/tests/chrome_layout")
            .exists(),
        "host chrome-layout fixtures belong with host composition tests, not Radiant native_shell"
    );
    assert!(
        !manifest_dir
            .join("src/gui/native_shell/state/tests/waveform_edit_fades.rs")
            .exists()
            && !manifest_dir
                .join("src/gui/native_shell/state/tests/waveform_edit_handles.rs")
                .exists(),
        "host waveform-edit fixtures belong with host composition tests, not Radiant native_shell"
    );
    let native_vello = fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello.rs"))
        .expect("native_vello.rs should be readable");
    for forbidden in [
        "pub struct NativeRuntimeArtifacts",
        "pub struct NativeRunReport",
        "struct NativeVelloRunner",
        "struct EventLoopProxyRepaintSignal",
        "fn try_mark_repaint_event_pending",
        "const INCREMENTAL_FRAME_PIPELINE_ENV",
        "fn ui_action_pointer_coords",
        "fn select_present_mode",
        "fn startup_renderer_options",
        "enum RuntimeUserEvent",
        "crate::compat::legacy_shell",
        "crate::gui::native_shell",
        "../../../../src/app_core",
        "pub fn run_native_vello_app",
        "pub fn run_native_vello_app_with_artifacts",
        "pub fn run_native_vello_app_declarative",
        "pub fn run_native_vello_app_declarative_with_artifacts",
        "pub fn run_native_vello_preview",
        "pub fn capture_gui_automation_snapshot",
    ] {
        assert!(
            !native_vello.contains(forbidden),
            "legacy shell native Vello API `{forbidden}` belongs under src/compat/legacy_native_vello"
        );
    }
    let legacy_shell_runner = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/legacy_shell_runner.rs"),
    )
    .expect("legacy_shell_runner.rs should be readable");
    assert!(
        legacy_shell_runner.contains("struct NativeVelloRunner"),
        "legacy shell runner state should live in legacy_shell_runner.rs"
    );
    let legacy_shell_config = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/legacy_shell_config.rs"),
    )
    .expect("legacy_shell_config.rs should be readable");
    for required in [
        "const INCREMENTAL_FRAME_PIPELINE_ENV",
        "fn ui_action_pointer_coords",
    ] {
        assert!(
            legacy_shell_config.contains(required),
            "legacy shell runtime tuning helpers should live in legacy_shell_config.rs"
        );
    }
    let legacy_shell_runtime = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/legacy_shell_runtime.rs"),
    )
    .expect("legacy_shell_runtime.rs should be readable");
    for required in [
        "CoalescingRepaintSignal",
        "run_legacy_shell_vello_app_with_artifacts",
    ] {
        assert!(
            legacy_shell_runtime.contains(required),
            "legacy shell event-loop compatibility glue should live in legacy_shell_runtime.rs"
        );
    }
    let repaint = fs::read_to_string(manifest_dir.join("src/gui/repaint.rs"))
        .expect("generic repaint module should be readable");
    for required in [
        "pub fn try_mark_repaint_pending",
        "pub struct CoalescingRepaintSignal",
    ] {
        assert!(
            repaint.contains(required),
            "repaint coalescing primitive `{required}` belongs in generic gui::repaint"
        );
    }
    let expectations: &[(&str, &[&str])] = &[
        (
            "src/lib.rs",
            &[
                "#[cfg(feature = \"legacy-shell\")]\npub(crate) mod app",
                "#[cfg(feature = \"legacy-shell\")]\n#[path = \"compat/legacy_shell/mod.rs\"]",
            ],
        ),
        (
            "src/compat.rs",
            &[
                "#[cfg(feature = \"legacy-shell\")]\npub mod legacy_native_vello",
                "#[cfg(feature = \"legacy-shell\")]\npub mod legacy_shell",
            ],
        ),
        (
            "src/gui/mod.rs",
            &["#[cfg(feature = \"legacy-shell\")]\npub(crate) mod native_shell"],
        ),
        (
            "src/gui_runtime/native_vello.rs",
            &[
                "#[cfg(feature = \"legacy-shell\")]\nmod input",
                "#[cfg(feature = \"legacy-shell\")]\nmod legacy_shell_config",
                "#[cfg(feature = \"legacy-shell\")]\nmod legacy_shell_prelude",
                "#[cfg(feature = \"legacy-shell\")]\nmod legacy_shell_runner",
                "#[cfg(feature = \"legacy-shell\")]\nmod legacy_shell_runtime",
                "#[cfg(feature = \"legacy-shell\")]\nmod legacy_shell_text_entry",
                "mod runtime_config",
                "mod runtime_event",
                "#[cfg(feature = \"legacy-shell\")]\nmod runtime_events",
                "#[cfg(all(test, feature = \"legacy-shell\"))]\nmod tests",
            ],
        ),
    ];
    for (relative, required) in expectations {
        let path = manifest_dir.join(relative);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        let source = source.replace("\r\n", "\n");
        for needle in *required {
            assert!(
                source.contains(needle),
                "{relative} should gate legacy shell compatibility with `{needle}`"
            );
        }
    }
}

#[test]
fn legacy_shell_contract_keeps_runtime_helpers_out_of_model_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module_path = manifest_dir.join("src/compat/legacy_shell/mod.rs");
    let source = fs::read_to_string(&module_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", module_path.display()));
    let native_vello_path = manifest_dir.join("src/compat/legacy_native_vello.rs");
    let native_vello = fs::read_to_string(&native_vello_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", native_vello_path.display()));

    assert!(
        !source.contains("DEFAULT_APP_TITLE"),
        "Radiant compatibility contracts should not re-export application-shaped title aliases; \
         use gui_runtime::DEFAULT_NATIVE_WINDOW_TITLE for the generic runtime default"
    );
    assert!(
        !native_vello.contains("run_native_vello_app_declarative"),
        "Radiant legacy native Vello compatibility should expose one runner entrypoint; host apps may keep declarative aliases in their own runtime facade"
    );
    assert!(
        !native_vello.contains("run_native_vello_app,")
            && !native_vello.contains("pub fn run_native_vello_app<"),
        "Radiant legacy native Vello compatibility should expose only the artifact-returning runner; host apps can derive result-only wrappers locally"
    );
    assert!(
        !source.contains("capture_gui_automation_snapshot")
            && !source.contains("capture_native_shell_shot_snapshot"),
        "host shell snapshot capture helpers belong in the consuming application once the local shell scaffold owns them"
    );
    assert!(
        !native_vello.contains("run_native_vello_preview")
            && !native_vello.contains("PreviewBridge"),
        "backend preview helpers should not remain in the legacy native Vello compatibility entrypoint"
    );
    assert!(
        !source.contains("run_native_vello_app_with_artifacts"),
        "legacy_shell should expose model/action/bridge contracts only; native runtime entrypoints belong under compat::legacy_native_vello"
    );
    assert!(
        native_vello.contains("pub fn run_native_vello_app_with_artifacts"),
        "compat::legacy_native_vello should expose the current artifact-returning compatibility runner"
    );
}

#[test]
fn legacy_shell_facade_is_reexport_only_glue() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module_path = manifest_dir.join("src/compat/legacy_shell/mod.rs");
    let source = fs::read_to_string(&module_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", module_path.display()));

    for forbidden in [
        "pub struct ",
        "pub enum ",
        "pub type ",
        "pub trait ",
        "impl ",
        "fn ",
    ] {
        assert!(
            !source.contains(forbidden),
            "Radiant legacy shell facade must stay re-export-only glue; `{forbidden}` belongs in host-owned compatibility modules"
        );
    }

    assert!(
        source.contains("pub use actions::{BrowserTriageTarget, UiAction};")
            && source.contains("pub use shell::{")
            && source.contains("pub use sources::SourcesPanelModel;")
            && source.contains(
                "pub use waveform::{NormalizedRangeModel, WaveformChromeModel, WaveformPanelModel};"
            ),
        "legacy_shell/mod.rs should remain explicit re-export glue while the compatibility feature exists"
    );
}

#[test]
fn core_api_documentation_covers_public_boundary_concepts() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs_path = manifest_dir.join("docs/API.md");
    let docs = fs::read_to_string(&docs_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", docs_path.display()));

    for required in [
        "# Radiant Core API",
        "Dependency Boundary",
        "## App",
        "View, Element, And Widget",
        "Message And Command",
        "## Layout",
        "VirtualListWindow",
        "virtual_list_view_start_after_scroll_delta",
        "virtual_list_scroll_delta_from_units",
        "fixed_width_row_rects_start",
        "visible_suffix_widths",
        "LayoutOutput::rect_for",
        "LayoutOutput::rect_for_clamped",
        "grouped_fixed_width_row_width",
        "fixed_width_item_extent_for_available_width",
        "ContentListPanel<Row, Editor>",
        "ContentListActions",
        "SignalChromeState",
        "horizontal_progress_fill_rect",
        "horizontal_progress_activity_rect",
        "horizontal_progress_track_rect",
        "horizontal_meter_fill_rect",
        "horizontal_discrete_meter_fill_rect",
        "inline_indicator_layout",
        "SignalToolState",
        "SignalRasterPreview",
        "TimelineViewport",
        "normalized_milli_point_in_rect",
        "TimelineTransportState",
        "TimelineEditPreview",
        "TimelineFeedbackEvents",
        "TimelinePresentationState",
        "Style And Theme",
        "## Renderer",
        "## Context",
        "Event And Focus",
        "logical_point_to_u16_coords",
        "snap_text_baseline_to_pixel",
        "Rect::center",
        "empty_at_max",
        "inset_horizontal",
        "inset_vertical",
        "split_at_y",
        "inset_horizontal_saturating",
        "inset_uniform_saturating",
        "centered_pixel_square",
        "centered_odd_pixel_square",
        "stroke_aligned_rect",
        "top_right_square",
        "top_edge_strip",
        "bottom_edge_strip",
        "left_edge_strip",
        "right_edge_strip",
        "Rect::union",
        "## Automation",
        "Generic Panels And Forms",
        "anchored_panel_rect",
        "InlineBadgeMetrics",
        "inline_badge_rects_for_labels",
        "Invalidation And Lifecycle",
        "GuiAutomationSnapshot",
        "AutomationNodeSnapshot",
        "VisualSnapshot",
        "SnapshotPrimitive",
        "SnapshotTextRun",
        "visual_snapshot_from_paint_frame",
        "UiSurface",
        "SurfaceNode",
        "SurfaceNode::badge",
        "SurfaceNode::card",
        "SurfaceNode::stack",
        "SurfaceNode::grid",
        "SurfaceNode::text_input",
        "SurfaceNode::toggle",
        "SurfaceNode::scrollbar",
        "SurfaceNode::list_item",
        "SurfaceNode::list_item_action",
        "SurfaceNode::list_item_mapped",
        "SurfaceNode::selectable",
        "SurfaceNode::selectable_mapped",
        "SurfaceNode::scroll_area",
        "SurfaceNode::virtual_scroll_area",
        "SurfaceNode::image",
        "SurfaceNode::canvas",
        "WidgetSpec",
        "WidgetId",
        "Command<Message>",
        "RuntimeRunReport<Artifacts>",
        "RuntimeBridge",
        "SurfaceRuntime",
        "ThemeTokens",
        "SurfacePaintPlan",
        "SplitPaneSidebarState",
        "InvalidationMask",
        "RetainedSegmentMask",
        "RetainedSegmentRevisions",
        "ContentViewChrome",
        "PairedStatusPanel",
        "PreferencePanelState",
        "TimelineSurfaceState",
    ] {
        assert!(
            docs.contains(required),
            "docs/API.md should document the public API concept `{required}`"
        );
    }

    assert!(
        docs.contains("host -> Radiant, never Radiant -> host"),
        "docs/API.md should make the host -> Radiant dependency direction explicit"
    );
    let normalized_docs = docs.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        normalized_docs.contains("compat::legacy_shell")
            && normalized_docs.contains("compat::legacy_native_vello")
            && normalized_docs.contains("temporary")
            && normalized_docs.contains("host-shaped model/action/bridge migration glue")
            && normalized_docs.contains("temporary native runtime entrypoint")
            && normalized_docs.contains(
                "generic Radiant code must not grow new dependencies on either compatibility namespace"
            ),
        "docs/API.md should identify legacy compatibility namespaces as temporary host-owned migration glue"
    );
}

#[derive(Debug)]
struct ExtractionRule {
    pattern: String,
    disposition: String,
    owner: String,
}

#[test]
fn radiant_source_docs_and_examples_do_not_name_host_product() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let matches = host_product_name_matches(&manifest_dir);
    assert!(
        matches.is_empty(),
        "Radiant source, docs, and examples must stay product-neutral; found host-product names:\n{}",
        matches.join("\n")
    );
}

#[test]
fn radiant_docs_do_not_use_host_product_domain_examples() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs_dir = manifest_dir.join("docs");
    let mut violations = Vec::new();

    collect_markdown_token_violations(
        &docs_dir,
        &manifest_dir,
        DOC_PRODUCT_DOMAIN_TERMS,
        &mut violations,
    );

    assert!(
        violations.is_empty(),
        "Radiant docs should describe reusable GUI concepts without host-product domain examples:\n{}",
        violations.join("\n")
    );
}

#[test]
fn domain_extraction_inventory_covers_current_domain_bearing_files() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rules = parse_extraction_inventory();
    let files = domain_bearing_rust_files(&manifest_dir);
    let mut matched_rules: BTreeMap<&str, usize> = rules
        .iter()
        .map(|rule| (rule.pattern.as_str(), 0))
        .collect();
    let mut violations = Vec::new();

    for file in &files {
        let matches = rules
            .iter()
            .filter(|rule| rule.matches(file))
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            violations.push(format!(
                "{file} should match exactly one extraction inventory rule, matched {:?}",
                matches
                    .iter()
                    .map(|rule| rule.pattern.as_str())
                    .collect::<Vec<_>>()
            ));
            continue;
        }
        *matched_rules.get_mut(matches[0].pattern.as_str()).unwrap() += 1;
    }

    let unused_rules = matched_rules
        .iter()
        .filter_map(|(pattern, count)| (*count == 0).then_some(*pattern))
        .collect::<Vec<_>>();
    if !unused_rules.is_empty() {
        violations.push(format!(
            "extraction inventory contains rules that match no current domain-bearing Rust files: {unused_rules:?}"
        ));
    }

    assert!(
        violations.is_empty(),
        "every current Radiant file with host-product domain terms must have a final extraction disposition:\n{}",
        violations.join("\n")
    );
}

#[test]
fn domain_extraction_inventory_uses_known_dispositions_and_owners() {
    let rules = parse_extraction_inventory();

    for expected_disposition in ["move_to_host", "split_generic_from_compat"] {
        assert!(
            rules
                .iter()
                .any(|rule| rule.disposition == expected_disposition),
            "domain extraction inventory should include at least one {expected_disposition} rule"
        );
    }

    for expected_owner in ["host_app", "radiant_boundary"] {
        assert!(
            rules.iter().any(|rule| rule.owner == expected_owner),
            "domain extraction inventory should include at least one {expected_owner} rule"
        );
    }
}

fn collect_violations(path: &Path, manifest_dir: &Path, violations: &mut Vec<String>) {
    if path.is_dir() {
        let mut entries = fs::read_dir(path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
            .map(|entry| {
                entry
                    .unwrap_or_else(|err| {
                        panic!("failed to read entry in {}: {err}", path.display())
                    })
                    .path()
            })
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            collect_violations(&entry, manifest_dir, violations);
        }
        return;
    }

    if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
        return;
    }

    collect_token_violations(path, manifest_dir, FORBIDDEN_GENERIC_TOKENS, violations);
}

fn collect_token_violations(
    path: &Path,
    manifest_dir: &Path,
    forbidden_tokens: &[&str],
    violations: &mut Vec<String>,
) {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    let uncommented = strip_rust_comments(&source);
    for (line_index, line) in uncommented.lines().enumerate() {
        let normalized = line
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>();
        for token in forbidden_tokens {
            if normalized.contains(token) {
                let relative = path.strip_prefix(manifest_dir).unwrap_or(path);
                violations.push(format!(
                    "{}:{} imports or names `{}`",
                    relative.display(),
                    line_index + 1,
                    token
                ));
            }
        }
    }
}

fn collect_markdown_token_violations(
    path: &Path,
    manifest_dir: &Path,
    forbidden_tokens: &[&str],
    violations: &mut Vec<String>,
) {
    if !path.exists() {
        return;
    }
    if path.is_dir() {
        let mut entries = fs::read_dir(path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
            .map(|entry| {
                entry
                    .unwrap_or_else(|err| {
                        panic!("failed to read entry in {}: {err}", path.display())
                    })
                    .path()
            })
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            collect_markdown_token_violations(&entry, manifest_dir, forbidden_tokens, violations);
        }
        return;
    }
    if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
        return;
    }

    let source = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    for (line_index, line) in source.lines().enumerate() {
        for token in forbidden_tokens {
            if line.contains(token) {
                let relative = path.strip_prefix(manifest_dir).unwrap_or(path);
                violations.push(format!(
                    "{}:{} names `{}`",
                    relative.display(),
                    line_index + 1,
                    token
                ));
            }
        }
    }
}

fn strip_rust_comments(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut block_depth = 0usize;

    while let Some(ch) = chars.next() {
        if block_depth > 0 {
            if ch == '/' && chars.peek() == Some(&'*') {
                chars.next();
                block_depth += 1;
            } else if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                block_depth -= 1;
            } else if ch == '\n' {
                output.push('\n');
            }
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'/') {
            for next in chars.by_ref() {
                if next == '\n' {
                    output.push('\n');
                    break;
                }
            }
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            block_depth = 1;
            continue;
        }

        output.push(ch);
    }

    output
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
        let disposition = columns[1].to_owned();
        assert!(
            INVENTORY_DISPOSITIONS.contains(&disposition.as_str()),
            "unknown extraction disposition {disposition:?} on line {}",
            line_index + 1
        );
        let owner = columns[2].to_owned();
        assert!(
            INVENTORY_OWNERS.contains(&owner.as_str()),
            "unknown extraction owner {owner:?} on line {}",
            line_index + 1
        );
        rules.push(ExtractionRule {
            pattern: columns[0].to_owned(),
            disposition,
            owner,
        });
    }
    assert!(
        !rules.is_empty(),
        "domain extraction inventory should not be empty"
    );
    rules
}

impl ExtractionRule {
    fn matches(&self, file: &str) -> bool {
        if let Some(prefix) = self.pattern.strip_suffix("/**") {
            file.starts_with(&format!("{prefix}/"))
        } else {
            self.pattern == file
        }
    }
}

fn host_product_slug() -> &'static str {
    concat!("sem", "pal")
}

fn host_product_display_name() -> &'static str {
    concat!("Sem", "pal")
}

fn host_product_name_matches(manifest_dir: &Path) -> Vec<String> {
    let mut matches = Vec::new();
    for root in HOST_PRODUCT_NAME_SCAN_ROOTS {
        collect_host_product_name_matches(&manifest_dir.join(root), manifest_dir, &mut matches);
    }
    matches.sort();
    matches
}

fn collect_host_product_name_matches(path: &Path, manifest_dir: &Path, matches: &mut Vec<String>) {
    if !path.exists() {
        return;
    }
    if path.is_dir() {
        let mut entries = fs::read_dir(path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
            .map(|entry| {
                entry
                    .unwrap_or_else(|err| {
                        panic!("failed to read entry in {}: {err}", path.display())
                    })
                    .path()
            })
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            collect_host_product_name_matches(&entry, manifest_dir, matches);
        }
        return;
    }

    let extension = path.extension().and_then(|extension| extension.to_str());
    if !matches!(extension, Some("rs" | "md")) {
        return;
    }

    let source = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    for (line_index, line) in source.lines().enumerate() {
        if line.contains(host_product_display_name()) || line.contains(host_product_slug()) {
            let relative = path
                .strip_prefix(manifest_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            matches.push(format!("{}:{}", relative, line_index + 1));
        }
    }
}

fn domain_bearing_rust_files(manifest_dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    for root in DOMAIN_SCAN_ROOTS {
        collect_domain_bearing_rust_files(&manifest_dir.join(root), manifest_dir, &mut files);
    }
    files.sort();
    files.dedup();
    files
}

fn collect_domain_bearing_rust_files(path: &Path, manifest_dir: &Path, files: &mut Vec<String>) {
    if !path.exists() {
        return;
    }
    if path.is_dir() {
        let mut entries = fs::read_dir(path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
            .map(|entry| {
                entry
                    .unwrap_or_else(|err| {
                        panic!("failed to read entry in {}: {err}", path.display())
                    })
                    .path()
            })
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            collect_domain_bearing_rust_files(&entry, manifest_dir, files);
        }
        return;
    }

    if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
        return;
    }

    let relative = path
        .strip_prefix(manifest_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    if DOMAIN_SCAN_EXEMPT_FILES.contains(&relative.as_str()) {
        return;
    }
    let source = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    let source = strip_domain_scan_false_positives(&source);
    if contains_domain_term(&source) {
        files.push(relative);
    }
}

fn strip_domain_scan_false_positives(source: &str) -> String {
    source.replace("#[serde(tag = \"kind\", rename_all = \"snake_case\")]", "")
}

fn contains_domain_term(source: &str) -> bool {
    DOMAIN_TERMS
        .iter()
        .any(|term| contains_domain_term_occurrence(source, term))
}

fn contains_domain_term_occurrence(source: &str, term: &str) -> bool {
    if term.contains('-') {
        return source.contains(term);
    }
    source.match_indices(term).any(|(start, _)| {
        let before = source[..start].chars().next_back();
        let after = source[start + term.len()..].chars().next();
        is_term_boundary(before) && is_term_boundary(after)
    })
}

fn is_term_boundary(ch: Option<char>) -> bool {
    ch.is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
}
