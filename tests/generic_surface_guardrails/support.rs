pub(crate) use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

pub(crate) use radiant::{
    layout::Vector2,
    runtime::{SurfaceNode, UiSurface, WidgetMessageMapper},
    widgets::{TextWidget, WidgetOutput, WidgetSizing},
};

pub(crate) const GENERIC_SOURCE_ROOTS: &[&str] = &[
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
    "src/gui/text_layout",
    "src/gui/types.rs",
    "src/gui/undo.rs",
    "src/gui/visualization.rs",
];

pub(crate) const EXEMPT_TOP_LEVEL_GUI_FILES: &[&str] = &["src/gui/mod.rs"];

pub(crate) const REQUIRED_BEHAVIOR_TESTS: &[&str] = &[
    "app_runtime_api.rs",
    "application_builder_public_api.rs",
    "custom_widget_public_api.rs",
    "generic_surface_guardrails.rs",
    "layout_public_api.rs",
    "runtime_bridge_public_api.rs",
    "runtime_surface_public_api.rs",
    "surface_hover_public_api.rs",
    "surface_node_public_api.rs",
    "surface_scroll_public_api.rs",
    "surface_widget_helpers_public_api.rs",
    "widgets_primitive_behaviors.rs",
    "widgets_public_api.rs",
];

pub(crate) fn relative_path(manifest_dir: &Path, path: &Path) -> String {
    path.strip_prefix(manifest_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn rust_sources_under(path: &Path) -> Vec<PathBuf> {
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

pub(crate) fn strip_toml_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split_once('#').map_or(line, |(before, _)| before))
        .collect::<Vec<_>>()
        .join("\n")
}
