use super::*;
use crate::gui::layout_core::engine::cache::UniformVirtualMetrics;

#[test]
fn uniform_virtual_window_matches_visible_span_bounds() {
    let metrics = LinearVirtualMetrics {
        spans: Vec::new(),
        main_sizes: Vec::new(),
        uniform: Some(UniformVirtualMetrics {
            count: 10_000,
            main_size: 28.0,
            step: 29.0,
        }),
        total_main: 289_999.0,
        leading_offset: 0.0,
        distributed_spacing: 1.0,
    };

    let window = compute_virtual_window(&metrics, 20_000.0, 140.0, 16.0);

    assert!(!window.clamped);
    assert_eq!(window.start, 19_984.0);
    assert_eq!(window.end, 20_156.0);
    assert!(window.first > 0);
    assert!(window.last_exclusive > window.first);
    assert!(window.last_exclusive - window.first < 16);
    let first_span = metrics.span(window.first).expect("first visible span");
    let last_span = metrics
        .span(window.last_exclusive - 1)
        .expect("last visible span");
    assert!(first_span.end > window.start);
    assert!(last_span.start < window.end);
}
