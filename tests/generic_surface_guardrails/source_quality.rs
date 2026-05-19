//! Source-quality guardrails for focused modules and readable public models.

use std::{fs, path::PathBuf};

use super::relative_path;

#[test]
fn application_view_lowering_keeps_container_defaults_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module = fs::read_to_string(manifest_dir.join("src/application/view_node.rs"))
        .expect("application view node module should be readable");
    let lowering = fs::read_to_string(manifest_dir.join("src/application/view_node/lowering.rs"))
        .expect("application view lowering should be readable");
    let defaults =
        fs::read_to_string(manifest_dir.join("src/application/view_node/lowering_defaults.rs"))
            .expect("application view lowering defaults should be readable");

    assert!(
        module.contains("mod lowering_defaults;")
            && lowering.contains("ViewNodeContainerDefaults::new("),
        "view lowering should consume container defaults from a focused helper"
    );
    assert!(
        !lowering.contains("DEFAULT_STYLED_CONTAINER_PADDING")
            && defaults.contains("DEFAULT_STYLED_CONTAINER_PADDING")
            && defaults.contains("fn default_container_padding")
            && defaults.contains("fn base_policy"),
        "declarative container default policy should stay outside the main view lowering match"
    );
}

#[test]
fn public_layout_policy_models_do_not_hide_dead_code() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let model_dir = manifest_dir.join("src/gui/layout_core/model");
    let mut violations = Vec::new();

    for path in [
        model_dir.join("alignment.rs"),
        model_dir.join("container.rs"),
        model_dir.join("virtualization.rs"),
    ] {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        if source.contains("#[allow(dead_code)]") {
            violations.push(relative_path(&manifest_dir, &path));
        }
    }

    assert!(
        violations.is_empty(),
        "public layout policy models should be exported, tested, or removed instead of hiding dead-code warnings:\n{}",
        violations.join("\n")
    );
}

#[test]
fn linear_layout_hot_path_uses_request_objects_instead_of_argument_suppressions() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let linear_dir = manifest_dir.join("src/gui/layout_core/engine/layout/linear");
    let mut violations = Vec::new();

    for path in [
        linear_dir.join("placement.rs"),
        linear_dir.join("sizing.rs"),
    ] {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        if source.contains("too_many_arguments") {
            violations.push(relative_path(&manifest_dir, &path));
        }
    }

    assert!(
        violations.is_empty(),
        "linear layout measurement and placement should use cohesive request objects instead of suppressing long parameter lists:\n{}",
        violations.join("\n")
    );
}

#[test]
fn editable_tree_rows_use_named_parts_instead_of_boolean_constructor_lists() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/list/editable/row.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct EditableTreeRowParts")
            && source.contains("pub fn from_parts(parts: EditableTreeRowParts) -> Self"),
        "editable tree rows should expose a named parts object for readable public construction"
    );
    assert!(
        !source.contains("too_many_arguments"),
        "editable tree rows should not hide long positional constructors behind clippy suppressions"
    );
}

#[test]
fn column_summaries_use_named_parts_for_title_and_count() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/list/editable/column.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let editable = fs::read_to_string(manifest_dir.join("src/gui/list/editable.rs"))
        .expect("editable list module should be readable");
    let list = fs::read_to_string(manifest_dir.join("src/gui/list.rs"))
        .expect("list module should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        source.contains("pub struct ColumnSummaryParts")
            && source.contains("pub fn from_parts(parts: ColumnSummaryParts) -> Self"),
        "column summaries should expose named parts for title and item count"
    );
    assert!(
        source.contains("Self::from_parts(ColumnSummaryParts {")
            && editable.contains("ColumnSummaryParts")
            && list.contains("ColumnSummaryParts")
            && lib.contains("ColumnSummaryParts"),
        "column summary compatibility constructor and public exports should keep the named-parts path available"
    );
}

#[test]
fn virtual_list_stack_metrics_use_named_parts_for_geometry() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/list/virtual_list/geometry.rs");
    let virtual_list_path = manifest_dir.join("src/gui/list/virtual_list.rs");
    let list_path = manifest_dir.join("src/gui/list.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let virtual_list = fs::read_to_string(&virtual_list_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", virtual_list_path.display()));
    let list = fs::read_to_string(&list_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", list_path.display()));
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        source.contains("pub struct VirtualListStackMetricsParts")
            && source.contains("pub fn from_parts(parts: VirtualListStackMetricsParts) -> Self"),
        "virtual-list stack metrics should expose named parts for extent, gap, and viewport cap"
    );
    assert!(
        source.contains("Self::from_parts(VirtualListStackMetricsParts {")
            && virtual_list.contains("VirtualListStackMetricsParts")
            && list.contains("VirtualListStackMetricsParts")
            && lib.contains("VirtualListStackMetricsParts"),
        "virtual-list stack metric constructor and public exports should keep the named-parts path available"
    );
}

#[test]
fn normalized_ranges_use_named_parts_for_milli_bounds() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/range.rs"))
        .expect("range root should be readable");
    let source_path = manifest_dir.join("src/gui/range/interval.rs");
    let interval = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        root.contains("mod interval;")
            && root.contains("pub use interval::{NormalizedRange, NormalizedRangeParts};")
            && !root.contains("pub struct NormalizedRange"),
        "range root should re-export the normalized interval model without owning its implementation"
    );

    assert!(
        interval.contains("pub struct NormalizedRangeParts")
            && interval.contains("pub fn from_parts(parts: NormalizedRangeParts) -> Self"),
        "normalized ranges should expose named parts for start and end milli-unit bounds"
    );
    assert!(
        interval.contains("Self::from_parts(NormalizedRangeParts {"),
        "normalized range compatibility constructor should keep the named-parts path available"
    );
}

#[test]
fn index_viewport_model_keeps_behavior_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let range = fs::read_to_string(manifest_dir.join("src/gui/range.rs"))
        .expect("range facade should be readable");
    let model = fs::read_to_string(manifest_dir.join("src/gui/range/index_viewport.rs"))
        .expect("index viewport model should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/range/index_viewport/tests.rs"))
        .expect("index viewport behavior tests should be readable");

    assert!(
        range.contains("pub use index_viewport::IndexViewport;")
            && model.contains("pub struct IndexViewport")
            && model.contains("#[path = \"index_viewport/tests.rs\"]")
            && !model.contains("fn index_viewport_clamps_visible_span_and_offset_fraction"),
        "index viewport should stay exported through the range facade while keeping behavior tests out of the model root"
    );
    assert!(
        tests.contains("fn index_viewport_clamps_visible_span_and_offset_fraction")
            && tests.contains("fn index_viewport_zooms_and_pans_around_visible_anchor")
            && tests.contains("fn index_viewport_sets_offset_and_projects_visible_ratio"),
        "index viewport behavior coverage should live in range/index_viewport/tests.rs"
    );
}

#[test]
fn normalized_viewports_use_named_parts_for_precision_bounds() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/range/viewport.rs");
    let module_path = manifest_dir.join("src/gui/range.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let projection = fs::read_to_string(manifest_dir.join("src/gui/range/viewport/projection.rs"))
        .expect("normalized viewport projection source should be readable");
    let module = fs::read_to_string(&module_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", module_path.display()));

    assert!(
        source.contains("pub struct NormalizedViewportParts")
            && source.contains("pub fn from_parts(parts: NormalizedViewportParts) -> Self"),
        "normalized viewports should expose named parts for micro and optional nano bounds"
    );
    assert!(
        source.contains("Self::from_parts(NormalizedViewportParts {")
            && source.contains("mod projection;")
            && source.contains("projection::x_for_ratio")
            && !source.contains("fn finite_ordered_x_bounds")
            && module.contains("NormalizedViewportParts"),
        "normalized viewport compatibility constructor and range export should keep the named-parts path available"
    );
    assert!(
        projection.contains("fn local_ratio")
            && projection.contains("fn x_for_ratio")
            && projection.contains("fn finite_ordered_x_bounds"),
        "normalized viewport projection math and x-bound sanitization should live in viewport/projection.rs"
    );
}

#[test]
fn normalized_scrollbars_keep_model_and_geometry_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/range/scrollbar.rs"))
        .expect("normalized scrollbar root should be readable");
    let model = fs::read_to_string(manifest_dir.join("src/gui/range/scrollbar/model.rs"))
        .expect("normalized scrollbar model should be readable");
    let geometry = fs::read_to_string(manifest_dir.join("src/gui/range/scrollbar/geometry.rs"))
        .expect("normalized scrollbar geometry should be readable");
    let range = fs::read_to_string(manifest_dir.join("src/gui/range.rs"))
        .expect("range facade should be readable");

    assert!(
        root.contains("mod geometry;")
            && root.contains("mod model;")
            && root.contains("pub use geometry::{")
            && root.contains("pub use model::{NormalizedScrollbar")
            && !root.contains("pub struct NormalizedScrollbarRequest")
            && !root.contains("fn clamped_normalized_span"),
        "normalized scrollbar root should re-export focused model and geometry modules"
    );
    assert!(
        model.contains("pub struct NormalizedScrollbarRequest")
            && model.contains("pub struct NormalizedScrollbar"),
        "normalized scrollbar public DTOs should live in scrollbar/model.rs"
    );
    assert!(
        geometry.contains("pub fn resolve_normalized_scrollbar")
            && geometry.contains("pub fn normalized_scrollbar_center_for_pointer")
            && geometry.contains("fn clamped_normalized_span")
            && !geometry.contains("pub struct NormalizedScrollbar"),
        "normalized scrollbar geometry and pointer resolution should live in scrollbar/geometry.rs"
    );
    assert!(
        range.contains("NormalizedScrollbarRequest")
            && range.contains("resolve_normalized_scrollbar")
            && range.contains("normalized_scrollbar_center_at_point"),
        "normalized scrollbar public API should remain available through the range facade"
    );
}

#[test]
fn preference_panel_state_uses_named_parts_for_projection_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/form.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct PreferencePanelParts")
            && source.contains("pub fn from_parts(parts: PreferencePanelParts<TOGGLES>) -> Self"),
        "preference panel state should expose a named parts object for readable public construction"
    );
    assert!(
        source.contains("Self::from_parts(PreferencePanelParts {"),
        "the positional compatibility constructor should delegate through the named parts object"
    );
}

