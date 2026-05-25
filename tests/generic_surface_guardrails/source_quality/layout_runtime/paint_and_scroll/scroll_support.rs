use super::*;

#[test]
fn runtime_scroll_support_keeps_affordance_and_hit_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let paint_scroll = fs::read_to_string(manifest_dir.join("src/runtime/paint/scroll.rs"))
        .expect("runtime paint scroll helpers should be readable");
    let paint_scroll_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/paint/scroll/tests.rs"))
            .expect("runtime paint scroll tests should be readable");
    let controller_scroll =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/scroll.rs"))
            .expect("runtime controller scroll root should be readable");
    let controller_scrollbar =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/scroll/scrollbar.rs"))
            .expect("runtime controller scrollbar helpers should be readable");
    let controller_scrollbar_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/scroll/scrollbar/tests.rs"))
            .expect("runtime controller scrollbar tests should be readable");

    assert!(
        paint_scroll.contains("struct ScrollAffordance")
            && paint_scroll.contains("fn push_scroll_affordance")
            && paint_scroll.contains("fn resolve_scroll_affordance")
            && paint_scroll.contains("#[path = \"scroll/tests.rs\"]")
            && !paint_scroll.contains("fn scroll_affordance_clamps_thumb_to_cramped_track"),
        "runtime scroll paint affordance helpers should live in paint/scroll.rs while behavior tests stay delegated"
    );
    assert!(
        paint_scroll_tests.contains("fn scroll_affordance_clamps_thumb_to_cramped_track")
            && paint_scroll_tests.contains("fn scroll_affordance_rejects_nonfinite_layout_rects"),
        "runtime scroll paint behavior coverage should live in paint/scroll/tests.rs"
    );
    assert!(
        controller_scroll.contains("use super::SurfaceRuntime;")
            && controller_scroll.contains("gui::types::{Point, Vector2}")
            && controller_scroll.contains("layout::{NodeId, OverflowPolicy}")
            && controller_scroll.contains("runtime::RuntimeBridge")
            && !controller_scroll.starts_with("use super::*;")
            && controller_scroll.contains("pub struct ScrollUpdate")
            && controller_scroll.contains("fn scroll_container_at"),
        "runtime controller scroll root should name controller, geometry, layout, bridge, and scroll-update dependencies"
    );
    assert!(
        controller_scrollbar.contains("super::{ScrollDragCapture, SurfaceRuntime}")
            && controller_scrollbar.contains("ScrollUpdate")
            && controller_scrollbar.contains("gui::types::{Point, Rect, Vector2}")
            && controller_scrollbar.contains("layout::NodeId")
            && controller_scrollbar
                .contains("runtime::{RuntimeBridge, paint::resolve_scroll_affordance}")
            && !controller_scrollbar.starts_with("use super::{super::*")
            && controller_scrollbar.contains("fn scrollbar_hit_column_contains_point")
            && controller_scrollbar.contains("fn scrollbar_thumb_hit_rect")
            && controller_scrollbar.contains("#[path = \"scrollbar/tests.rs\"]")
            && !controller_scrollbar
                .contains("fn scrollbar_hit_column_rejects_points_far_from_right_edge"),
        "runtime scrollbar hit and drag helpers should name controller, state, scroll update, geometry, layout, bridge, and paint affordance dependencies while behavior tests stay delegated"
    );
    assert!(
        controller_scrollbar_tests
            .contains("fn scrollbar_hit_column_rejects_points_far_from_right_edge"),
        "runtime scrollbar behavior coverage should live in controller/scroll/scrollbar/tests.rs"
    );
}
