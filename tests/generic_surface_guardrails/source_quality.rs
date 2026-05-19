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

    assert!(
        source.contains("pub struct SignalChromeParts")
            && source.contains("pub fn from_parts(parts: SignalChromeParts) -> Self"),
        "signal chrome state should expose named parts for readable public construction"
    );
    assert!(
        source.contains("pub struct SignalRasterPreviewParts")
            && source.contains("pub fn from_parts(parts: SignalRasterPreviewParts) -> Self"),
        "signal raster preview state should expose named parts for readable public construction"
    );
    assert!(
        source.contains("Self::from_parts(SignalChromeParts {")
            && source.contains("Self::from_parts(SignalRasterPreviewParts {"),
        "signal compatibility constructors should delegate through named parts objects"
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
fn inline_badge_metrics_use_named_parts_for_geometry_tokens() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/badge/inline.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct InlineBadgeMetricsParts")
            && source.contains("pub fn from_parts(parts: InlineBadgeMetricsParts) -> Self"),
        "inline badge metrics should expose named parts for readable public construction"
    );
    assert!(
        source.contains("Self::from_parts(InlineBadgeMetricsParts {"),
        "the positional compatibility constructor should delegate through the named metrics object"
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
    let application = fs::read_to_string(manifest_dir.join("src/application.rs"))
        .expect("application module should be readable");

    assert!(
        source.contains("pub struct TreeListItemParts")
            && source.contains("pub fn from_parts(parts: TreeListItemParts) -> Self"),
        "tree-list items should expose named parts for id, depth, and label construction"
    );
    assert!(
        source.contains("Self::from_parts(TreeListItemParts {")
            && application.contains("TreeListItemParts"),
        "tree-list compatibility constructor and public exports should keep the named-parts path available"
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

    assert!(
        index.contains("struct SurfaceContainerTraversalRecord")
            && index.contains("struct SurfaceWidgetTraversalRecord")
            && index.contains("fn record_container")
            && index.contains("fn record_widget"),
        "surface traversal index should own traversal bucket mutation helpers"
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
