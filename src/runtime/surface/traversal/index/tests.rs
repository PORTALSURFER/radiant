use super::*;

#[test]
fn traversal_records_route_to_expected_buckets() {
    let mut index = SurfaceTraversalIndex::<()>::with_stats(SurfaceTraversalStats {
        source_nodes: 0,
        widgets: 1,
        stateful_widgets: 1,
        styled_hoverable_containers: 1,
        scroll_containers: 1,
        clipped_containers: 1,
        split_pane_focus_order_candidates: 0,
        max_depth: 1,
        max_scroll_depth: 1,
    });

    index.record_container(SurfaceContainerTraversalRecord {
        id: 10,
        clipped_by: &[1],
        scroll_content: Some(11),
        styled_hoverable: true,
        layout_interaction: None,
        split_pane_runtime: None,
        split_pane_divider: None,
        split_pane_ratio_action: None,
        virtual_layout: None,
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
    let input = crate::gui::layout_core::SplitPaneRuntimeStateInput {
        container_id: 10,
        initial_ratio: 0.5,
        mode: crate::gui::layout_core::SplitPaneRuntimeMode::RuntimeOwned {
            collapse_policy: None,
        },
        policy_revision: Default::default(),
    };
    index.record_split_pane_focus_order_candidate(SurfaceSplitPaneFocusOrderCandidate {
        widget_index: 0,
        target: crate::layout::LayoutTargetIdentity::new(
            10,
            crate::gui::layout_core::SPLIT_PANE_DIVIDER_REGION_ID,
        ),
        state_id: input.state_id(),
        descriptor: crate::gui::layout_core::SplitPaneDividerDescriptor {
            container_id: 10,
            first_child: 11,
            second_child: 12,
            axis: crate::layout::SplitPaneAxis::Horizontal,
            first_min_extent: 0.0,
            second_min_extent: 0.0,
            divider_extent: 8.0,
        },
        ownership: crate::gui::layout_core::SplitPaneRuntimeOwnership::RuntimeOwned,
        contract_version: crate::layout::LAYOUT_CAPABILITIES_STATE_CONTRACT_VERSION,
        state_schema_version: input.state_id().schema_version(),
        policy_revision: input.policy_revision,
    });
    assert_eq!(index.keyboard_focus_order_candidates.len(), 1);
    assert_eq!(index.keyboard_focus_order_candidates[0].widget_index, 1);
    assert_eq!(index.pointer_hit_order, vec![20]);
    assert_eq!(index.wheel_hit_order, vec![20]);
    assert_eq!(
        index.wheel_target_order,
        vec![
            WheelHitTarget::ScrollContainer(10),
            WheelHitTarget::Widget(20)
        ]
    );
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