#[test]
fn signal_visualization_state_uses_named_parts_for_status_and_preview_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/visualization/signal.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let chrome = fs::read_to_string(manifest_dir.join("src/gui/visualization/signal/chrome.rs"))
        .expect("signal chrome state source should be readable");
    let preview = fs::read_to_string(manifest_dir.join("src/gui/visualization/signal/preview.rs"))
        .expect("signal raster preview source should be readable");
    let tools = fs::read_to_string(manifest_dir.join("src/gui/visualization/signal/tools.rs"))
        .expect("signal tool state source should be readable");

    for required in [
        "mod chrome;",
        "mod preview;",
        "mod tools;",
        "pub use chrome::{ChannelViewMode, SignalChromeParts, SignalChromeState};",
        "pub use preview::{SignalRasterPreview, SignalRasterPreviewParts};",
        "pub use tools::{SignalToolFlags, SignalToolState};",
    ] {
        assert!(
            source.contains(required),
            "signal visualization root should keep public re-exports while delegating `{required}`"
        );
    }

    assert!(
        chrome.contains("pub struct SignalChromeParts")
            && chrome.contains("pub fn from_parts(parts: SignalChromeParts) -> Self"),
        "signal chrome state should expose named parts for readable public construction"
    );
    assert!(
        preview.contains("pub struct SignalRasterPreviewParts")
            && preview.contains("pub fn from_parts(parts: SignalRasterPreviewParts) -> Self"),
        "signal raster preview state should expose named parts for readable public construction"
    );
    assert!(
        chrome.contains("Self::from_parts(SignalChromeParts {")
            && preview.contains("Self::from_parts(SignalRasterPreviewParts {"),
        "signal compatibility constructors should delegate through named parts objects"
    );
    assert!(
        !source.contains("pub struct SignalChromeState")
            && !source.contains("pub struct SignalRasterPreview")
            && !source.contains("pub struct SignalToolState")
            && chrome.contains("pub enum ChannelViewMode")
            && preview.contains("Arc<ImageRgba>")
            && tools.contains("pub struct SignalToolFlags")
            && tools.contains("pub fn from_flags(flags: SignalToolFlags) -> Self"),
        "signal chrome, raster preview, and tool flags should stay in focused visualization modules"
    );
}

#[test]
fn canvas_layer_state_uses_named_parts_for_hit_test_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/visualization/canvas.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct CanvasLayerParts")
            && source.contains("pub fn from_parts(parts: CanvasLayerParts) -> Self"),
        "canvas layer state should expose named parts for readable public construction"
    );
    assert!(
        source.contains("Self::from_parts(CanvasLayerParts {"),
        "the positional compatibility constructor should delegate through the named parts object"
    );
}

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
            && badge.contains("pub use model::{PillEditorPanel, SelectablePill};")
            && badge.contains("#[path = \"badge/tests.rs\"]")
            && !badge.contains("pub struct SelectablePill")
            && !badge.contains("fn selectable_pill_preserves_identity_label_and_state"),
        "badge facade should re-export focused pill models and keep behavior tests out of the root module"
    );
    assert!(
        model.contains("pub struct SelectablePill")
            && model.contains("pub struct PillEditorPanel")
            && tests.contains("fn selectable_pill_preserves_identity_label_and_state")
            && tests.contains("fn inline_badge_rects_handle_empty_or_cramped_inputs"),
        "badge model DTOs and behavior tests should live in focused badge/model.rs and badge/tests.rs modules"
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

#[test]
fn timeline_metadata_state_uses_named_parts_for_projection_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let timeline_dir = manifest_dir.join("src/gui/visualization/timeline");

    for (file, parts, from_parts, wrapper) in [
        (
            "transport.rs",
            "pub struct TimelineTransportParts",
            "pub fn from_parts(parts: TimelineTransportParts) -> Self",
            "Self::from_parts(TimelineTransportParts {",
        ),
        (
            "feedback.rs",
            "pub struct TimelineFeedbackParts",
            "pub fn from_parts(parts: TimelineFeedbackParts) -> Self",
            "Self::from_parts(TimelineFeedbackParts {",
        ),
        (
            "presentation.rs",
            "pub struct TimelinePresentationParts",
            "pub fn from_parts(parts: TimelinePresentationParts) -> Self",
            "Self::from_parts(TimelinePresentationParts {",
        ),
    ] {
        let source_path = timeline_dir.join(file);
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

        assert!(
            source.contains(parts) && source.contains(from_parts),
            "timeline metadata in {file} should expose named parts for readable public construction"
        );
        assert!(
            source.contains(wrapper),
            "timeline metadata compatibility constructor in {file} should delegate through named parts"
        );
    }
}

#[test]
fn timeline_viewport_uses_named_parts_for_precision_bounds() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/visualization/timeline/viewport.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct TimelineViewportParts")
            && source.contains("pub fn from_parts(parts: TimelineViewportParts) -> Self"),
        "timeline viewport should expose named parts for readable multi-precision bounds"
    );
    assert!(
        source.contains("Self::from_parts(TimelineViewportParts {")
            && source.contains("Self::from_parts(TimelineViewportParts::default())"),
        "timeline viewport compatibility/default constructors should delegate through named parts"
    );
}

#[test]
fn layout_constraints_use_named_parts_for_min_max_bounds() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/layout_core/constraints.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let module = fs::read_to_string(manifest_dir.join("src/gui/layout_core/mod.rs"))
        .expect("layout module should be readable");

    assert!(
        source.contains("pub struct ConstraintsParts")
            && source.contains("pub fn from_parts(parts: ConstraintsParts) -> Self"),
        "layout constraints should expose named parts for readable min/max bound construction"
    );
    assert!(
        source.contains("Self::from_parts(ConstraintsParts {")
            && module.contains("pub use constraints::{Constraints, ConstraintsParts};"),
        "layout constraint constructors and public exports should keep the named-parts path available"
    );
}

#[test]
fn layout_tree_nodes_use_named_parts_for_public_tree_construction() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/layout_core/tree.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let module = fs::read_to_string(manifest_dir.join("src/gui/layout_core/mod.rs"))
        .expect("layout module should be readable");

    for (parts, from_parts, wrapper) in [
        (
            "pub struct SlotChildParts",
            "pub fn from_parts(parts: SlotChildParts) -> Self",
            "Self::from_parts(SlotChildParts {",
        ),
        (
            "pub struct ContainerNodeParts",
            "pub fn from_parts(parts: ContainerNodeParts) -> Self",
            "Self::from_parts(ContainerNodeParts {",
        ),
        (
            "pub struct WidgetNodeParts",
            "pub fn from_parts(parts: WidgetNodeParts) -> Self",
            "Self::from_parts(WidgetNodeParts {",
        ),
    ] {
        assert!(
            source.contains(parts) && source.contains(from_parts) && source.contains(wrapper),
            "layout tree public nodes should expose named parts and compatibility wrappers for {parts}"
        );
    }
    assert!(
        source.contains("pub fn container_from_parts(parts: ContainerNodeParts) -> Self")
            && source.contains("pub fn widget_from_parts(parts: WidgetNodeParts) -> Self")
            && module.contains("ContainerNodeParts")
            && module.contains("SlotChildParts")
            && module.contains("WidgetNodeParts"),
        "layout tree named parts should be available through the public layout module"
    );
}

#[test]
fn runtime_surface_nodes_use_named_parts_for_public_tree_construction() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/runtime/surface/node.rs"))
        .expect("runtime surface node module should be readable");
    let builders = fs::read_to_string(manifest_dir.join("src/runtime/surface/builders.rs"))
        .expect("runtime surface builders should be readable");
    let container_builders =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/builders/container.rs"))
            .expect("runtime surface container builders should be readable");
    let leaf_builders =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/builders/leaf.rs"))
            .expect("runtime surface leaf builders should be readable");
    let overlay_builders =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/builders/overlay.rs"))
            .expect("runtime surface overlay builders should be readable");
    let surface = fs::read_to_string(manifest_dir.join("src/runtime/surface.rs"))
        .expect("runtime surface module should be readable");
    let runtime =
        fs::read_to_string(manifest_dir.join("src/runtime/mod.rs")).expect("runtime module");

    for (parts, from_parts, wrapper) in [
        (
            "pub struct SurfaceChildParts<Message>",
            "pub fn from_parts(parts: SurfaceChildParts<Message>) -> Self",
            "Self::from_parts(SurfaceChildParts {",
        ),
        (
            "pub struct SurfaceContainerParts<Message>",
            "pub fn from_parts(parts: SurfaceContainerParts<Message>) -> Self",
            "Self::from_parts(SurfaceContainerParts {",
        ),
    ] {
        assert!(
            source.contains(parts) && source.contains(from_parts) && source.contains(wrapper),
            "runtime surface nodes should expose named parts and compatibility wrappers for {parts}"
        );
    }
    assert!(
        builders.contains("mod container;")
            && builders.contains("mod leaf;")
            && builders.contains("mod overlay;")
            && !builders
                .contains("pub fn container_from_parts(parts: SurfaceContainerParts<Message>)")
            && !builders.contains("pub fn widget(")
            && !builders.contains("pub fn overlay_panel(")
            && container_builders.contains(
                "pub fn container_from_parts(parts: SurfaceContainerParts<Message>) -> Self"
            )
            && container_builders.contains("pub fn virtual_scroll_area(")
            && container_builders.contains("fn scroll_area_with_virtualization(")
            && leaf_builders.contains("pub fn widget(")
            && leaf_builders.contains("pub fn custom_widget_box(")
            && leaf_builders.contains("pub fn static_widget(")
            && overlay_builders.contains("pub fn overlay_panel(")
            && overlay_builders.contains("pub fn overlay_marker(")
            && surface.contains("SurfaceChildParts")
            && surface.contains("SurfaceContainerParts")
            && runtime.contains("SurfaceChildParts")
            && runtime.contains("SurfaceContainerParts"),
        "runtime surface builders should stay focused while named parts remain publicly available"
    );
}

#[test]
fn runtime_bridge_app_contract_stays_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bridge = fs::read_to_string(manifest_dir.join("src/runtime/bridge.rs"))
        .expect("runtime bridge module should be readable");
    let app = fs::read_to_string(manifest_dir.join("src/runtime/bridge/app.rs"))
        .expect("runtime bridge app contract module should be readable");
    let contract = fs::read_to_string(manifest_dir.join("src/runtime/bridge/contract.rs"))
        .expect("runtime bridge contract module should be readable");
    let runtime =
        fs::read_to_string(manifest_dir.join("src/runtime/mod.rs")).expect("runtime module");

    assert!(
        bridge.contains("mod app;")
            && bridge.contains("pub use app::App;")
            && runtime.contains("App,"),
        "runtime bridge root should publicly re-export the focused App contract"
    );
    assert!(
        app.contains("pub trait App<Message>: RuntimeBridge<Message>")
            && app.contains("impl<Bridge, Message> App<Message> for Bridge where Bridge: RuntimeBridge<Message> {}")
            && !contract.contains("pub trait App<Message>"),
        "the public App marker contract should stay in runtime/bridge/app.rs"
    );
}

