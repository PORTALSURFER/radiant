use super::fixtures::fixed_virtualized_root;
use crate::gui::{
    layout_core::{
        constraints::Constraints,
        engine::{LayoutDebugOptions, LayoutEngine, LayoutState},
        model::{ContainerPolicy, SizeModeCross, SizeModeMain, SlotParams},
        tree::{LayoutNode, SlotChild, WidgetNode},
    },
    types::{Point, Rect, Vector2},
};

#[test]
fn layout_engine_prunes_stale_measure_cache_versions() {
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 64.0));
    let mut engine = LayoutEngine::default();

    for state_version in 0..16 {
        let root = LayoutNode::Widget(WidgetNode {
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
fn layout_engine_keeps_exact_measure_cache_without_rebuilding_storage() {
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 64.0));
    let root = LayoutNode::Widget(WidgetNode {
        id: 1,
        intrinsic: Vector2::new(40.0, 20.0),
        state_version: 1,
    });
    let mut engine = LayoutEngine::default();

    engine.layout(&root, viewport);
    let capacity = engine.measure_cache.capacity();

    engine.layout(&root, viewport);

    assert_eq!(engine.measure_cache.len(), 1);
    assert_eq!(engine.measure_cache.capacity(), capacity);
}

#[test]
fn layout_engine_presizes_measure_scratch_from_persistent_cache() {
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(320.0, 120.0));
    let root = many_widget_row(96);
    let mut engine = LayoutEngine::default();

    engine.layout(&root, viewport);
    let cache_len = engine.measure_cache.len();

    engine.scratch.measured.shrink_to(0);
    engine.layout(&root, viewport);

    assert_eq!(engine.measure_cache.len(), cache_len);
    assert!(engine.scratch.measured.capacity() >= cache_len);
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
fn layout_engine_keeps_exact_virtual_cache_without_rebuilding_storage() {
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 64.0));
    let state = LayoutState::default();
    let root = fixed_virtualized_root(48, 12.0);
    let mut engine = LayoutEngine::default();

    engine.layout_with_state(&root, viewport, &state, LayoutDebugOptions::default());
    let capacity = engine.virtual_cache.capacity();

    engine.layout_with_state(&root, viewport, &state, LayoutDebugOptions::default());

    assert_eq!(engine.virtual_cache.len(), 1);
    assert_eq!(engine.virtual_cache.capacity(), capacity);
}

#[test]
fn layout_engine_presizes_virtual_scratch_from_persistent_cache() {
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 64.0));
    let state = LayoutState::default();
    let root = fixed_virtualized_root(48, 12.0);
    let mut engine = LayoutEngine::default();

    engine.layout_with_state(&root, viewport, &state, LayoutDebugOptions::default());
    let cache_len = engine.virtual_cache.len();

    engine.scratch.virtual_touched.shrink_to(0);
    engine.layout_with_state(&root, viewport, &state, LayoutDebugOptions::default());

    assert_eq!(engine.virtual_cache.len(), cache_len);
    assert!(engine.scratch.virtual_touched.capacity() >= cache_len);
}

fn many_widget_row(count: u64) -> LayoutNode {
    LayoutNode::container(
        1,
        ContainerPolicy::default(),
        (0..count)
            .map(|index| {
                SlotChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Fixed(8.0),
                        size_cross: SizeModeCross::Fixed(12.0),
                        constraints: Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    LayoutNode::widget(index + 10, Vector2::new(8.0, 12.0)),
                )
            })
            .collect(),
    )
}
