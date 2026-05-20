use super::*;

#[test]
fn scroll_virtualization_limits_materialized_nodes_for_large_lists() {
    let root = scroll_with_content(
        ContainerKind::Column,
        10_000,
        VirtualizationAxis::Vertical,
        0.0,
    );
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(0.0, 40_000.0));

    let output = layout_tree_with_state(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(220.0, 120.0)),
        &state,
        LayoutDebugOptions::default(),
    );

    let info = output
        .virtual_windows
        .get(&1)
        .expect("virtual window metadata");
    assert_eq!(info.total_children, 10_000);
    assert!(info.first_index > 0);
    assert!(info.culled_after > 0);
    assert!(info.last_index_exclusive - info.first_index < 64);
    assert!(output.stats.materialized_nodes < 128);
    assert!(
        output.stats.measured_nodes < 64,
        "direct intrinsic widget rows should not force cold full-list measurement"
    );
    assert_eq!(output.rects.len(), output.stats.materialized_nodes);
}

#[test]
fn fixed_size_virtualized_scroll_avoids_cold_full_list_measurement() {
    let children = (0..10_000_u64)
        .map(|index| SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fixed(28.0),
                size_cross: SizeModeCross::Fill,
                constraints: Constraints::unconstrained(),
                margin: Default::default(),
                align_cross_override: None,
                allow_fixed_compress: false,
            },
            child: LayoutNode::widget(index + 10, Vector2::new(180.0, 20.0)),
        })
        .collect::<Vec<_>>();
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis: VirtualizationAxis::Vertical,
                overscan_px: 56.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                2,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing: 2.0,
                    ..ContainerPolicy::default()
                },
                children,
            ),
        }],
    );
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(0.0, 8_000.0));

    let output = LayoutEngine::default().layout_with_state(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 140.0)),
        &state,
        LayoutDebugOptions::default(),
    );

    let window = output
        .virtual_windows
        .get(&1)
        .expect("fixed-size virtual window metadata");
    assert_eq!(window.total_children, 10_000);
    assert!(window.last_index_exclusive - window.first_index < 32);
    assert!(
        output.stats.measured_nodes < 64,
        "fixed-size virtual metrics should not measure every row on a cold layout"
    );
}