#[test]
fn declarative_runtime_bridges_use_named_parts_for_host_closures() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let message =
        fs::read_to_string(manifest_dir.join("src/runtime/bridge/declarative/message.rs"))
            .expect("declarative message bridge should be readable");
    let command =
        fs::read_to_string(manifest_dir.join("src/runtime/bridge/declarative/command.rs"))
            .expect("declarative command bridge should be readable");
    let bridge = fs::read_to_string(manifest_dir.join("src/runtime/bridge.rs"))
        .expect("runtime bridge module should be readable");
    let runtime =
        fs::read_to_string(manifest_dir.join("src/runtime/mod.rs")).expect("runtime module");

    for (source, parts, from_parts, wrapper) in [
        (
            message.as_str(),
            "pub struct DeclarativeRuntimeBridgeParts",
            "pub fn from_parts(parts: DeclarativeRuntimeBridgeParts<State, Project, Reduce>) -> Self",
            "Self::from_parts(DeclarativeRuntimeBridgeParts {",
        ),
        (
            message.as_str(),
            "pub struct DeclarativeOwnedRuntimeBridgeParts",
            "pub fn from_parts(parts: DeclarativeOwnedRuntimeBridgeParts<State, Project, Reduce>) -> Self",
            "Self::from_parts(DeclarativeOwnedRuntimeBridgeParts {",
        ),
        (
            command.as_str(),
            "pub struct DeclarativeCommandRuntimeBridgeParts",
            "pub fn from_parts(parts: DeclarativeCommandRuntimeBridgeParts<State, Project, Update>) -> Self",
            "Self::from_parts(DeclarativeCommandRuntimeBridgeParts {",
        ),
        (
            command.as_str(),
            "pub struct DeclarativeOwnedCommandRuntimeBridgeParts",
            "parts: DeclarativeOwnedCommandRuntimeBridgeParts<State, Project, Update>",
            "Self::from_parts(DeclarativeOwnedCommandRuntimeBridgeParts {",
        ),
    ] {
        assert!(
            source.contains(parts) && source.contains(from_parts) && source.contains(wrapper),
            "declarative runtime bridge should expose named parts and compatibility wrappers for {parts}"
        );
    }
    for export in [
        "DeclarativeRuntimeBridgeParts",
        "DeclarativeOwnedRuntimeBridgeParts",
        "DeclarativeCommandRuntimeBridgeParts",
        "DeclarativeOwnedCommandRuntimeBridgeParts",
    ] {
        assert!(
            bridge.contains(export) && runtime.contains(export),
            "runtime bridge named parts type {export} should stay publicly exported"
        );
    }
}

#[test]
fn scroll_commands_use_named_parts_for_reveal_requests() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let command = fs::read_to_string(manifest_dir.join("src/runtime/command.rs"))
        .expect("runtime command module should be readable");
    let constructors = fs::read_to_string(manifest_dir.join("src/runtime/command/constructors.rs"))
        .expect("runtime command constructors should be readable");
    let scroll_constructors =
        fs::read_to_string(manifest_dir.join("src/runtime/command/constructors/scroll.rs"))
            .expect("runtime command scroll constructors should be readable");
    let update_context =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context.rs"))
            .expect("application update context should be readable");
    let update_context_surface =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/surface.rs"))
            .expect("application update context surface helpers should be readable");
    let runtime =
        fs::read_to_string(manifest_dir.join("src/runtime/mod.rs")).expect("runtime module");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        command.contains("pub struct ScrollIntoViewParts")
            && command.contains("pub struct ScrollFixedRowIntoViewParts"),
        "scroll reveal commands should expose named request parts"
    );
    assert!(
        constructors.contains("mod scroll;")
            && !constructors.contains(
                "pub const fn scroll_into_view_from_parts(parts: ScrollIntoViewParts) -> Self"
            )
            && scroll_constructors.contains(
                "pub const fn scroll_into_view_from_parts(parts: ScrollIntoViewParts) -> Self"
            )
            && scroll_constructors.contains("pub const fn scroll_fixed_row_into_view_from_parts")
            && scroll_constructors
                .contains("Self::scroll_into_view_from_parts(ScrollIntoViewParts {")
            && scroll_constructors.contains(
                "Self::scroll_fixed_row_into_view_from_parts(ScrollFixedRowIntoViewParts {"
            ),
        "scroll command constructors should stay in their focused module and delegate positional helpers through named request parts"
    );
    assert!(
        update_context.contains("mod surface;")
            && update_context_surface.contains(
                "pub fn scroll_into_view_from_parts(&mut self, parts: ScrollIntoViewParts)"
            )
            && update_context_surface.contains("pub fn scroll_fixed_row_into_view_from_parts")
            && runtime.contains("ScrollIntoViewParts")
            && runtime.contains("ScrollFixedRowIntoViewParts")
            && lib.contains("ScrollIntoViewParts")
            && lib.contains("ScrollFixedRowIntoViewParts"),
        "scroll reveal named request parts should be available through runtime and prelude paths"
    );
}

#[test]
fn update_context_keeps_followup_command_groups_in_focused_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/application/runtime/update_context.rs"))
        .expect("application update context root should be readable");
    let commands =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/commands.rs"))
            .expect("application update context command helpers should be readable");
    let platform =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/platform.rs"))
            .expect("application update context platform helpers should be readable");
    let tasks =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/tasks.rs"))
            .expect("application update context task helpers should be readable");
    let surface =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/surface.rs"))
            .expect("application update context surface helpers should be readable");

    for required in [
        "mod commands;",
        "mod platform;",
        "mod surface;",
        "mod tasks;",
        "pub struct UpdateContext<Message>",
        "fn into_command(self) -> Command<Message>",
    ] {
        assert!(
            root.contains(required),
            "update context root should own the queue and delegate `{required}`"
        );
    }
    assert!(
        commands.contains("pub fn request_repaint")
            && commands.contains("pub fn request_paint_only")
            && commands.contains("pub fn repaint")
            && commands.contains("pub fn after")
            && commands.contains("pub fn exit"),
        "basic command and repaint helpers should live in update_context/commands.rs"
    );
    assert!(
        platform.contains("pub fn begin_external_drag")
            && platform.contains("pub fn platform_request")
            && platform.contains("pub fn pick_folder")
            && platform.contains("pub fn confirm"),
        "platform and external-drag helpers should live in update_context/platform.rs"
    );
    assert!(
        tasks.contains("pub fn spawn<Output>")
            && tasks.contains("pub fn spawn_cancellable")
            && tasks.contains("pub fn spawn_latest")
            && tasks.contains("pub fn spawn_resource"),
        "runtime task and resource helpers should live in update_context/tasks.rs"
    );
    assert!(
        surface.contains("pub fn focus")
            && surface.contains("pub fn scroll_to")
            && surface.contains("pub fn scroll_into_view_from_parts")
            && surface.contains("pub fn scroll_fixed_row_into_view_from_parts"),
        "focus and scroll helpers should live in update_context/surface.rs"
    );
}

#[test]
fn application_task_helpers_keep_cancellation_completion_and_latest_state_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/application/runtime/task.rs"))
        .expect("application runtime task root should be readable");
    let cancellation =
        fs::read_to_string(manifest_dir.join("src/application/runtime/task/cancellation.rs"))
            .expect("application runtime cancellation token module should be readable");
    let completion =
        fs::read_to_string(manifest_dir.join("src/application/runtime/task/completion.rs"))
            .expect("application runtime task completion module should be readable");
    let latest = fs::read_to_string(manifest_dir.join("src/application/runtime/task/latest.rs"))
        .expect("application runtime latest task module should be readable");
    let keyed_latest =
        fs::read_to_string(manifest_dir.join("src/application/runtime/task/keyed_latest.rs"))
            .expect("application runtime keyed latest task module should be readable");
    let runtime = fs::read_to_string(manifest_dir.join("src/application/runtime.rs"))
        .expect("application runtime module should be readable");

    for required in [
        "mod cancellation;",
        "mod completion;",
        "mod keyed_latest;",
        "mod latest;",
        "pub use cancellation::CancellationToken;",
        "pub use completion::{KeyedTaskCompletion, TaskCompletion, TaskTicket};",
        "pub use keyed_latest::KeyedLatestTasks;",
        "pub use latest::LatestTask;",
    ] {
        assert!(
            root.contains(required),
            "application runtime task root should delegate `{required}`"
        );
    }
    assert!(
        !root.contains("pub struct CancellationToken")
            && !root.contains("pub struct TaskCompletion")
            && !root.contains("pub struct LatestTask")
            && !root.contains("pub struct KeyedLatestTasks"),
        "application runtime task root should re-export task helpers without owning implementation"
    );
    assert!(
        cancellation.contains("pub struct CancellationToken")
            && cancellation.contains("pub fn cancel(&self)")
            && cancellation.contains("pub fn is_cancelled(&self) -> bool"),
        "task cancellation token should live in application/runtime/task/cancellation.rs"
    );
    assert!(
        completion.contains("pub struct TaskTicket")
            && completion.contains("pub struct TaskCompletion<Output>")
            && completion.contains("pub struct KeyedTaskCompletion<Key, Output>"),
        "task tickets and completion DTOs should live in application/runtime/task/completion.rs"
    );
    assert!(
        latest.contains("pub struct LatestTask")
            && latest.contains("pub fn begin(&mut self) -> TaskTicket")
            && latest.contains("pub fn finish(&mut self, ticket: TaskTicket) -> bool"),
        "single-resource latest task state should live in application/runtime/task/latest.rs"
    );
    assert!(
        keyed_latest.contains("pub struct KeyedLatestTasks<Key>")
            && keyed_latest.contains("pub fn begin(&mut self, key: Key) -> TaskTicket")
            && keyed_latest.contains("pub fn remove(&mut self, key: &Key) -> Option<LatestTask>"),
        "keyed latest task registry should live in application/runtime/task/keyed_latest.rs"
    );
    assert!(
        runtime.contains("CancellationToken")
            && runtime.contains("KeyedLatestTasks")
            && runtime.contains("TaskCompletion")
            && runtime.contains("TaskTicket"),
        "application runtime facade should keep task helpers available through the public runtime path"
    );
}

#[test]
fn controller_commands_keep_outcome_drain_and_dispatch_in_focused_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/runtime/controller/commands.rs"))
        .expect("runtime controller command root should be readable");
    let outcome =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/outcome.rs"))
            .expect("runtime command outcome module should be readable");
    let drain = fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/drain.rs"))
        .expect("runtime command drain module should be readable");
    let dispatch =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/dispatch.rs"))
            .expect("runtime command dispatch module should be readable");

    for required in [
        "mod dispatch;",
        "mod drain;",
        "mod outcome;",
        "pub use outcome::CommandOutcome;",
    ] {
        assert!(
            root.contains(required),
            "runtime controller command root should delegate `{required}`"
        );
    }
    assert!(
        outcome.contains("pub struct CommandOutcome")
            && outcome.contains("fn finish_command_outcome")
            && !root.contains("pub struct CommandOutcome"),
        "command pass result and finalization should live in commands/outcome.rs"
    );
    assert!(
        drain.contains("pub fn drain_runtime_messages")
            && drain.contains("take_runtime_command_batch_into")
            && !root.contains("pub fn drain_runtime_messages"),
        "runtime work draining should live in commands/drain.rs"
    );
    assert!(
        dispatch.contains("fn execute_command_inner")
            && dispatch.contains("Command::PlatformRequest")
            && dispatch.contains("Command::ScrollFixedRowIntoView")
            && !root.contains("fn execute_command_inner"),
        "command execution branches should live in commands/dispatch.rs"
    );
}

