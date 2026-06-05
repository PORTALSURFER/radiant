use super::{IndexViewport, IndexViewportScope};
use crate::gui::range::NormalizedRange;

fn assert_ratio_near(actual: Option<f32>, expected: f32) {
    let actual = actual.expect("visible ratio");
    assert!(
        (actual - expected).abs() <= 0.000_001,
        "actual={actual} expected={expected}"
    );
}

fn assert_range_near(actual: Option<(f32, f32)>, expected: (f32, f32)) {
    let actual = actual.expect("visible range");
    assert!(
        (actual.0 - expected.0).abs() <= 0.000_001,
        "actual={actual:?} expected={expected:?}"
    );
    assert!(
        (actual.1 - expected.1).abs() <= 0.000_001,
        "actual={actual:?} expected={expected:?}"
    );
}

fn assert_normalized_range_near(actual: Option<NormalizedRange>, expected: (f32, f32)) {
    let actual = actual.expect("visible normalized range");
    assert!(
        (actual.start_fraction() - expected.0).abs() <= 0.000_001,
        "actual={actual:?} expected={expected:?}"
    );
    assert!(
        (actual.end_fraction() - expected.1).abs() <= 0.000_001,
        "actual={actual:?} expected={expected:?}"
    );
}

#[test]
fn index_viewport_clamps_visible_span_and_offset_fraction() {
    let viewport = IndexViewport {
        start: 990,
        end: 1_020,
    }
    .clamp(1_000, 128);

    assert_eq!(
        viewport,
        IndexViewport {
            start: 872,
            end: 1_000
        }
    );
    assert_eq!(viewport.visible_items(), 128);
    assert_eq!(viewport.visible_fraction(1_000), 0.128);
    assert_eq!(viewport.offset_fraction(1_000), 1.0);
}

#[test]
fn index_viewport_zooms_and_pans_around_visible_anchor() {
    let viewport = IndexViewport {
        start: 200,
        end: 600,
    };

    assert_eq!(
        viewport.zoom_around_anchor(1_000, 100, 0.5, 0.25),
        IndexViewport {
            start: 250,
            end: 450
        }
    );
    assert_eq!(
        viewport.pan_by_visible_fraction(1_000, 100, 0.5),
        IndexViewport {
            start: 400,
            end: 800
        }
    );
}

#[test]
fn index_viewport_pans_by_visible_ratio_drag() {
    let viewport = IndexViewport {
        start: 12_000,
        end: 36_000,
    };

    assert_eq!(
        viewport.pan_by_visible_ratio_drag(96_000, 256, 0.5, 0.25),
        IndexViewport {
            start: 18_000,
            end: 42_000,
        }
    );
    assert_eq!(
        viewport.pan_by_visible_ratio_drag(96_000, 256_000, 0.5, 0.75),
        IndexViewport {
            start: 0,
            end: 96_000,
        }
    );
}

#[test]
fn index_viewport_sets_offset_and_projects_visible_ratio() {
    let viewport = IndexViewport { start: 0, end: 250 }.with_offset_fraction(1_000, 100, 0.5);

    assert_eq!(
        viewport,
        IndexViewport {
            start: 375,
            end: 625
        }
    );
    assert_eq!(viewport.absolute_ratio_from_visible(1_000, 100, 0.5), 0.5);
}

#[test]
fn index_viewport_projects_absolute_ratio_into_visible_ratio() {
    let viewport = IndexViewport {
        start: 200,
        end: 600,
    };

    assert_ratio_near(viewport.visible_ratio_from_absolute(1_000, 0.2), 0.0);
    assert_ratio_near(viewport.visible_ratio_from_absolute(1_000, 0.4), 0.5);
    assert_ratio_near(viewport.visible_ratio_from_absolute(1_000, 0.6), 1.0);
    assert_eq!(viewport.visible_ratio_from_absolute(1_000, 0.7), None);
    assert_eq!(viewport.visible_ratio_from_absolute(1_000, f32::NAN), None);
}

#[test]
fn index_viewport_projects_and_clips_absolute_ranges() {
    let viewport = IndexViewport {
        start: 200,
        end: 600,
    };

    assert_range_near(
        viewport.visible_range_from_absolute(1_000, 0.25, 0.5),
        (0.125, 0.75),
    );
    assert_range_near(
        viewport.visible_range_from_absolute(1_000, 0.1, 0.3),
        (0.0, 0.25),
    );
    assert_range_near(
        viewport.visible_range_from_absolute(1_000, 0.5, 0.7),
        (0.75, 1.0),
    );
    assert_eq!(viewport.visible_range_from_absolute(1_000, 0.7, 0.8), None);
    assert_eq!(viewport.visible_range_from_absolute(1_000, 0.3, 0.3), None);
}

#[test]
fn index_viewport_projects_and_clips_normalized_ranges() {
    let viewport = IndexViewport {
        start: 200,
        end: 600,
    };

    assert_normalized_range_near(
        viewport.visible_normalized_range(1_000, NormalizedRange::from_fractions(0.25, 0.5)),
        (0.125, 0.75),
    );
    assert_normalized_range_near(
        viewport.visible_normalized_range(1_000, NormalizedRange::from_fractions(0.5, 0.7)),
        (0.75, 1.0),
    );
    assert_eq!(
        viewport.visible_normalized_range(1_000, NormalizedRange::from_fractions(0.7, 0.8)),
        None
    );
    assert_eq!(
        viewport.visible_normalized_range(1_000, NormalizedRange::from_fractions(0.3, 0.3)),
        None
    );
}

#[test]
fn index_viewport_scope_binds_total_and_min_visible_items() {
    let scope = IndexViewportScope::new(
        IndexViewport {
            start: 990,
            end: 1_020,
        },
        1_000,
        128,
    );

    assert_eq!(
        scope.viewport(),
        IndexViewport {
            start: 872,
            end: 1_000
        }
    );
    assert_eq!(scope.total_items(), 1_000);
    assert_eq!(scope.min_visible_items(), 128);
    assert_eq!(scope.visible_items(), 128);
    assert_eq!(scope.visible_fraction(), 0.128);
    assert_eq!(scope.offset_fraction(), 1.0);
    assert!(scope.is_zoomed_in());
    assert_eq!(
        scope.with_offset_fraction(0.0),
        IndexViewport { start: 0, end: 128 }
    );
}

#[test]
fn index_viewport_scope_projects_and_updates_without_repeated_domain_args() {
    let scope = IndexViewportScope::new(
        IndexViewport {
            start: 200,
            end: 600,
        },
        1_000,
        100,
    );

    assert_eq!(scope.absolute_ratio_from_visible(0.5), 0.4);
    assert_ratio_near(scope.visible_ratio_from_absolute(0.4), 0.5);
    assert_range_near(scope.visible_range_from_absolute(0.25, 0.5), (0.125, 0.75));
    assert_normalized_range_near(
        scope.visible_normalized_range(NormalizedRange::from_fractions(0.25, 0.5)),
        (0.125, 0.75),
    );
    assert_eq!(
        scope.zoom_around_anchor(0.5, 0.25),
        IndexViewport {
            start: 250,
            end: 450
        }
    );
    assert_eq!(
        scope.pan_by_visible_fraction(0.5),
        IndexViewport {
            start: 400,
            end: 800
        }
    );
    assert_eq!(
        scope.pan_by_visible_ratio_drag(0.5, 0.25),
        IndexViewport {
            start: 300,
            end: 700
        }
    );
}
