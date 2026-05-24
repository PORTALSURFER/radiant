use super::*;

#[test]
fn split_pane_assigned_rows_use_named_parts_for_assignment_flags() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/panel/split_pane/assigned_row.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct SplitPaneAssignment")
            && source.contains("pub struct SplitPaneAssignedRowParts")
            && source.contains("pub fn from_parts(parts: SplitPaneAssignedRowParts) -> Self"),
        "split-pane assigned rows should expose named parts for readable public construction"
    );
    assert!(
        source.contains("Self::from_parts(SplitPaneAssignedRowParts {")
            && source.contains("self.with_assignment(SplitPaneAssignment { upper, lower })"),
        "split-pane compatibility constructors should delegate through named assignment objects"
    );
}

#[test]
fn floating_panel_drags_use_named_parts_for_pointer_geometry() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/panel/floating.rs");
    let module_path = manifest_dir.join("src/gui/panel.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let module = fs::read_to_string(&module_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", module_path.display()));

    assert!(
        source.contains("pub struct FloatingPanelDragParts")
            && source.contains("pub fn from_parts(parts: FloatingPanelDragParts) -> Self"),
        "floating-panel drags should expose named parts for panel rect and pointer geometry"
    );
    assert!(
        source.contains("Self::from_parts(FloatingPanelDragParts {")
            && module.contains("FloatingPanelDragParts"),
        "floating-panel drag compatibility constructor and panel export should keep the named-parts path available"
    );
}

#[test]
fn panel_rect_helpers_use_named_parts_for_geometry_requests() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let anchored_path = manifest_dir.join("src/gui/panel/anchored.rs");
    let floating_path = manifest_dir.join("src/gui/panel/floating.rs");
    let module_path = manifest_dir.join("src/gui/panel.rs");
    let tests_path = manifest_dir.join("src/gui/panel/tests.rs");
    let anchored = fs::read_to_string(&anchored_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", anchored_path.display()));
    let floating = fs::read_to_string(&floating_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", floating_path.display()));
    let module = fs::read_to_string(&module_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", module_path.display()));
    let tests = fs::read_to_string(&tests_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", tests_path.display()));

    assert!(
        anchored.contains("pub struct AnchoredPanelRectParts")
            && anchored.contains("pub fn anchored_panel_rect_from_parts(")
            && anchored.contains("anchored_panel_rect_from_parts(AnchoredPanelRectParts {"),
        "anchored panel geometry should expose named parts and keep the positional helper as a wrapper"
    );
    assert!(
        floating.contains("pub struct FloatingPanelRectParts")
            && floating.contains("pub fn floating_panel_rect_from_parts(")
            && floating.contains("floating_panel_rect_from_parts(FloatingPanelRectParts {"),
        "floating panel geometry should expose named parts and keep the positional helper as a wrapper"
    );
    assert!(
        module.contains("AnchoredPanelRectParts")
            && module.contains("anchored_panel_rect_from_parts")
            && module.contains("FloatingPanelRectParts")
            && module.contains("floating_panel_rect_from_parts"),
        "panel geometry named parts should remain exported through the panel facade"
    );
    assert!(
        tests.contains("fn anchored_panel_rect_compatibility_helper_delegates_to_named_parts")
            && tests
                .contains("fn floating_panel_rect_compatibility_helper_delegates_to_named_parts"),
        "panel geometry tests should cover named-parts construction and compatibility wrappers"
    );
}

#[test]
fn inline_badge_metrics_use_named_parts_for_geometry_tokens() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let badge = fs::read_to_string(manifest_dir.join("src/gui/badge.rs"))
        .expect("badge facade should be readable");
    let model = fs::read_to_string(manifest_dir.join("src/gui/badge/model.rs"))
        .expect("badge model module should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/badge/tests.rs"))
        .expect("badge behavior tests should be readable");
    let root = fs::read_to_string(manifest_dir.join("src/gui/badge/inline.rs"))
        .expect("inline badge root should be readable");
    let metrics = fs::read_to_string(manifest_dir.join("src/gui/badge/inline/metrics.rs"))
        .expect("inline badge metrics should be readable");
    let labels = fs::read_to_string(manifest_dir.join("src/gui/badge/inline/labels.rs"))
        .expect("inline badge labels should be readable");
    let geometry = fs::read_to_string(manifest_dir.join("src/gui/badge/inline/geometry.rs"))
        .expect("inline badge geometry should be readable");

    assert!(
        badge.contains("mod model;")
            && badge.contains("PillEditorChoices")
            && badge.contains("PillEditorInput")
            && badge.contains("PillEditorPanel")
            && badge.contains("PillEditorStatus")
            && badge.contains("SelectablePill")
            && badge.contains("#[path = \"badge/tests.rs\"]")
            && !badge.contains("pub struct SelectablePill")
            && !badge.contains("fn selectable_pill_preserves_identity_label_and_state"),
        "badge facade should re-export focused pill models and keep behavior tests out of the root module"
    );
    assert!(
        model.contains("pub struct SelectablePill")
            && model.contains("pub struct PillEditorStatus")
            && model.contains("pub struct PillEditorInput")
            && model.contains("pub struct PillEditorChoices")
            && model.contains("pub struct PillEditorPanel")
            && tests.contains("fn selectable_pill_preserves_identity_label_and_state")
            && tests.contains("fn inline_badge_rects_handle_empty_or_cramped_inputs"),
        "badge model DTOs should stay split into focused submodels, with behavior tests in badge/tests.rs"
    );
    assert!(
        root.contains("mod geometry;")
            && root.contains("mod labels;")
            && root.contains("mod metrics;")
            && root.contains("pub use metrics::{InlineBadgeMetrics, InlineBadgeMetricsParts};"),
        "inline badge root should delegate metrics, label parsing, and geometry helpers"
    );
    assert!(
        metrics.contains("pub struct InlineBadgeMetricsParts")
            && metrics.contains("pub fn from_parts(parts: InlineBadgeMetricsParts) -> Self"),
        "inline badge metrics should expose named parts for readable public construction"
    );
    assert!(
        metrics.contains("Self::from_parts(InlineBadgeMetricsParts {"),
        "the positional compatibility constructor should delegate through the named metrics object"
    );
    assert!(
        labels.contains("pub fn inline_badge_labels")
            && labels.contains("pub fn inline_badge_labels_owned_into"),
        "inline badge label splitting and materialization should live in inline/labels.rs"
    );
    assert!(
        geometry.contains("pub fn inline_badge_rects_for_labels_into")
            && geometry.contains("pub fn inline_badge_text_origin")
            && geometry.contains("pub fn inline_badge_cluster_reserved_width"),
        "inline badge geometry and text placement should live in inline/geometry.rs"
    );
}
