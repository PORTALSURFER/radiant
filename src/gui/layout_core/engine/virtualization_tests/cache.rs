use super::*;
use crate::gui::layout_core::engine::cache::{CachedVirtualMetrics, LinearVirtualMetrics};
use std::sync::Arc;

#[test]
fn virtualized_metrics_cache_tracks_fixed_row_shape_changes() {
    let mut engine = LayoutEngine::default();
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(0.0, 160.0));
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 140.0));

    let first = engine.layout_with_state(
        &fixed_virtualized_scroll_root(24.0),
        viewport,
        &state,
        LayoutDebugOptions::default(),
    );
    let second = engine.layout_with_state(
        &fixed_virtualized_scroll_root(40.0),
        viewport,
        &state,
        LayoutDebugOptions::default(),
    );

    assert_eq!(
        first
            .virtual_windows
            .get(&1)
            .expect("first virtual window")
            .resolved_total_main,
        24.0 * 128.0 + 2.0 * 127.0
    );
    assert_eq!(
        second
            .virtual_windows
            .get(&1)
            .expect("second virtual window")
            .resolved_total_main,
        40.0 * 128.0 + 2.0 * 127.0
    );
}

#[test]
fn invalid_cached_virtual_metrics_are_rebuilt_before_window_resolution() {
    let root = fixed_virtualized_scroll_root(24.0);
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 140.0));
    let mut engine = LayoutEngine::default();

    let first = engine.layout_with_state(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );
    assert!(first.virtual_windows.contains_key(&1));
    let key = *engine
        .virtual_cache
        .keys()
        .next()
        .expect("warmed virtual metrics key");
    engine.virtual_cache.insert(
        key,
        CachedVirtualMetrics::new(
            Arc::new(LinearVirtualMetrics {
                total_main: f32::NAN,
                ..LinearVirtualMetrics::default()
            }),
            vec![2],
        ),
    );

    let second = engine.layout_with_state(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );

    assert!(second.virtual_windows.contains_key(&1));
    assert!(
        !second
            .diagnostics
            .iter()
            .any(|item| item.code == LayoutDiagnosticCode::VirtualizationSpanResolutionFallback)
    );
    let cached = engine
        .virtual_cache
        .get(&key)
        .expect("rebuilt virtual metrics");
    assert!(cached.metrics.total_main.is_finite());
    assert_eq!(cached.metrics.len(), 128);
}