#[test]
fn text_input_state_keeps_models_selection_navigation_and_editing_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let model = fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/model.rs"))
        .expect("text input model root should be readable");
    let selection = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/text_input/model/selection.rs"),
    )
    .expect("text input selection model should be readable");
    let navigation = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/text_input/model/navigation.rs"),
    )
    .expect("text input navigation model should be readable");
    let editing =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/model/editing.rs"))
            .expect("text input editing model should be readable");

    for required in ["mod editing;", "mod navigation;", "mod selection;"] {
        assert!(
            model.contains(required),
            "text input model root should delegate `{required}`"
        );
    }
    assert!(
        model.contains("pub struct TextInputProps")
            && model.contains("pub struct TextInputState")
            && model.contains("pub struct TextInputEditResult")
            && model.contains("pub fn from_value")
            && !model.contains("TextEditCommand")
            && !model.contains("WidgetKey"),
        "text input model root should keep public state definitions separate from command handling"
    );
    assert!(
        selection.contains("pub fn selected_text")
            && selection.contains("pub fn selection_range")
            && selection.contains("pub fn has_selection"),
        "text input selection queries should live in model/selection.rs"
    );
    assert!(
        navigation.contains("pub fn set_caret")
            && navigation.contains("fn move_left")
            && navigation.contains("fn move_right"),
        "text input caret movement should live in model/navigation.rs"
    );
    assert!(
        editing.contains("pub fn apply_edit_command")
            && editing.contains("pub fn apply_key")
            && editing.contains("pub fn insert_text")
            && editing.contains("fn delete_selected_text"),
        "text input mutation and edit command handling should live in model/editing.rs"
    );
}

#[test]
fn retained_invalidation_primitives_stay_in_focused_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/invalidation.rs"))
        .expect("invalidation root should be readable");
    let mask = fs::read_to_string(manifest_dir.join("src/gui/invalidation/mask.rs"))
        .expect("invalidation mask module should be readable");
    let retained_mask =
        fs::read_to_string(manifest_dir.join("src/gui/invalidation/retained_mask.rs"))
            .expect("retained mask module should be readable");
    let segment = fs::read_to_string(manifest_dir.join("src/gui/invalidation/segment.rs"))
        .expect("retained segment module should be readable");

    for required in [
        "mod mask;",
        "mod retained_mask;",
        "mod segment;",
        "pub use mask::InvalidationMask;",
        "pub use retained_mask::RetainedSegmentMask;",
    ] {
        assert!(
            root.contains(required),
            "invalidation root should delegate `{required}`"
        );
    }
    assert!(
        root.contains("RetainedSegmentPlan")
            && root.contains("RetainedSegmentRevisions")
            && !root.contains("pub struct InvalidationMask")
            && !root.contains("pub struct RetainedSegmentMask")
            && !root.contains("pub struct RetainedSegmentPlan"),
        "invalidation root should re-export public primitives without owning their implementations"
    );
    assert!(
        mask.contains("pub struct InvalidationMask")
            && mask.contains("pub const fn from_bits")
            && mask.contains("pub fn insert"),
        "raw invalidation bit operations should live in invalidation/mask.rs"
    );
    assert!(
        retained_mask.contains("pub struct RetainedSegmentMask")
            && retained_mask.contains("pub const fn requires_static_rebuild")
            && retained_mask.contains("pub const fn requires_overlay_rebuild"),
        "typed retained segment masks should live in invalidation/retained_mask.rs"
    );
    assert!(
        segment.contains("pub struct RetainedSegmentPlan")
            && segment.contains("pub struct RetainedSegmentRevisions")
            && segment.contains("pub enum RetainedSegmentKind")
            && segment.contains("pub fn bump_revisions"),
        "retained segment metadata, plans, and revisions should live in invalidation/segment.rs"
    );
}

#[test]
fn text_input_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input.rs"))
        .expect("text-input primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/builders.rs"))
            .expect("text-input primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct TextInputWidget")
            && root.contains("impl Widget for TextInputWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "text-input primitive root should own widget behavior and delegate runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn text_input(")
            && builders.contains("pub fn text_input_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "text-input runtime builder helpers should live in text_input/builders.rs"
    );
}

#[test]
fn input_key_identity_and_keypress_state_stay_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input = fs::read_to_string(manifest_dir.join("src/gui/input.rs"))
        .expect("input module should be readable");
    let key = fs::read_to_string(manifest_dir.join("src/gui/input/key.rs"))
        .expect("input key module should be readable");
    let press = fs::read_to_string(manifest_dir.join("src/gui/input/key/press.rs"))
        .expect("input keypress module should be readable");

    assert!(
        input.contains("pub use key::{KeyCode, KeyPress};")
            && key.contains("mod press;")
            && key.contains("pub use press::KeyPress;"),
        "input facade should preserve KeyCode and KeyPress exports through the key module"
    );
    assert!(
        key.contains("pub enum KeyCode")
            && !key.contains("pub struct KeyPress")
            && press.contains("pub struct KeyPress")
            && press.contains("pub const fn with_command")
            && press.contains("fn keypress_constructors_preserve_modifier_state"),
        "key identity should stay in key.rs while modifier-bearing keypress state lives in key/press.rs"
    );
}

#[test]
fn shortcut_primitives_stay_in_resolution_gesture_and_layer_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/shortcuts.rs"))
        .expect("shortcut root should be readable");
    let resolution = fs::read_to_string(manifest_dir.join("src/gui/shortcuts/resolution.rs"))
        .expect("shortcut resolution module should be readable");
    let gesture = fs::read_to_string(manifest_dir.join("src/gui/shortcuts/gesture.rs"))
        .expect("shortcut gesture module should be readable");
    let layer = fs::read_to_string(manifest_dir.join("src/gui/shortcuts/layer.rs"))
        .expect("shortcut layer module should be readable");

    for required in [
        "mod gesture;",
        "mod layer;",
        "mod resolution;",
        "pub use gesture::{ShortcutGesture, ShortcutModifier};",
        "pub use layer::{ShortcutBinding, ShortcutLayer};",
        "pub use resolution::ShortcutResolution;",
    ] {
        assert!(
            root.contains(required),
            "shortcut root should delegate `{required}`"
        );
    }
    assert!(
        !root.contains("pub struct ShortcutResolution")
            && !root.contains("pub struct ShortcutLayer")
            && !root.contains("pub struct ShortcutGesture"),
        "shortcut root should re-export public primitives instead of owning their implementations"
    );
    assert!(
        resolution.contains("pub struct ShortcutResolution")
            && resolution.contains("pub fn unhandled")
            && resolution.contains("pub fn pending_chord"),
        "shortcut result constructors should live in shortcuts/resolution.rs"
    );
    assert!(
        gesture.contains("pub enum ShortcutModifier")
            && gesture.contains("pub struct ShortcutGesture")
            && gesture.contains("impl From<KeyPress> for ShortcutGesture"),
        "shortcut modifier and key matching should live in shortcuts/gesture.rs"
    );
    assert!(
        layer.contains("pub struct ShortcutBinding")
            && layer.contains("pub struct ShortcutLayer")
            && layer.contains("pub fn resolve_or_else"),
        "shortcut binding collections and modal resolution should live in shortcuts/layer.rs"
    );
}

#[test]
fn canvas_gesture_primitives_stay_in_event_pointer_and_state_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/interaction/canvas_gesture.rs"))
        .expect("canvas gesture root should be readable");
    let event =
        fs::read_to_string(manifest_dir.join("src/widgets/interaction/canvas_gesture/event.rs"))
            .expect("canvas gesture event module should be readable");
    let pointer =
        fs::read_to_string(manifest_dir.join("src/widgets/interaction/canvas_gesture/pointer.rs"))
            .expect("canvas gesture pointer module should be readable");
    let state =
        fs::read_to_string(manifest_dir.join("src/widgets/interaction/canvas_gesture/state.rs"))
            .expect("canvas gesture state module should be readable");
    let active_press = fs::read_to_string(
        manifest_dir.join("src/widgets/interaction/canvas_gesture/state/active_press.rs"),
    )
    .expect("canvas gesture active press module should be readable");
    let state_tests = fs::read_to_string(
        manifest_dir.join("src/widgets/interaction/canvas_gesture/state/tests.rs"),
    )
    .expect("canvas gesture state tests should be readable");

    for required in [
        "mod event;",
        "mod pointer;",
        "mod state;",
        "pub use event::CanvasGestureEvent;",
        "pub use pointer::CanvasPointer;",
        "pub use state::CanvasGestureState;",
    ] {
        assert!(
            root.contains(required),
            "canvas gesture root should delegate `{required}`"
        );
    }
    assert!(
        !root.contains("pub enum CanvasGestureEvent")
            && !root.contains("pub struct CanvasPointer")
            && !root.contains("pub struct CanvasGestureState"),
        "canvas gesture root should re-export public primitives instead of owning their implementations"
    );
    assert!(
        event.contains("pub enum CanvasGestureEvent")
            && event.contains("Hover(CanvasPointer)")
            && event.contains("FocusChanged(bool)"),
        "canvas gesture event variants should live in canvas_gesture/event.rs"
    );
    assert!(
        pointer.contains("pub struct CanvasPointer")
            && pointer.contains("fn canvas_pointer")
            && pointer.contains("fn point_delta"),
        "canvas pointer projection and delta helpers should live in canvas_gesture/pointer.rs"
    );
    assert!(
        state.contains("mod active_press;")
            && state.contains("#[cfg(test)]")
            && state.contains("mod tests;")
            && state.contains("pub struct CanvasGestureState")
            && state.contains("pub fn handle_input"),
        "canvas retained state and input resolution should live in canvas_gesture/state.rs"
    );
    assert!(
        !state.contains("struct ActiveCanvasPress")
            && active_press.contains("struct ActiveCanvasPress")
            && active_press.contains("origin: CanvasPointer")
            && active_press.contains("button: PointerButton")
            && active_press.contains("modifiers: PointerModifiers"),
        "canvas retained press metadata should live in canvas_gesture/state/active_press.rs"
    );
    assert!(
        !state.contains("fn canvas_gesture_state_tracks_press_drag_and_release")
            && state_tests
                .contains("fn canvas_gesture_state_projects_local_and_normalized_positions")
            && state_tests.contains("fn canvas_gesture_state_tracks_press_drag_and_release")
            && state_tests.contains("fn canvas_gesture_state_clears_drag_on_focus_loss"),
        "canvas gesture state regression tests should live in canvas_gesture/state/tests.rs"
    );
}

#[test]
fn resource_completions_use_named_parts_for_request_results() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/runtime/resource/load.rs"))
        .expect("resource load module should be readable");
    let resource = fs::read_to_string(manifest_dir.join("src/runtime/resource.rs"))
        .expect("runtime resource module should be readable");
    let runtime =
        fs::read_to_string(manifest_dir.join("src/runtime/mod.rs")).expect("runtime module");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");
    let update_context_tasks =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/tasks.rs"))
            .expect("application update context task helpers should be readable");

    assert!(
        source.contains("pub struct ResourceCompletionParts")
            && source.contains("pub fn from_parts(parts: ResourceCompletionParts<T>) -> Self")
            && source.contains("Self::from_parts(ResourceCompletionParts { request, load })"),
        "resource completions should expose named parts and keep the compatibility constructor"
    );
    assert!(
        source.contains("ResourceCompletion::from_parts(ResourceCompletionParts {")
            && update_context_tasks.contains(
                "ResourceCompletion::from_parts(ResourceCompletionParts { request, load })"
            ),
        "resource completion mapping and spawn helpers should use the named-parts construction path"
    );
    assert!(
        resource.contains("ResourceCompletionParts")
            && runtime.contains("ResourceCompletionParts")
            && lib.contains("ResourceCompletionParts"),
        "resource completion parts should remain publicly exported through runtime and prelude"
    );
}

