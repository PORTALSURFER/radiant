use super::*;

#[test]
fn traversal_records_route_to_expected_buckets() {
    let mut index = SurfaceTraversalIndex::with_stats(SurfaceTraversalStats {
        widgets: 1,
        stateful_widgets: 1,
        styled_hoverable_containers: 1,
        scroll_containers: 1,
        clipped_containers: 1,
        max_depth: 1,
        max_scroll_depth: 1,
    });

    index.record_container(SurfaceContainerTraversalRecord {
        id: 10,
        clipped_by: &[1],
        scroll_content: Some(11),
        styled_hoverable: true,
    });
    index.record_widget(SurfaceWidgetTraversalRecord {
        id: 20,
        child_path: &[0, 1],
        clipped_by: &[10],
        focusable: true,
        keyboard_focusable: true,
        receives_pointer_hit_testing: true,
        receives_wheel_input: true,
        accepts_native_file_drop: true,
        needs_state_synchronization: true,
        suppresses_container_hover: true,
    });

    assert_eq!(index.scroll_container_order, vec![10]);
    assert_eq!(index.scroll_content_by_container.get(&10), Some(&11));
    assert_eq!(index.styled_container_order, vec![10]);
    assert_eq!(
        index
            .container_clip_ancestors
            .get(&10)
            .map(|path| path.as_slice()),
        Some(&[1][..])
    );
    assert_eq!(index.widget_paint_order, vec![20]);
    assert_eq!(index.focusable_widget_order, vec![20]);
    assert_eq!(index.keyboard_focus_order, vec![20]);
    assert_eq!(index.pointer_hit_order, vec![20]);
    assert_eq!(index.wheel_hit_order, vec![20]);
    assert_eq!(index.native_file_drop_hit_order, vec![20]);
    assert_eq!(index.stateful_widget_order, vec![20]);
    assert!(index.container_hover_suppression.contains(&20));
    assert_eq!(
        index.widget_paths.get(&20).map(|path| path.as_slice()),
        Some(&[0, 1][..])
    );
    assert_eq!(
        index
            .widget_clip_ancestors
            .get(&20)
            .map(|path| path.as_slice()),
        Some(&[10][..])
    );
}
