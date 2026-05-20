use super::*;

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
