use super::fixtures::fixed_virtualized_root;
use crate::gui::{
    layout_core::{
        engine::{LayoutDebugOptions, LayoutEngine, LayoutState},
        tree::{LayoutNode, WidgetNode},
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
