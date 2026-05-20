use super::{NormalizedPixelSnap, NormalizedViewport, NormalizedViewportParts};
use crate::gui::types::{Point, Rect};

#[test]
fn normalized_viewport_projects_absolute_ratios_into_rect() {
    let rect = Rect::from_min_max(Point::new(10.0, 0.0), Point::new(110.0, 20.0));
    let viewport = NormalizedViewport::from_micros(250_000, 750_000);

    assert_eq!(
        viewport.x_for_ratio(rect, 0.25, NormalizedPixelSnap::Nearest),
        10.0
    );
    assert_eq!(
        viewport.x_for_ratio(rect, 0.5, NormalizedPixelSnap::Nearest),
        60.0
    );
    assert_eq!(
        viewport.x_for_ratio(rect, 0.75, NormalizedPixelSnap::Nearest),
        110.0
    );
}

#[test]
fn normalized_viewport_projection_sanitizes_invalid_inputs() {
    let rect = Rect::from_min_max(Point::new(10.0, 0.0), Point::new(110.0, 20.0));
    let viewport = NormalizedViewport::from_micros(250_000, 750_000);

    assert_eq!(
        viewport.x_for_ratio(rect, f64::NAN, NormalizedPixelSnap::Nearest),
        10.0
    );
    assert_eq!(
        viewport.x_for_ratio(
            Rect::from_min_max(Point::new(f32::NAN, 0.0), Point::new(110.0, 20.0)),
            0.5,
            NormalizedPixelSnap::Nearest
        ),
        0.0
    );
    assert_eq!(
        viewport.x_for_ratio(
            Rect::from_min_max(Point::new(10.0, 0.0), Point::new(f32::INFINITY, 20.0)),
            0.5,
            NormalizedPixelSnap::Nearest
        ),
        0.0
    );
}

#[test]
fn normalized_viewport_uses_nanos_only_when_they_match_micro_mirrors() {
    let viewport =
        NormalizedViewport::from_bounds(500_123, 500_124, Some(500_123_000), Some(500_123_200));

    assert_eq!(viewport.start_ratio, 0.500123);
    assert!((viewport.width_ratio - 0.0000002).abs() < f64::EPSILON);

    let fallback =
        NormalizedViewport::from_bounds(500_123, 500_124, Some(400_000_000), Some(400_100_000));

    assert_eq!(fallback, NormalizedViewport::from_micros(500_123, 500_124));
}

#[test]
fn normalized_viewport_supports_named_parts_construction() {
    let viewport = NormalizedViewport::from_parts(NormalizedViewportParts {
        start_micros: 500_123,
        end_micros: 500_124,
        start_nanos: Some(500_123_000),
        end_nanos: Some(500_123_200),
    });

    assert_eq!(viewport.start_ratio, 0.500123);
    assert!((viewport.width_ratio - 0.0000002).abs() < f64::EPSILON);
}