#[test]
fn resource_slots_keep_load_state_and_lifecycle_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let slot = fs::read_to_string(manifest_dir.join("src/runtime/resource/slot.rs"))
        .expect("resource slot module should be readable");
    let state = fs::read_to_string(manifest_dir.join("src/runtime/resource/slot/state.rs"))
        .expect("resource slot state module should be readable");
    let resource = fs::read_to_string(manifest_dir.join("src/runtime/resource.rs"))
        .expect("runtime resource module should be readable");
    let runtime =
        fs::read_to_string(manifest_dir.join("src/runtime/mod.rs")).expect("runtime module");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        slot.contains("mod state;")
            && slot.contains("pub use state::ResourceLoadState;")
            && slot.contains("pub struct ResourceSlot<T>")
            && slot.contains("pub fn begin_load")
            && slot.contains("pub fn apply_for"),
        "resource slot lifecycle should stay in slot.rs while delegating load-state model"
    );
    assert!(
        !slot.contains("pub enum ResourceLoadState")
            && state.contains("pub enum ResourceLoadState")
            && state.contains("Idle")
            && state.contains("Loading")
            && state.contains("Ready")
            && state.contains("Failed"),
        "resource load-state enum should live in runtime/resource/slot/state.rs"
    );
    assert!(
        resource.contains("pub use slot::{ResourceLoadState, ResourceSlot};")
            && runtime.contains("ResourceLoadState")
            && runtime.contains("ResourceSlot")
            && lib.contains("ResourceLoadState")
            && lib.contains("ResourceSlot"),
        "resource slot and load-state types should remain exported through runtime and prelude"
    );
}

#[test]
fn gpu_surface_widget_uses_named_parts_for_retained_resource_identity() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/widgets/primitives/gpu_surface.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let widgets = fs::read_to_string(manifest_dir.join("src/widgets/mod.rs"))
        .expect("widgets module should be readable");
    let application_builder =
        fs::read_to_string(manifest_dir.join("src/application/builders/leaf.rs"))
            .expect("application leaf builders should be readable");

    assert!(
        source.contains("pub struct GpuSurfaceParts")
            && source.contains("pub fn from_parts(parts: GpuSurfaceParts) -> Self"),
        "retained GPU surfaces should expose named parts for resource identity, revision, and content"
    );
    assert!(
        source.contains("Self::from_parts(GpuSurfaceParts {")
            && widgets.contains("GpuSurfaceParts")
            && application_builder.contains("pub fn gpu_surface_from_parts"),
        "GPU surface compatibility constructors, public exports, and application builders should keep the named-parts path available"
    );
}

#[test]
fn gpu_surface_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/gpu_surface.rs"))
        .expect("gpu-surface primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/gpu_surface/builders.rs"))
            .expect("gpu-surface primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct GpuSurfaceWidget")
            && root.contains("impl Widget for GpuSurfaceWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>"),
        "gpu-surface primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn gpu_surface("),
        "gpu-surface runtime builder helper should live in gpu_surface/builders.rs"
    );
}

#[test]
fn gpu_surface_content_models_stay_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let content = fs::read_to_string(manifest_dir.join("src/runtime/gpu_surface/content.rs"))
        .expect("GPU surface content module should be readable");
    let model = fs::read_to_string(manifest_dir.join("src/runtime/gpu_surface/content/model.rs"))
        .expect("GPU surface content model module should be readable");
    let validation =
        fs::read_to_string(manifest_dir.join("src/runtime/gpu_surface/content/validation.rs"))
            .expect("GPU surface content validation module should be readable");
    let gpu_surface = fs::read_to_string(manifest_dir.join("src/runtime/gpu_surface.rs"))
        .expect("GPU surface runtime facade should be readable");

    assert!(
        content.contains("mod model;")
            && content.contains("pub use model::{GpuSignalGainPreview, GpuSignalRenderShape};")
            && content.contains("pub enum GpuSurfaceContent")
            && !content.contains("pub struct GpuSignalGainPreview")
            && !content.contains("pub struct GpuSignalRenderShape"),
        "GPU surface content root should expose the retained content enum while re-exporting focused signal content models"
    );
    assert!(
        model.contains("pub struct GpuSignalGainPreview")
            && model.contains("pub fade_in_length: f32")
            && model.contains("pub struct GpuSignalRenderShape")
            && model.contains("pub sample_count: usize"),
        "GPU signal gain-preview and render-shape DTOs should live in content/model.rs"
    );
    assert!(
        validation.contains("validate_signal_gain_preview")
            && validation.contains("validate_signal_render_shape"),
        "GPU surface content validation should stay in the validation module"
    );
    assert!(
        gpu_surface.contains("GpuSignalGainPreview")
            && gpu_surface.contains("GpuSignalRenderShape")
            && gpu_surface.contains("GpuSurfaceContent")
            && gpu_surface.contains("GpuSurfaceContentError"),
        "GPU surface content models and diagnostics should remain available through the runtime facade"
    );
}

#[test]
fn property_panel_rows_use_named_parts_for_public_inspector_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/property_panel.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let application = fs::read_to_string(manifest_dir.join("src/application.rs"))
        .expect("application module should be readable");

    assert!(
        source.contains("pub struct PropertyRowParts")
            && source.contains("pub fn from_parts(parts: PropertyRowParts) -> Self"),
        "property panel rows should expose named parts for id, label, and value construction"
    );
    assert!(
        source.contains("Self::from_parts(PropertyRowParts {")
            && application.contains("PropertyRowParts"),
        "property row compatibility constructor and public exports should keep the named-parts path available"
    );
}

#[test]
fn tree_list_items_use_named_parts_for_public_navigation_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/tree_list.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let row = fs::read_to_string(manifest_dir.join("src/application/tree_list/row.rs"))
        .expect("tree-list row view module should be readable");
    let application = fs::read_to_string(manifest_dir.join("src/application.rs"))
        .expect("application module should be readable");

    assert!(
        source.contains("pub struct TreeListItemParts")
            && source.contains("pub fn from_parts(parts: TreeListItemParts) -> Self"),
        "tree-list items should expose named parts for id, depth, and label construction"
    );
    assert!(
        source.contains("Self::from_parts(TreeListItemParts {")
            && source.contains("mod row;")
            && application.contains("TreeListItemParts"),
        "tree-list compatibility constructor and public exports should keep the named-parts path available"
    );
    assert!(
        !source.contains("fn tree_list_row")
            && row.contains("fn tree_list_row")
            && row.contains("drag_handle()")
            && row.contains("tree-list-toggle-")
            && row.contains("WidgetProminence::Subtle"),
        "tree-list private row assembly should live in application/tree_list/row.rs"
    );
}

#[test]
fn application_menus_use_named_parts_for_context_overlay_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/menu.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let application = fs::read_to_string(manifest_dir.join("src/application.rs"))
        .expect("application module should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    for (parts, constructor) in [
        (
            "pub struct MenuItemParts",
            "pub fn from_parts(parts: MenuItemParts<State>) -> Self",
        ),
        (
            "pub struct MenuParts",
            "pub fn menu_from_parts<State: 'static>(parts: MenuParts<State>) -> StateView<State>",
        ),
        (
            "pub struct ContextMenuOverlayParts",
            "pub fn context_menu_overlay_from_parts<State: 'static>",
        ),
    ] {
        assert!(
            source.contains(parts) && source.contains(constructor),
            "application menu APIs should expose named parts for {parts}"
        );
    }
    assert!(
        source.contains("Self::from_parts(MenuItemParts {")
            && source.contains("context_menu_overlay_from_parts(ContextMenuOverlayParts {")
            && application.contains("ContextMenuOverlayParts")
            && application.contains("MenuItemParts")
            && application.contains("MenuParts")
            && lib.contains("ContextMenuOverlayParts")
            && lib.contains("context_menu_overlay_from_parts"),
        "menu compatibility helpers and public exports should keep the named-parts path available"
    );
}

#[test]
fn application_widget_views_use_named_parts_for_custom_widget_mapping() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/application/widget_view.rs"))
        .expect("application widget view module should be readable");
    let application = fs::read_to_string(manifest_dir.join("src/application.rs"))
        .expect("application module should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    for (parts, from_parts, wrapper) in [
        (
            "pub struct MappedWidgetParts",
            "pub fn from_parts(parts: MappedWidgetParts<W, Message>) -> Self",
            "Self::from_parts(MappedWidgetParts { widget, messages })",
        ),
        (
            "pub struct DynamicWidgetParts",
            "pub fn from_parts(parts: DynamicWidgetParts<Message>) -> Self",
            "Self::from_parts(DynamicWidgetParts {",
        ),
    ] {
        assert!(
            source.contains(parts) && source.contains(from_parts) && source.contains(wrapper),
            "application widget views should expose named parts and compatibility wrappers for {parts}"
        );
    }
    assert!(
        application.contains("MappedWidgetParts")
            && application.contains("DynamicWidgetParts")
            && lib.contains("MappedWidgetParts")
            && lib.contains("DynamicWidgetParts"),
        "custom widget view parts should stay publicly exported through application and prelude"
    );
}

#[test]
fn details_list_rows_use_named_parts_for_public_row_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/details_list/model.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let application = fs::read_to_string(manifest_dir.join("src/application.rs"))
        .expect("application module should be readable");
    let module = fs::read_to_string(manifest_dir.join("src/application/details_list.rs"))
        .expect("details-list module should be readable");

    assert!(
        source.contains("pub struct DetailsRowParts")
            && source.contains("pub fn from_parts(parts: DetailsRowParts) -> Self"),
        "details-list rows should expose named parts for id and cell construction"
    );
    assert!(
        source.contains("Self::from_parts(DetailsRowParts {")
            && module.contains("DetailsRowParts")
            && application.contains("DetailsRowParts"),
        "details-row compatibility constructor and public exports should keep the named-parts path available"
    );
}

#[test]
fn details_list_columns_use_named_parts_for_public_column_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/details_list/model.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let application = fs::read_to_string(manifest_dir.join("src/application.rs"))
        .expect("application module should be readable");
    let module = fs::read_to_string(manifest_dir.join("src/application/details_list.rs"))
        .expect("details-list module should be readable");

    assert!(
        source.contains("pub struct DetailsColumnParts")
            && source.contains("pub fn from_parts(parts: DetailsColumnParts) -> Self"),
        "details-list columns should expose named parts for id, label, and width construction"
    );
    assert!(
        source.contains("Self::from_parts(DetailsColumnParts {")
            && module.contains("DetailsColumnParts")
            && application.contains("DetailsColumnParts"),
        "details-column compatibility constructors and public exports should keep the named-parts path available"
    );
}

