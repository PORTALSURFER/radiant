use super::*;

#[test]
fn layout_engine_reuses_scratch_maps_between_passes() {
    let children = (0..64)
        .map(|index| {
            SlotChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(12.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                LayoutNode::widget(index + 10, Vector2::new(40.0, 12.0)),
            )
        })
        .collect();
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis: VirtualizationAxis::Vertical,
                overscan_px: 8.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild::new(
            SlotParams::fill(),
            LayoutNode::container(
                2,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    ..ContainerPolicy::default()
                },
                children,
            ),
        )],
    );
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 64.0));
    let mut engine = LayoutEngine::default();

    let first = engine.layout_with_state(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );
    assert!(first.virtual_windows.contains_key(&1));
    let measured_capacity = engine.scratch.measured.capacity();
    let measured_by_node_capacity = engine.scratch.measured_by_node.capacity();
    let linear_window_capacity = engine.scratch.linear_windows.capacity();
    assert!(measured_capacity > 0);
    assert_eq!(
        measured_by_node_capacity, 0,
        "default layout should not populate measured-by-node debug storage"
    );
    assert!(linear_window_capacity > 0);

    let second = engine.layout_with_state(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );

    assert!(second.virtual_windows.contains_key(&1));
    assert!(engine.scratch.measured.capacity() >= measured_capacity);
    assert_eq!(engine.scratch.measured_by_node.capacity(), 0);
    assert!(engine.scratch.linear_windows.capacity() >= linear_window_capacity);

    let debug = engine.layout_with_state(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::all_enabled(),
    );

    assert!(debug.virtual_windows.contains_key(&1));
    assert!(engine.scratch.measured_by_node.capacity() > 0);
}

#[test]
fn layout_engine_reuses_linear_scratch_vectors_between_passes() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Column,
            overflow: OverflowPolicy::Shrink,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(80.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: true,
                },
                LayoutNode::widget(10, Vector2::new(40.0, 80.0)),
            ),
            SlotChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fill(1.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints {
                        min_h: 40.0,
                        ..Constraints::unconstrained()
                    },
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                LayoutNode::widget(20, Vector2::new(40.0, 40.0)),
            ),
            SlotChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(80.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: true,
                },
                LayoutNode::widget(30, Vector2::new(40.0, 80.0)),
            ),
        ],
    );
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 100.0));
    let mut engine = LayoutEngine::default();

    let first = engine.layout(&root, viewport);
    let linear_sizes_capacity = engine.scratch.linear_sizes.capacity();
    let linear_unresolved_capacity = engine.scratch.linear_unresolved.capacity();

    assert!(first.rects.contains_key(&30));
    assert!(linear_sizes_capacity >= 3);
    assert!(linear_unresolved_capacity >= 1);
    assert!(engine.scratch.linear_sizes.is_empty());
    assert!(engine.scratch.linear_unresolved.is_empty());

    let second = engine.layout(&root, viewport);

    assert!(second.rects.contains_key(&30));
    assert!(engine.scratch.linear_sizes.capacity() >= linear_sizes_capacity);
    assert!(engine.scratch.linear_unresolved.capacity() >= linear_unresolved_capacity);
    assert!(engine.scratch.linear_sizes.is_empty());
    assert!(engine.scratch.linear_unresolved.is_empty());
}

#[test]
fn layout_engine_prunes_stale_measure_cache_versions() {
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 64.0));
    let mut engine = LayoutEngine::default();

    for state_version in 0..16 {
        let root = LayoutNode::Widget(crate::gui::layout_core::tree::WidgetNode {
            id: 1,
            intrinsic: Vector2::new(40.0, 20.0),
            state_version,
        });
        let output = engine.layout(&root, viewport);

        assert!(output.rects.contains_key(&1));
        assert_eq!(
            engine.measure_cache.len(),
            1,
            "persistent measure cache should retain only entries touched by the latest layout pass"
        );
    }
}

#[test]
fn layout_engine_prunes_stale_virtualization_cache_entries() {
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 64.0));
    let state = LayoutState::default();
    let mut engine = LayoutEngine::default();

    for child_count in 32..48 {
        let root = fixed_virtualized_root(child_count, 12.0);
        let output =
            engine.layout_with_state(&root, viewport, &state, LayoutDebugOptions::default());

        assert!(output.virtual_windows.contains_key(&1));
        assert_eq!(
            engine.virtual_cache.len(),
            1,
            "persistent virtualization cache should retain only entries touched by the latest layout pass"
        );
    }
}

#[test]
fn dirty_subtree_invalidates_virtual_metrics_cache_for_whole_marked_set() {
    let children = (0..64)
        .map(|index| {
            SlotChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(12.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                LayoutNode::widget(index + 10, Vector2::new(40.0, 12.0)),
            )
        })
        .collect();
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis: VirtualizationAxis::Vertical,
                overscan_px: 8.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild::new(
            SlotParams::fill(),
            LayoutNode::container(
                2,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    ..ContainerPolicy::default()
                },
                children,
            ),
        )],
    );
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 64.0));
    let mut engine = LayoutEngine::default();

    let first = engine.layout_with_state(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );
    assert!(first.virtual_windows.contains_key(&1));
    assert!(!engine.virtual_cache.is_empty());
    let dependencies = &engine
        .virtual_cache
        .values()
        .next()
        .expect("cached virtual metrics")
        .dependencies;
    assert!(dependencies.contains(&2));
    assert!(dependencies.contains(&10));
    assert_eq!(
        dependencies.len(),
        65,
        "virtual metric dependencies should be stored as one compact subtree id list"
    );

    engine.mark_layout_dirty_subtree(&root, 2);

    assert!(
        engine.virtual_cache.is_empty(),
        "dirtying virtualized content should drop cached span metrics"
    );
}

fn fixed_virtualized_root(child_count: u64, row_height: f32) -> LayoutNode {
    let children = (0..child_count)
        .map(|index| {
            SlotChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(row_height),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                LayoutNode::widget(index + 10, Vector2::new(40.0, row_height)),
            )
        })
        .collect();
    LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis: VirtualizationAxis::Vertical,
                overscan_px: 8.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild::new(
            SlotParams::fill(),
            LayoutNode::container(
                2,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    ..ContainerPolicy::default()
                },
                children,
            ),
        )],
    )
}

#[test]
fn dirty_subtree_traversal_reuses_scratch_buffers_between_marks() {
    let children = (0..48)
        .map(|index| {
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(index + 10, Vector2::new(40.0, 12.0)),
            )
        })
        .collect();
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Column,
            ..ContainerPolicy::default()
        },
        vec![SlotChild::new(
            SlotParams::fill(),
            LayoutNode::container(
                2,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    ..ContainerPolicy::default()
                },
                children,
            ),
        )],
    );
    let mut engine = LayoutEngine::default();

    engine.mark_layout_dirty_subtree(&root, 2);
    let path_capacity = engine.scratch.dirty_path.capacity();
    let marked_capacity = engine.scratch.dirty_marked.capacity();

    assert!(path_capacity >= 2);
    assert!(marked_capacity >= 49);
    assert!(engine.scratch.dirty_path.is_empty());
    assert!(engine.scratch.dirty_marked.is_empty());

    engine.clear_dirty();
    engine.mark_measure_dirty_subtree(&root, 2);

    assert!(engine.scratch.dirty_path.capacity() >= path_capacity);
    assert!(engine.scratch.dirty_marked.capacity() >= marked_capacity);
    assert!(engine.scratch.dirty_path.is_empty());
    assert!(engine.scratch.dirty_marked.is_empty());
}
