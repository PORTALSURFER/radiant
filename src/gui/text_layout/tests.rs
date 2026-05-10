use super::*;

#[test]
fn centered_line_reuses_cached_geometry_for_identical_inputs() {
    let mut cache = TextLineLayoutCache::new();
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 60.0));
    let first = centered_text_line_with_cache(
        &mut cache,
        bounds,
        12.0,
        TextLineInsets::symmetric(8.0, 3.0),
        0.0,
        1,
    );
    let second = centered_text_line_with_cache(
        &mut cache,
        bounds,
        12.0,
        TextLineInsets::symmetric(8.0, 3.0),
        0.0,
        1,
    );
    assert_eq!(first, second);
    assert_eq!(cache.misses_for_test(), 1);
}

#[test]
fn centered_line_invalidates_when_font_bounds_or_insets_change() {
    let mut cache = TextLineLayoutCache::new();
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 60.0));
    let base = centered_text_line_with_cache(
        &mut cache,
        bounds,
        12.0,
        TextLineInsets::symmetric(8.0, 3.0),
        0.0,
        2,
    );
    let taller = centered_text_line_with_cache(
        &mut cache,
        bounds,
        14.0,
        TextLineInsets::symmetric(8.0, 3.0),
        0.0,
        2,
    );
    let wider = centered_text_line_with_cache(
        &mut cache,
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(230.0, 60.0)),
        12.0,
        TextLineInsets::symmetric(8.0, 3.0),
        0.0,
        2,
    );
    let inset = centered_text_line_with_cache(
        &mut cache,
        bounds,
        12.0,
        TextLineInsets::symmetric(10.0, 3.0),
        0.0,
        2,
    );
    assert_ne!(base, taller);
    assert_ne!(base, wider);
    assert_ne!(base, inset);
    assert_eq!(cache.misses_for_test(), 4);
}

#[test]
fn top_line_uses_top_edge_after_horizontal_inset() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 60.0));
    let line = top_text_line(bounds, 11.0, TextLineInsets::horizontal(5.0));
    assert_eq!(line.min, Point::new(15.0, 20.0));
    assert_eq!(line.max, Point::new(205.0, 31.0));
}

#[test]
fn explicit_cache_eviction_keeps_capacity_bounded() {
    let mut cache = TextLineLayoutCache::with_capacity(2);
    let bounds = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(120.0, 40.0));

    for family_id in 1..=3 {
        let _ = top_text_line_with_cache(
            &mut cache,
            bounds,
            12.0,
            TextLineInsets::horizontal(4.0),
            family_id,
        );
    }

    assert_eq!(cache.len(), 2);
    assert_eq!(cache.misses_for_test(), 3);
}

#[test]
fn custom_cache_capacity_is_clamped_to_default_limit() {
    let mut cache = TextLineLayoutCache::with_capacity(usize::MAX);
    let bounds = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(120.0, 40.0));

    for family_id in 0..256 {
        let _ = top_text_line_with_cache(
            &mut cache,
            bounds,
            12.0,
            TextLineInsets::horizontal(4.0),
            family_id,
        );
    }

    assert_eq!(cache.len(), 128);
    assert_eq!(cache.misses_for_test(), 256);
}

#[test]
fn snap_text_baseline_to_pixel_keeps_height_and_rounds_bottom_edge() {
    let line = Rect::from_min_max(Point::new(10.0, 20.25), Point::new(110.0, 34.75));

    assert_eq!(
        snap_text_baseline_to_pixel(line),
        Rect::from_min_max(Point::new(10.0, 20.5), Point::new(110.0, 35.0))
    );
}