#[test]
fn details_list_sort_uses_named_parts_for_public_sort_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/application/details_list/model.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let application = fs::read_to_string(manifest_dir.join("src/application.rs"))
        .expect("application module should be readable");
    let module = fs::read_to_string(manifest_dir.join("src/application/details_list.rs"))
        .expect("details-list module should be readable");

    assert!(
        source.contains("pub struct DetailsSortParts")
            && source.contains("pub fn from_parts(parts: DetailsSortParts) -> Self"),
        "details-list sort state should expose named parts for column id and direction construction"
    );
    assert!(
        source.contains("Self::from_parts(DetailsSortParts {")
            && module.contains("DetailsSortParts")
            && application.contains("DetailsSortParts"),
        "details-sort compatibility constructor and public exports should keep the named-parts path available"
    );
}

#[test]
fn confirm_dialogs_use_named_parts_for_public_prompt_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/runtime/platform.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let runtime = fs::read_to_string(manifest_dir.join("src/runtime/mod.rs"))
        .expect("runtime module should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        source.contains("pub struct ConfirmDialogParts")
            && source.contains("pub fn from_parts(parts: ConfirmDialogParts) -> Self"),
        "confirmation dialogs should expose named parts for title, message, level, and buttons"
    );
    assert!(
        source.contains("Self::from_parts(ConfirmDialogParts {")
            && runtime.contains("ConfirmDialogParts")
            && lib.contains("ConfirmDialogParts"),
        "confirmation dialog compatibility constructor and public exports should keep the named-parts path available"
    );
}

#[test]
fn widget_sizing_uses_named_parts_for_intrinsic_bounds() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/widgets/contract/sizing.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let contract = fs::read_to_string(manifest_dir.join("src/widgets/contract.rs"))
        .expect("widget contract module should be readable");
    let widgets = fs::read_to_string(manifest_dir.join("src/widgets/mod.rs"))
        .expect("widgets module should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        source.contains("pub struct WidgetSizingParts")
            && source.contains("pub fn from_parts(parts: WidgetSizingParts) -> Self"),
        "widget sizing should expose named parts for minimum, preferred, and baseline values"
    );
    assert!(
        source.contains("Self::from_parts(WidgetSizingParts {")
            && contract.contains("WidgetSizingParts")
            && widgets.contains("WidgetSizingParts")
            && lib.contains("WidgetSizingParts"),
        "widget sizing compatibility constructor and public exports should keep the named-parts path available"
    );
}

#[test]
fn labeled_primitive_widgets_use_named_parts_for_identity_content_and_sizing() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let primitives_dir = manifest_dir.join("src/widgets/primitives");
    let widgets = fs::read_to_string(manifest_dir.join("src/widgets/mod.rs"))
        .expect("widgets module should be readable");

    for (file, parts, from_parts, wrapper) in [
        (
            "button.rs",
            "pub struct ButtonWidgetParts",
            "pub fn from_parts(parts: ButtonWidgetParts) -> Self",
            "Self::from_parts(ButtonWidgetParts {",
        ),
        (
            "toggle.rs",
            "pub struct ToggleWidgetParts",
            "pub fn from_parts(parts: ToggleWidgetParts) -> Self",
            "Self::from_parts(ToggleWidgetParts {",
        ),
        (
            "text.rs",
            "pub struct TextWidgetParts",
            "pub fn from_parts(parts: TextWidgetParts) -> Self",
            "Self::from_parts(TextWidgetParts {",
        ),
        (
            "badge.rs",
            "pub struct BadgeWidgetParts",
            "pub fn from_parts(parts: BadgeWidgetParts) -> Self",
            "Self::from_parts(BadgeWidgetParts {",
        ),
        (
            "list_item.rs",
            "pub struct ListItemWidgetParts",
            "pub fn from_parts(parts: ListItemWidgetParts) -> Self",
            "Self::from_parts(ListItemWidgetParts {",
        ),
        (
            "selectable.rs",
            "pub struct SelectableWidgetParts",
            "pub fn from_parts(parts: SelectableWidgetParts) -> Self",
            "Self::from_parts(SelectableWidgetParts {",
        ),
        (
            "text_input.rs",
            "pub struct TextInputWidgetParts",
            "pub fn from_parts(parts: TextInputWidgetParts) -> Self",
            "Self::from_parts(TextInputWidgetParts {",
        ),
        (
            "slider.rs",
            "pub struct SliderWidgetParts",
            "pub fn from_parts(parts: SliderWidgetParts) -> Self",
            "Self::from_parts(SliderWidgetParts {",
        ),
        (
            "scrollbar.rs",
            "pub struct ScrollbarWidgetParts",
            "pub fn from_parts(parts: ScrollbarWidgetParts) -> Self",
            "Self::from_parts(ScrollbarWidgetParts {",
        ),
        (
            "icon_button.rs",
            "pub struct IconButtonWidgetParts",
            "pub fn from_parts(parts: IconButtonWidgetParts) -> Self",
            "Self::from_parts(IconButtonWidgetParts {",
        ),
        (
            "image.rs",
            "pub struct ImageWidgetParts",
            "pub fn from_parts(parts: ImageWidgetParts) -> Self",
            "Self::from_parts(ImageWidgetParts {",
        ),
        (
            "canvas.rs",
            "pub struct CanvasWidgetParts",
            "pub fn from_parts(parts: CanvasWidgetParts) -> Self",
            "Self::from_parts(CanvasWidgetParts {",
        ),
        (
            "card.rs",
            "pub struct CardWidgetParts",
            "pub fn from_parts(parts: CardWidgetParts) -> Self",
            "Self::from_parts(CardWidgetParts {",
        ),
        (
            "drag_handle.rs",
            "pub struct DragHandleWidgetParts",
            "pub fn from_parts(parts: DragHandleWidgetParts) -> Self",
            "Self::from_parts(DragHandleWidgetParts {",
        ),
        (
            "interactive_row.rs",
            "pub struct InteractiveRowWidgetParts",
            "pub fn from_parts(parts: InteractiveRowWidgetParts) -> Self",
            "Self::from_parts(InteractiveRowWidgetParts {",
        ),
    ] {
        let source_path = primitives_dir.join(file);
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
        assert!(
            source.contains(parts) && source.contains(from_parts) && source.contains(wrapper),
            "labeled primitive widget in {file} should expose named parts and compatibility constructor"
        );
        assert!(
            widgets.contains(parts.trim_start_matches("pub struct ")),
            "widgets module should export {parts}"
        );
    }
}

#[test]
fn scrollbar_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/scrollbar.rs"))
        .expect("scrollbar primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/scrollbar/builders.rs"))
            .expect("scrollbar primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct ScrollbarWidget")
            && root.contains("impl Widget for ScrollbarWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "scrollbar primitive root should own widget behavior and delegate runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn scrollbar(")
            && builders.contains("pub fn scrollbar_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "scrollbar runtime builder helpers should live in scrollbar/builders.rs"
    );
}

#[test]
fn drag_handle_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/drag_handle.rs"))
        .expect("drag-handle primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/drag_handle/builders.rs"))
            .expect("drag-handle primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct DragHandleWidget")
            && root.contains("impl Widget for DragHandleWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "drag-handle primitive root should own widget behavior and delegate runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn drag_handle_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>")
            && builders.contains("pub fn drag_handle("),
        "drag-handle runtime builder helpers should live in drag_handle/builders.rs"
    );
}

#[test]
fn slider_primitive_keeps_surface_builders_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/slider.rs"))
        .expect("slider primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/slider/builders.rs"))
            .expect("slider primitive builders should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/widgets/primitives/slider/tests.rs"))
        .expect("slider primitive tests should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct SliderWidget")
            && root.contains("impl Widget for SliderWidget")
            && root.contains("#[path = \"slider/tests.rs\"]")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>")
            && !root.contains("fn slider_pointer_drag_emits_clamped_values"),
        "slider primitive root should own widget behavior while delegating runtime builders and behavior tests"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn slider(")
            && builders.contains("pub fn slider_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "slider runtime builder helpers should live in slider/builders.rs"
    );
    assert!(
        tests.contains("fn slider_pointer_drag_emits_clamped_values")
            && tests.contains("fn focused_slider_responds_to_keyboard_steps"),
        "slider behavior tests should live in slider/tests.rs"
    );
}

#[test]
fn toggle_primitive_keeps_surface_builders_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/toggle.rs"))
        .expect("toggle primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/toggle/builders.rs"))
            .expect("toggle primitive builders should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/widgets/primitives/toggle/tests.rs"))
        .expect("toggle primitive tests should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct ToggleWidget")
            && root.contains("impl Widget for ToggleWidget")
            && root.contains("#[path = \"toggle/tests.rs\"]")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>")
            && !root.contains("fn toggle_keyboard_activation_flips_active_state"),
        "toggle primitive root should own widget behavior while delegating runtime builders and behavior tests"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn toggle(")
            && builders.contains("pub fn toggle_with_checked(")
            && builders.contains("pub fn toggle_mapped(")
            && builders.contains("pub fn toggle_mapped_with_checked(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "toggle runtime builder helpers should live in toggle/builders.rs"
    );
    assert!(
        tests.contains("fn toggle_keyboard_activation_flips_active_state"),
        "toggle behavior tests should live in toggle/tests.rs"
    );
}

#[test]
fn badge_primitive_keeps_surface_builders_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/badge.rs"))
        .expect("badge primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/badge/builders.rs"))
            .expect("badge primitive builders should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/widgets/primitives/badge/tests.rs"))
        .expect("badge primitive tests should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct BadgeWidget")
            && root.contains("impl Widget for BadgeWidget")
            && root.contains("#[path = \"badge/tests.rs\"]")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>")
            && !root.contains("fn badge_releases_inside_bounds_emit_activation"),
        "badge primitive root should own widget behavior while delegating runtime builders and behavior tests"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn badge(")
            && builders.contains("pub fn badge_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "badge runtime builder helpers should live in badge/builders.rs"
    );
    assert!(
        tests.contains("fn badge_releases_inside_bounds_emit_activation")
            && tests.contains("fn focused_badge_enter_emits_activation"),
        "badge behavior tests should live in badge/tests.rs"
    );
}

#[test]
fn button_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/button.rs"))
        .expect("button primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/button/builders.rs"))
            .expect("button primitive builders should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/widgets/primitives/button/tests.rs"))
        .expect("button primitive tests should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct ButtonWidget")
            && root.contains("impl Widget for ButtonWidget")
            && root.contains("mod tests;")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>")
            && !root.contains("fn button_releases_inside_bounds_emit_activation"),
        "button primitive root should own widget behavior while delegating runtime builders and behavior tests"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn button(")
            && builders.contains("pub fn button_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "button runtime builder helpers should live in button/builders.rs"
    );
    assert!(
        tests.contains("fn button_releases_inside_bounds_emit_activation")
            && tests.contains("fn focused_button_space_emits_activation"),
        "button behavior tests should stay in button/tests.rs"
    );
}

