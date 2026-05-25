use super::*;

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
    let index_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/traversal/index/tests.rs"))
            .expect("surface traversal index tests should be readable");
    let capacity_tests = fs::read_to_string(
        manifest_dir.join("src/runtime/surface/traversal/index/capacity/tests.rs"),
    )
    .expect("surface traversal capacity tests should be readable");

    assert!(
        index.contains("mod capacity;")
            && index.contains("mod recording;")
            && index.contains("mod records;")
            && index.contains("#[path = \"index/tests.rs\"]")
            && index.contains("pub(in crate::runtime) use records::{")
            && !index.contains("fn record_container")
            && !index.contains("fn record_widget")
            && !index.contains("fn traversal_records_route_to_expected_buckets"),
        "surface traversal index root should delegate traversal bucket mutation helpers and behavior tests"
    );
    assert!(
        index_tests.contains("fn traversal_records_route_to_expected_buckets"),
        "surface traversal index behavior coverage should live in traversal/index/tests.rs"
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
            && capacity.contains("#[path = \"capacity/tests.rs\"]")
            && !index.contains("fn reserve_vec_capacity")
            && !index.contains("fn reserve_map_capacity")
            && !index.contains("fn reserve_set_capacity")
            && !capacity.contains("fn widget_clip_capacity_is_zero_without_scroll_containers"),
        "surface traversal capacity and reuse helpers should live in traversal/index/capacity.rs while behavior tests stay delegated"
    );
    assert!(
        capacity_tests.contains("fn widget_clip_capacity_is_zero_without_scroll_containers")
            && capacity_tests
                .contains("fn widget_clip_capacity_tracks_widgets_when_scroll_containers_exist"),
        "surface traversal capacity behavior coverage should live in traversal/index/capacity/tests.rs"
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
            && layout.contains("traversal.record_widget(SurfaceWidgetTraversalRecord")
            && layout.contains("SurfaceContainer, SurfaceContainerTraversalRecord, SurfaceNode")
            && layout.contains("SurfaceTraversalIndex")
            && layout.contains("SurfaceTraversalStats")
            && layout.contains("SurfaceWidget, SurfaceWidgetTraversalRecord, UiSurface")
            && layout.contains("layout::{ContainerKind, LayoutNode, NodeId, SlotChild, Vector2}")
            && !layout.starts_with("use super::*;"),
        "surface layout projection should name surface, traversal, and layout dependencies while describing traversal records instead of mutating buckets directly"
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
