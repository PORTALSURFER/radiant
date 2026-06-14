mod geometry;
mod hit_testing;
mod paint;
mod ramps;

use crate::gui::{
    range::{NormalizedPixelSnap, NormalizedRange},
    types::{Point, Rect, Vector2},
    visualization::{
        TimelineCoordinateMapper, TimelineEditHandleGeometry, TimelineEditPreview,
        TimelineEditPreviewParts, TimelineEditRegionGeometry, TimelineViewport,
    },
};

fn bounds() -> Rect {
    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 80.0))
}

fn mapper() -> TimelineCoordinateMapper {
    TimelineCoordinateMapper::new(
        TimelineViewport::default(),
        bounds(),
        NormalizedPixelSnap::None,
    )
}

fn preview() -> TimelineEditPreview {
    TimelineEditPreview::from_parts(TimelineEditPreviewParts {
        selection: Some(NormalizedRange::from_micros(200_000, 600_000)),
        leading_end_micros: Some(300_000),
        leading_inner_start_micros: Some(100_000),
        trailing_start_micros: Some(500_000),
        trailing_inner_end_micros: Some(700_000),
        ..TimelineEditPreviewParts::default()
    })
}

fn geometry() -> TimelineEditHandleGeometry {
    let mapper = mapper();
    preview()
        .handle_geometry(mapper, 10.0)
        .expect("visible handle geometry")
}

fn region_geometry() -> TimelineEditRegionGeometry {
    let mapper = mapper();
    preview()
        .region_geometry(mapper)
        .expect("visible region geometry")
}

fn assert_rect_near(actual: Option<Rect>, expected: Rect) {
    let actual = actual.expect("projected rect");
    let epsilon = 0.001;
    assert!(
        (actual.min.x - expected.min.x).abs() <= epsilon,
        "{actual:?}"
    );
    assert!(
        (actual.min.y - expected.min.y).abs() <= epsilon,
        "{actual:?}"
    );
    assert!(
        (actual.max.x - expected.max.x).abs() <= epsilon,
        "{actual:?}"
    );
    assert!(
        (actual.max.y - expected.max.y).abs() <= epsilon,
        "{actual:?}"
    );
}
