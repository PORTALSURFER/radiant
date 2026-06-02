use crate::gui::{
    range::{NormalizedPixelSnap, NormalizedRange},
    types::{Point, Rect, Vector2},
    visualization::{
        TimelineCoordinateMapper, TimelineEditHandle, TimelineEditHandleGeometry,
        TimelineEditPreview, TimelineEditPreviewParts, TimelineEditRegion,
        TimelineEditRegionGeometry, TimelineViewport,
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
    TimelineEditHandleGeometry {
        bounds: mapper.rect,
        selection_rect: preview()
            .selection_rect(mapper)
            .expect("visible selection rect"),
        handle_size: 10.0,
    }
}

fn region_geometry() -> TimelineEditRegionGeometry {
    let mapper = mapper();
    TimelineEditRegionGeometry {
        bounds: mapper.rect,
        selection_rect: preview()
            .selection_rect(mapper)
            .expect("visible selection rect"),
    }
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

#[test]
fn timeline_edit_preview_projects_standard_handle_rects() {
    let preview = preview();
    let mapper = mapper();
    let geometry = geometry();

    assert_rect_near(
        preview.handle_rect(mapper, geometry, TimelineEditHandle::LeadingEnd),
        Rect::from_min_max(Point::new(55.0, 0.0), Point::new(65.0, 10.0)),
    );
    assert_rect_near(
        preview.handle_rect(mapper, geometry, TimelineEditHandle::TrailingEnd),
        Rect::from_min_max(Point::new(115.0, 70.0), Point::new(120.0, 80.0)),
    );
    assert_rect_near(
        preview.handle_rect(mapper, geometry, TimelineEditHandle::TrailingOuterEnd),
        Rect::from_min_max(Point::new(135.0, 35.0), Point::new(145.0, 45.0)),
    );
}

#[test]
fn timeline_edit_preview_projects_standard_region_rects() {
    let preview = preview();
    let mapper = mapper();
    let geometry = region_geometry();

    assert_rect_near(
        preview.region_rect(mapper, geometry, TimelineEditRegion::LeadingInner),
        Rect::from_min_max(Point::new(40.0, 0.0), Point::new(60.0, 80.0)),
    );
    assert_rect_near(
        preview.region_rect(mapper, geometry, TimelineEditRegion::TrailingInner),
        Rect::from_min_max(Point::new(100.0, 0.0), Point::new(120.0, 80.0)),
    );
    assert_rect_near(
        preview.region_rect(mapper, geometry, TimelineEditRegion::LeadingOuter),
        Rect::from_min_max(Point::new(20.0, 0.0), Point::new(40.0, 80.0)),
    );
    assert_rect_near(
        preview.region_rect(mapper, geometry, TimelineEditRegion::TrailingOuter),
        Rect::from_min_max(Point::new(120.0, 0.0), Point::new(140.0, 80.0)),
    );
}

#[test]
fn timeline_edit_preview_hit_tests_outer_handles_outside_selection_rect() {
    let preview = preview();
    let mapper = mapper();
    let geometry = geometry();

    assert_eq!(
        preview.handle_at(
            mapper,
            geometry,
            [TimelineEditHandle::TrailingOuterEnd],
            Point::new(140.0, 40.0),
        ),
        Some(TimelineEditHandle::TrailingOuterEnd)
    );
}

#[test]
fn timeline_edit_preview_omits_handles_outside_viewport() {
    let mapper = TimelineCoordinateMapper::new(
        TimelineViewport::new(0, 650, 0, 650_000, 0, 650_000_000),
        bounds(),
        NormalizedPixelSnap::None,
    );
    let geometry = TimelineEditHandleGeometry {
        bounds: mapper.rect,
        selection_rect: preview()
            .selection_rect(mapper)
            .expect("visible selection rect"),
        handle_size: 10.0,
    };

    assert_eq!(
        preview().handle_rect(mapper, geometry, TimelineEditHandle::TrailingOuterEnd),
        None
    );
}