#[test]
fn icon_button_primitive_keeps_surface_mappers_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/icon_button.rs"))
        .expect("icon-button primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/icon_button/builders.rs"))
            .expect("icon-button primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct IconButtonWidget")
            && root.contains("impl Widget for IconButtonWidget")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "icon-button primitive root should own widget behavior and delegate runtime mappers"
    );
    assert!(
        builders.contains("impl<Message> WidgetMessageMapper<Message>")
            && builders.contains("pub fn icon_button("),
        "icon-button runtime mapper helper should live in icon_button/builders.rs"
    );
}

#[test]
fn interactive_row_primitive_keeps_surface_mappers_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/interactive_row.rs"))
        .expect("interactive-row primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/interactive_row/builders.rs"))
            .expect("interactive-row primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct InteractiveRowWidget")
            && root.contains("impl Widget for InteractiveRowWidget")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "interactive-row primitive root should own widget behavior and delegate runtime mappers"
    );
    assert!(
        builders.contains("impl<Message> WidgetMessageMapper<Message>")
            && builders.contains("pub fn interactive_row("),
        "interactive-row runtime mapper helper should live in interactive_row/builders.rs"
    );
}

#[test]
fn list_item_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/list_item.rs"))
        .expect("list item primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/list_item/builders.rs"))
            .expect("list item primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct ListItemWidget")
            && root.contains("impl Widget for ListItemWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "list item primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn list_item(")
            && builders.contains("pub fn list_item_action(")
            && builders.contains("pub fn list_item_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "list item runtime builder helpers should live in list_item/builders.rs"
    );
}

#[test]
fn selectable_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/selectable.rs"))
        .expect("selectable primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/selectable/builders.rs"))
            .expect("selectable primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct SelectableWidget")
            && root.contains("impl Widget for SelectableWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "selectable primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn selectable(")
            && builders.contains("pub fn selectable_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "selectable runtime builder helpers should live in selectable/builders.rs"
    );
}

#[test]
fn canvas_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/canvas.rs"))
        .expect("canvas primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/canvas/builders.rs"))
            .expect("canvas primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct CanvasWidget")
            && root.contains("pub struct RetainedSurfaceDescriptor")
            && root.contains("impl Widget for CanvasWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "canvas primitive root should own widget behavior and retained-surface contract while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn canvas(")
            && builders.contains("pub fn canvas_mapped(")
            && builders.contains("pub fn retained_canvas_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "canvas runtime builder helpers should live in canvas/builders.rs"
    );
}

#[test]
fn card_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/card.rs"))
        .expect("card primitive root should be readable");
    let builders = fs::read_to_string(manifest_dir.join("src/widgets/primitives/card/builders.rs"))
        .expect("card primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct CardWidget")
            && root.contains("impl Widget for CardWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>"),
        "card primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn card("),
        "card runtime builder helper should live in card/builders.rs"
    );
}

#[test]
fn image_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/image.rs"))
        .expect("image primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/image/builders.rs"))
            .expect("image primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct ImageWidget")
            && root.contains("impl Widget for ImageWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>"),
        "image primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn image("),
        "image runtime builder helper should live in image/builders.rs"
    );
}

#[test]
fn text_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/text.rs"))
        .expect("text primitive root should be readable");
    let builders = fs::read_to_string(manifest_dir.join("src/widgets/primitives/text/builders.rs"))
        .expect("text primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct TextWidget")
            && root.contains("impl Widget for TextWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>"),
        "text primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn text("),
        "text runtime builder helper should live in text/builders.rs"
    );
}

#[test]
fn status_line_entries_use_named_parts_for_source_and_message() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/feedback/status/line.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let status = fs::read_to_string(manifest_dir.join("src/gui/feedback/status.rs"))
        .expect("feedback status module should be readable");
    let recovery = fs::read_to_string(manifest_dir.join("src/gui/feedback/status/recovery.rs"))
        .expect("feedback recovery status module should be readable");
    let health = fs::read_to_string(manifest_dir.join("src/gui/feedback/status/health.rs"))
        .expect("feedback health status module should be readable");
    let drag_overlay =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/status/drag_overlay.rs"))
            .expect("feedback drag-overlay status module should be readable");
    let update = fs::read_to_string(manifest_dir.join("src/gui/feedback/status/update.rs"))
        .expect("feedback update status module should be readable");
    let prompt = fs::read_to_string(manifest_dir.join("src/gui/feedback/status/prompt.rs"))
        .expect("feedback prompt status module should be readable");
    let feedback = fs::read_to_string(manifest_dir.join("src/gui/feedback.rs"))
        .expect("feedback module should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        source.contains("pub struct StatusLineEntryParts")
            && source.contains("pub fn from_parts(parts: StatusLineEntryParts) -> Self"),
        "status-line entries should expose named parts for source and message text"
    );
    assert!(
        source.contains("Self::from_parts(StatusLineEntryParts {")
            && status.contains("StatusLineEntryParts")
            && feedback.contains("StatusLineEntryParts")
            && lib.contains("StatusLineEntryParts"),
        "status-line entry compatibility constructor and public exports should keep the named-parts path available"
    );
    for required in [
        "mod drag_overlay;",
        "mod health;",
        "mod prompt;",
        "mod recovery;",
        "mod update;",
        "pub use drag_overlay::DragOverlay;",
        "pub use health::HealthState;",
        "pub use prompt::{ConfirmPrompt, PromptIntent};",
        "pub use recovery::RecoverySummary;",
        "pub use update::{UpdatePanel, UpdateStatus};",
    ] {
        assert!(
            status.contains(required),
            "feedback status facade should delegate `{required}`"
        );
    }
    assert!(
        !status.contains("pub struct RecoverySummary")
            && !status.contains("pub enum HealthState")
            && !status.contains("pub struct DragOverlay")
            && !status.contains("pub struct UpdatePanel")
            && !status.contains("pub struct ConfirmPrompt"),
        "feedback status root should re-export focused models instead of owning them"
    );
    assert!(
        recovery.contains("pub struct RecoverySummary")
            && health.contains("pub enum HealthState")
            && drag_overlay.contains("pub struct DragOverlay")
            && update.contains("pub enum UpdateStatus")
            && update.contains("pub struct UpdatePanel")
            && prompt.contains("pub enum PromptIntent")
            && prompt.contains("pub struct ConfirmPrompt"),
        "feedback status models should live in their focused status child modules"
    );
}

#[test]
fn inline_feedback_indicators_keep_model_geometry_and_sanitizers_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/feedback/inline.rs"))
        .expect("inline feedback root should be readable");
    let model = fs::read_to_string(manifest_dir.join("src/gui/feedback/inline/model.rs"))
        .expect("inline feedback model should be readable");
    let geometry = fs::read_to_string(manifest_dir.join("src/gui/feedback/inline/geometry.rs"))
        .expect("inline feedback geometry should be readable");
    let sanitize = fs::read_to_string(manifest_dir.join("src/gui/feedback/inline/sanitize.rs"))
        .expect("inline feedback sanitizers should be readable");
    let feedback = fs::read_to_string(manifest_dir.join("src/gui/feedback.rs"))
        .expect("feedback facade should be readable");

    assert!(
        root.contains("mod geometry;")
            && root.contains("mod model;")
            && root.contains("mod sanitize;")
            && root.contains("pub use geometry::{inline_indicator_layout")
            && root.contains("pub use model::{InlineIndicatorAnchor")
            && !root.contains("pub struct InlineIndicatorMetrics")
            && !root.contains("fn finite_nonnegative"),
        "inline feedback root should re-export focused model and geometry modules"
    );
    assert!(
        model.contains("pub struct InlineIndicatorMetrics")
            && model.contains("pub struct InlineIndicatorAnchor")
            && model.contains("pub struct InlineIndicatorLayout"),
        "inline feedback public DTOs should live in inline/model.rs"
    );
    assert!(
        geometry.contains("pub fn inline_indicator_reserved_width")
            && geometry.contains("pub fn inline_indicator_layout")
            && geometry.contains("finite_nonnegative")
            && geometry.contains("finite_or")
            && !geometry.contains("fn finite_nonnegative")
            && !geometry.contains("pub struct InlineIndicatorLayout"),
        "inline feedback geometry should consume model DTOs and sanitizer helpers"
    );
    assert!(
        sanitize.contains("fn finite_nonnegative") && sanitize.contains("fn finite_or"),
        "inline feedback numeric sanitizers should live in inline/sanitize.rs"
    );
    assert!(
        feedback.contains("InlineIndicatorMetrics") && feedback.contains("inline_indicator_layout"),
        "inline feedback public API should remain available through the feedback facade"
    );
}

#[test]
fn progress_feedback_keeps_overlay_state_and_track_geometry_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/feedback/progress.rs"))
        .expect("progress feedback root should be readable");
    let overlay = fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/overlay.rs"))
        .expect("progress overlay module should be readable");
    let track = fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/track.rs"))
        .expect("progress track module should be readable");
    let progress_track =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/track/progress.rs"))
            .expect("progress track geometry module should be readable");
    let meter_track =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/track/meter.rs"))
            .expect("progress meter geometry module should be readable");
    let sanitize =
        fs::read_to_string(manifest_dir.join("src/gui/feedback/progress/track/sanitize.rs"))
            .expect("progress track sanitizer module should be readable");
    let feedback = fs::read_to_string(manifest_dir.join("src/gui/feedback.rs"))
        .expect("feedback module should be readable");

    for required in [
        "mod overlay;",
        "mod track;",
        "pub use overlay::ProgressOverlay;",
        "pub use track::{",
    ] {
        assert!(
            root.contains(required),
            "progress feedback root should delegate `{required}`"
        );
    }
    assert!(
        !root.contains("pub struct ProgressOverlay")
            && !root.contains("fn horizontal_progress_fill_rect"),
        "progress feedback root should re-export public primitives without owning implementation"
    );
    assert!(
        overlay.contains("pub struct ProgressOverlay")
            && overlay.contains("pub visible: bool")
            && overlay.contains("pub cancel_requested: bool"),
        "progress overlay state should live in progress/overlay.rs"
    );
    assert!(
        track.contains("mod meter;")
            && track.contains("mod progress;")
            && track.contains("mod sanitize;")
            && track.contains(
                "pub use meter::{horizontal_discrete_meter_fill_rect, horizontal_meter_fill_rect};"
            )
            && track.contains("pub use progress::{")
            && !track.contains("pub fn horizontal_progress_fill_rect")
            && !track.contains("pub fn horizontal_meter_fill_rect"),
        "progress track root should re-export focused geometry modules without owning implementation"
    );
    assert!(
        progress_track.contains("pub fn horizontal_progress_fill_rect")
            && progress_track.contains("pub fn horizontal_progress_activity_rect")
            && progress_track.contains("pub fn horizontal_progress_track_rect"),
        "progress track fill and activity geometry should live in progress/track/progress.rs"
    );
    assert!(
        meter_track.contains("pub fn horizontal_meter_fill_rect")
            && meter_track.contains("pub fn horizontal_discrete_meter_fill_rect"),
        "progress meter geometry should live in progress/track/meter.rs"
    );
    assert!(
        sanitize.contains("fn normalized_fraction") && sanitize.contains("fn finite_nonnegative"),
        "progress track geometry sanitizers should live in progress/track/sanitize.rs"
    );
    assert!(
        feedback.contains("ProgressOverlay")
            && feedback.contains("horizontal_progress_fill_rect")
            && feedback.contains("horizontal_meter_fill_rect"),
        "feedback facade should continue exporting progress overlay and track helpers"
    );
}

