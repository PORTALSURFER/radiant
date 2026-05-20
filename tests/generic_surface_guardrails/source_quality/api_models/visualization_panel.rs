use super::*;

#[test]
fn visualization_behavior_tests_stay_grouped_by_surface_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/visualization/tests.rs"))
        .expect("visualization test root should be readable");
    let spatial = fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/spatial.rs"))
        .expect("spatial visualization tests should be readable");
    let canvas = fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/canvas.rs"))
        .expect("canvas visualization tests should be readable");
    let signal = fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/signal.rs"))
        .expect("signal visualization tests should be readable");
    let timeline = fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/timeline.rs"))
        .expect("timeline visualization test root should be readable");
    let timeline_mapper =
        fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/timeline/mapper.rs"))
            .expect("timeline mapper tests should be readable");
    let timeline_metadata =
        fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/timeline/metadata.rs"))
            .expect("timeline metadata tests should be readable");
    let timeline_aggregate =
        fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/timeline/aggregate.rs"))
            .expect("timeline aggregate tests should be readable");
    let timeline_fixtures =
        fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/timeline/fixtures.rs"))
            .expect("timeline visualization test fixtures should be readable");

    assert!(
        root.contains("mod spatial;")
            && root.contains("mod canvas;")
            && root.contains("mod signal;")
            && root.contains("mod timeline;")
            && !root.contains("fn timeline_motion_state")
            && !root.contains("fn canvas_layer_hit_testing"),
        "visualization test root should index focused behavior groups instead of owning all visualization cases"
    );
    assert!(
        spatial.contains("fn normalized_milli_point_projects_and_clamps_into_rect")
            && canvas.contains("fn canvas_invalidation_splits_scene_and_interaction_rebuilds")
            && signal.contains("fn signal_tool_state_preserves_generic_interaction_flags")
            && timeline.contains("mod mapper;")
            && timeline.contains("mod metadata;")
            && timeline.contains("mod aggregate;")
            && timeline.contains("mod fixtures;")
            && !timeline.contains("fn timeline_motion_state_aggregates_surface_chrome_tools"),
        "visualization behavior tests should stay grouped by spatial, canvas, signal, and timeline concerns"
    );
    assert!(
        timeline_mapper.contains("fn timeline_coordinate_mapper_projects_and_back_projects_micros")
            && timeline_metadata.contains(
                "fn timeline_transport_state_preserves_positions_and_resolves_micro_playhead"
            )
            && timeline_aggregate
                .contains("fn timeline_motion_state_aggregates_surface_chrome_tools")
            && timeline_fixtures.contains("fn timeline_viewport_parts"),
        "timeline visualization tests should stay grouped by mapper, metadata, aggregate, and fixture concerns"
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
