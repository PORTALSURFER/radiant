use super::fixtures::fixed_virtualized_root;
use crate::gui::{
    layout_core::{
        engine::{LayoutDebugOptions, LayoutEngine, LayoutState},
        model::{ContainerKind, ContainerPolicy, SlotParams},
        tree::{LayoutNode, SlotChild},
    },
    types::{Point, Rect, Vector2},
};

#[test]
fn dirty_subtree_invalidates_virtual_metrics_cache_for_whole_marked_set() {
    let root = fixed_virtualized_root(64, 12.0);
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
