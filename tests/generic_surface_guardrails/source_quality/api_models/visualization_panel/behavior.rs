use super::*;

#[test]
fn visualization_behavior_tests_stay_grouped_by_surface_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/visualization/tests.rs"))
        .expect("visualization test root should be readable");
    let spatial = fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/spatial.rs"))
        .expect("spatial visualization tests should be readable");
    let spatial_model = fs::read_to_string(manifest_dir.join("src/gui/visualization/spatial.rs"))
        .expect("spatial visualization model should be readable");
    let canvas = fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/canvas.rs"))
        .expect("canvas visualization tests should be readable");
    let canvas_layers =
        fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/canvas/layers.rs"))
            .expect("canvas layer visualization tests should be readable");
    let grid = fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/grid.rs"))
        .expect("dense grid visualization tests should be readable");
    let grid_model = fs::read_to_string(manifest_dir.join("src/gui/visualization/grid.rs"))
        .expect("dense grid visualization model should be readable");
    let signal = fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/signal.rs"))
        .expect("signal visualization tests should be readable");
    let strip =
        fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/strip_layout.rs"))
            .expect("strip visualization tests should be readable");
    let strip_model =
        fs::read_to_string(manifest_dir.join("src/gui/visualization/strip_layout.rs"))
            .expect("strip visualization model should be readable");
    let timeline = fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/timeline.rs"))
        .expect("timeline visualization test root should be readable");
    let timeline_mapper =
        fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/timeline/mapper.rs"))
            .expect("timeline mapper tests should be readable");
    let timeline_item =
        fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/timeline/item.rs"))
            .expect("timeline item tests should be readable");
    let timeline_metadata =
        fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/timeline/metadata.rs"))
            .expect("timeline metadata tests should be readable");
    let timeline_pitch =
        fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/timeline/pitch.rs"))
            .expect("timeline pitch tests should be readable");
    let timeline_aggregate =
        fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/timeline/aggregate.rs"))
            .expect("timeline aggregate tests should be readable");
    let timeline_fixtures =
        fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/timeline/fixtures.rs"))
            .expect("timeline visualization test fixtures should be readable");

    assert!(
        root.contains("mod spatial;")
            && root.contains("mod canvas;")
            && root.contains("mod grid;")
            && root.contains("mod signal;")
            && root.contains("mod timeline;")
            && !root.contains("fn timeline_motion_state")
            && !root.contains("fn canvas_layer_hit_testing"),
        "visualization test root should index focused behavior groups instead of owning all visualization cases"
    );
    assert!(
        spatial.contains("fn normalized_milli_point_projects_and_clamps_into_rect")
            && spatial.contains("fn spatial_panel_groups_labels_selection_and_point_data")
            && canvas.contains("mod layers;")
            && canvas_layers
                .contains("fn canvas_invalidation_splits_scene_and_interaction_rebuilds")
            && grid.contains("fn dense_grid_raster_layout_projects_bottom_up_cells_with_bleed")
            && signal.contains("fn signal_tool_state_preserves_generic_interaction_flags")
            && strip.contains("fn vertical_strip_stack_layout_projects_bottom_anchored_slots")
            && timeline.contains("mod item;")
            && timeline.contains("mod mapper;")
            && timeline.contains("mod metadata;")
            && timeline.contains("mod pitch;")
            && timeline.contains("mod aggregate;")
            && timeline.contains("mod fixtures;")
            && !timeline.contains("fn timeline_motion_state_aggregates_surface_chrome_tools"),
        "visualization behavior test root should delegate timeline behavior groups"
    );
    assert!(
        timeline_aggregate
            .contains("fn timeline_motion_state_aggregates_surface_chrome_tools_and_transport"),
        "visualization behavior tests should stay grouped by spatial, canvas, signal, and timeline concerns"
    );
    assert!(
        spatial_model.contains("pub struct SpatialPanelStatus")
            && spatial_model.contains("pub struct SpatialPanelLabels")
            && spatial_model.contains("pub struct SpatialPanelSelection")
            && spatial_model.contains("pub struct SpatialPanelPoints")
            && spatial_model.contains("pub struct SpatialPanel"),
        "spatial panel state should stay grouped by status, labels, selection, and point data"
    );
    assert!(
        grid_model.contains("pub struct DenseGridRasterLayoutParts")
            && grid_model.contains("pub struct DenseGridLabelLayoutParts")
            && grid_model.contains("pub enum DenseGridRowOrigin")
            && grid_model.contains("pub fn row_label_rect(self, label_bounds: Rect, row: usize)")
            && grid_model.contains("pub fn cell_rect(self, cell: DenseGridCell) -> Option<Rect>"),
        "dense grid visualization model should own reusable raster and label projection"
    );
    assert!(
        strip_model.contains("pub struct HorizontalStripLayoutParts")
            && strip_model.contains("pub struct VerticalStripStackLayoutParts")
            && strip_model.contains("pub enum VerticalStripStackOrigin")
            && strip_model.contains("pub fn slot_rect(self, slot: usize) -> Option<Rect>")
            && strip_model.contains("pub fn strip_rect(self, strip: usize) -> Option<Rect>"),
        "strip visualization model should own reusable dense strip and stacked slot projection"
    );
    assert!(
        timeline_mapper.contains("fn timeline_coordinate_mapper_projects_and_back_projects_micros")
            && timeline_item.contains("fn timeline_item_layout_projects_centered_lane_items")
            && timeline_metadata.contains(
                "fn timeline_transport_state_preserves_positions_and_resolves_micro_playhead"
            )
            && timeline_pitch.contains("fn timeline_pitch_layout_projects_top_down_pitch_rows")
            && timeline_aggregate
                .contains("fn timeline_motion_state_aggregates_surface_chrome_tools")
            && timeline_fixtures.contains("fn timeline_viewport_parts"),
        "timeline visualization tests should stay grouped by item, mapper, metadata, pitch, aggregate, and fixture concerns"
    );
}