#[test]
fn window_specs_use_named_parts_for_manifest_identity_and_options() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let spec_path = manifest_dir.join("src/gui_runtime/window_manifest/spec.rs");
    let builder_path = manifest_dir.join("src/gui_runtime/window_manifest/spec/builders.rs");
    let spec = fs::read_to_string(&spec_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", spec_path.display()));
    let builders = fs::read_to_string(&builder_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", builder_path.display()));
    let manifest = fs::read_to_string(manifest_dir.join("src/gui_runtime/window_manifest.rs"))
        .expect("window manifest module should be readable");
    let runtime = fs::read_to_string(manifest_dir.join("src/runtime/mod.rs"))
        .expect("runtime module should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        spec.contains("pub struct WindowSpecParts")
            && builders.contains("pub fn from_parts(parts: WindowSpecParts) -> Self"),
        "window specs should expose named parts for stable key and native options"
    );
    assert!(
        builders.contains("Self::from_parts(WindowSpecParts {")
            && manifest.contains("WindowSpecParts")
            && runtime.contains("WindowSpecParts")
            && lib.contains("WindowSpecParts"),
        "window spec compatibility constructors and public exports should keep the named-parts path available"
    );
}

#[test]
fn status_segments_use_named_parts_for_chrome_slots() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/chrome.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        source.contains("pub struct StatusSegmentsParts")
            && source.contains("pub fn from_parts(parts: StatusSegmentsParts) -> Self"),
        "status segments should expose named parts for left, center, and right chrome slots"
    );
    assert!(
        source.contains("Self::from_parts(StatusSegmentsParts {")
            && lib.contains("StatusSegmentsParts"),
        "status segment compatibility constructor and prelude export should keep the named-parts path available"
    );
}

#[test]
fn timeline_visualization_state_uses_named_parts_for_large_projection_buckets() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let timeline_dir = manifest_dir.join("src/gui/visualization/timeline");
    let mut violations = Vec::new();

    for path in [
        timeline_dir.join("edit.rs"),
        timeline_dir.join("surface.rs"),
    ] {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        if source.contains("too_many_arguments") {
            violations.push(relative_path(&manifest_dir, &path));
        }
    }

    assert!(
        violations.is_empty(),
        "timeline visualization state should use named projection parts instead of suppressing long positional constructors:\n{}",
        violations.join("\n")
    );
}

#[test]
fn layout_virtual_cache_invalidation_stays_with_cache_types() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let engine = fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/mod.rs"))
        .expect("layout engine module should be readable");
    let cache = fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/cache.rs"))
        .expect("layout cache module should be readable");

    assert!(
        engine.contains("invalidate_virtual_cache_for(&mut self.virtual_cache, node_id)")
            && engine.contains("invalidate_virtual_cache_for_any(&mut self.virtual_cache"),
        "layout engine should delegate virtual-cache invalidation to the cache module"
    );
    assert!(
        !engine.contains("fn invalidate_virtual_cache_for_any")
            && !engine.contains("entry.dependencies.iter()"),
        "layout engine orchestration should not own cached virtual-metric dependency scans"
    );
    assert!(
        cache.contains("fn invalidate_virtual_cache_for(")
            && cache.contains("fn invalidate_virtual_cache_for_any(")
            && cache.contains("impl CachedVirtualMetrics")
            && cache.contains("fn depends_on"),
        "virtual cache dependency invalidation should live with cached metric types"
    );
}

#[test]
fn layout_scroll_virtual_window_search_stays_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let helpers = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/engine/layout/scroll_helpers.rs"),
    )
    .expect("layout scroll helpers should be readable");
    let window = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/engine/layout/scroll_helpers/window.rs"),
    )
    .expect("layout scroll virtual window helper should be readable");
    let virtualization = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/engine/layout/scroll/virtualization.rs"),
    )
    .expect("layout scroll virtualization module should be readable");

    assert!(
        helpers.contains("mod window;")
            && helpers.contains("pub(super) use window::compute_virtual_window;")
            && virtualization.contains("compute_virtual_window"),
        "scroll virtualization should consume focused virtual-window search helpers through scroll_helpers"
    );
    assert!(
        !helpers.contains("fn lower_bound_end")
            && !helpers.contains("fn lower_bound_start")
            && window.contains("fn compute_virtual_window")
            && window.contains("fn lower_bound_end")
            && window.contains("fn lower_bound_start"),
        "virtual-window binary search bounds should live in scroll_helpers/window.rs"
    );
}

#[test]
fn surface_paint_plan_buffering_stays_with_capacity_policy() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let projection = fs::read_to_string(manifest_dir.join("src/runtime/surface/projection.rs"))
        .expect("surface paint projection should be readable");
    let frame = fs::read_to_string(manifest_dir.join("src/runtime/controller/context/frame.rs"))
        .expect("runtime frame paint projection should be readable");
    let capacity = fs::read_to_string(manifest_dir.join("src/runtime/surface/paint/capacity.rs"))
        .expect("surface paint capacity policy should be readable");

    assert!(
        capacity.contains("fn empty_paint_plan_for_layout")
            && capacity.contains("fn clear_paint_plan_for_layout")
            && capacity.contains("fn estimated_paint_primitive_capacity"),
        "layout-aware paint-plan buffer lifecycle should live with the capacity policy"
    );
    assert!(
        projection.contains("empty_paint_plan_for_layout(layout, theme)")
            && projection.contains("clear_paint_plan_for_layout(plan, layout, theme)")
            && frame.contains("empty_paint_plan_for_layout(&self.layout, theme)"),
        "surface and runtime paint projection should consume layout-aware plan helpers"
    );
    assert!(
        !projection.contains("estimated_paint_primitive_capacity")
            && !frame.contains("estimated_paint_primitive_capacity")
            && !projection.contains("SurfacePaintPlan::empty_with_capacity")
            && !frame.contains("SurfacePaintPlan::empty_with_capacity"),
        "paint projection callers should not duplicate capacity-policy mechanics"
    );
}

#[test]
fn surface_layout_projection_records_traversal_through_index_methods() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let layout = fs::read_to_string(manifest_dir.join("src/runtime/surface/layout.rs"))
        .expect("surface layout projection should be readable");
    let index = fs::read_to_string(manifest_dir.join("src/runtime/surface/traversal/index.rs"))
        .expect("surface traversal index should be readable");
    let records =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/traversal/index/records.rs"))
            .expect("surface traversal records should be readable");
    let recording =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/traversal/index/recording.rs"))
            .expect("surface traversal recording helpers should be readable");
    let capacity =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/traversal/index/capacity.rs"))
            .expect("surface traversal capacity helpers should be readable");

    assert!(
        index.contains("mod capacity;")
            && index.contains("mod recording;")
            && index.contains("mod records;")
            && index.contains("pub(in crate::runtime) use records::{")
            && !index.contains("fn record_container")
            && !index.contains("fn record_widget"),
        "surface traversal index root should delegate traversal bucket mutation helpers"
    );
    assert!(
        records.contains("struct SurfaceContainerTraversalRecord")
            && records.contains("struct SurfaceWidgetTraversalRecord")
            && !index.contains("struct SurfaceContainerTraversalRecord")
            && !index.contains("struct SurfaceWidgetTraversalRecord"),
        "surface traversal record DTOs should live in traversal/index/records.rs"
    );
    assert!(
        capacity.contains("fn widget_clip_capacity")
            && capacity.contains("fn reserve_vec_capacity")
            && capacity.contains("fn reserve_map_capacity")
            && capacity.contains("fn reserve_set_capacity")
            && !index.contains("fn reserve_vec_capacity")
            && !index.contains("fn reserve_map_capacity")
            && !index.contains("fn reserve_set_capacity"),
        "surface traversal capacity and reuse helpers should live in traversal/index/capacity.rs"
    );
    assert!(
        recording.contains("fn record_container")
            && recording.contains("fn record_widget")
            && recording.contains(".widget_paint_order.push")
            && recording.contains(".scroll_content_by_container.insert")
            && recording.contains(".container_hover_suppression.insert"),
        "surface traversal bucket recording should live in traversal/index/recording.rs"
    );
    assert!(
        layout.contains("traversal.record_container(SurfaceContainerTraversalRecord")
            && layout.contains("traversal.record_widget(SurfaceWidgetTraversalRecord"),
        "surface layout projection should describe traversal records instead of mutating buckets directly"
    );
    for forbidden in [
        ".widget_paint_order.push",
        ".widget_paths",
        ".focusable_widget_order.push",
        ".keyboard_focus_order.push",
        ".pointer_hit_order.push",
        ".wheel_hit_order.push",
        ".stateful_widget_order.push",
        ".container_hover_suppression",
        ".widget_clip_ancestors",
        ".container_clip_ancestors",
        ".scroll_container_order.push",
        ".scroll_content_by_container",
        ".styled_container_order.push",
    ] {
        assert!(
            !layout.contains(forbidden),
            "surface layout projection should not directly mutate traversal bucket `{forbidden}`"
        );
    }
}

#[test]
fn runtime_layout_refresh_delegates_traversal_state_handoff() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let layout = fs::read_to_string(manifest_dir.join("src/runtime/controller/state/layout.rs"))
        .expect("runtime layout state should be readable");
    let traversal =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/state/traversal.rs"))
            .expect("runtime traversal state should be readable");

    assert!(
        layout.contains("self.install_traversal_index(traversal)")
            && layout.contains("self.refresh_visible_traversal_orders()"),
        "runtime layout refresh should delegate traversal bucket installation and visible-order refresh"
    );
    assert!(
        traversal.contains("fn install_traversal_index")
            && traversal.contains("fn take_reusable_traversal_index")
            && traversal.contains("fn refresh_visible_traversal_orders"),
        "runtime traversal state handoff should live in a focused helper module"
    );
    for forbidden in [
        "self.widget_hit_order = traversal.",
        "self.widget_paths = traversal.",
        "set_order(traversal.",
        "self.container_hover_suppression = traversal.",
        "self.stateful_widget_order = traversal.",
        "self.widget_clip_ancestors = traversal.",
        "self.container_clip_ancestors = traversal.",
        "self.scroll_content_by_container = traversal.",
    ] {
        assert!(
            !layout.contains(forbidden),
            "runtime layout refresh should not directly install traversal bucket `{forbidden}`"
        );
    }
}
