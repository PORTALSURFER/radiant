use super::{super::super::*, fixtures::timeline_viewport_parts};
use crate::gui::{
    range::{NormalizedPixelSnap, NormalizedRange, NormalizedViewport},
    types::{Point, Rect},
};

#[test]
fn timeline_coordinate_mapper_projects_and_back_projects_micros() {
    let viewport = TimelineViewport::from_parts(timeline_viewport_parts(
        250,
        750,
        250_000,
        750_000,
        250_000_000,
        750_000_000,
    ));
    let rect = Rect::from_min_max(Point::new(10.0, 0.0), Point::new(210.0, 40.0));
    let mapper = TimelineCoordinateMapper::new(viewport, rect, NormalizedPixelSnap::Nearest);

    assert_eq!(
        viewport.normalized_viewport(),
        NormalizedViewport::from_micros(250_000, 750_000)
    );
    assert_eq!(mapper.x_for_micros(250_000), 10.0);
    assert_eq!(mapper.x_for_micros(500_000), 110.0);
    assert_eq!(
        mapper.x_range_for(NormalizedRange::from_micros(300_000, 700_000)),
        (30.0, 190.0)
    );
    assert_eq!(mapper.micros_for_x(110.0), 500_000);
}

#[test]
fn timeline_coordinate_mapper_sanitizes_invalid_back_projection_inputs() {
    let viewport = TimelineViewport::from_parts(timeline_viewport_parts(
        250,
        750,
        250_000,
        750_000,
        250_000_000,
        750_000_000,
    ));
    let rect = Rect::from_min_max(Point::new(10.0, 0.0), Point::new(210.0, 40.0));
    let mapper = TimelineCoordinateMapper::new(viewport, rect, NormalizedPixelSnap::Nearest);
    let invalid_rect_mapper = TimelineCoordinateMapper::new(
        viewport,
        Rect::from_min_max(Point::new(f32::NAN, 0.0), Point::new(210.0, 40.0)),
        NormalizedPixelSnap::Nearest,
    );

    assert_eq!(mapper.micros_for_x(f32::NAN), 250_000);
    assert_eq!(mapper.micros_for_x(f32::INFINITY), 250_000);
    assert_eq!(invalid_rect_mapper.micros_for_x(110.0), 250_000);
}
