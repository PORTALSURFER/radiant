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
            && spatial.contains("fn spatial_panel_groups_labels_selection_and_point_data")
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
        spatial_model.contains("pub struct SpatialPanelStatus")
            && spatial_model.contains("pub struct SpatialPanelLabels")
            && spatial_model.contains("pub struct SpatialPanelSelection")
            && spatial_model.contains("pub struct SpatialPanelPoints")
            && spatial_model.contains("pub struct SpatialPanel"),
        "spatial panel state should stay grouped by status, labels, selection, and point data"
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
