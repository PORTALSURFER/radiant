use super::*;
use crate::gui::visualization::{TimelineEditHandle, TimelineEditRegion};

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
fn timeline_edit_preview_iterates_standard_region_rects() {
    let rects = preview()
        .standard_region_rects(mapper(), region_geometry())
        .collect::<Vec<_>>();

    let expected = [
        (
            TimelineEditRegion::LeadingInner,
            Rect::from_min_max(Point::new(40.0, 0.0), Point::new(60.0, 80.0)),
        ),
        (
            TimelineEditRegion::TrailingInner,
            Rect::from_min_max(Point::new(100.0, 0.0), Point::new(120.0, 80.0)),
        ),
        (
            TimelineEditRegion::LeadingOuter,
            Rect::from_min_max(Point::new(20.0, 0.0), Point::new(40.0, 80.0)),
        ),
        (
            TimelineEditRegion::TrailingOuter,
            Rect::from_min_max(Point::new(120.0, 0.0), Point::new(140.0, 80.0)),
        ),
    ];
    assert_eq!(rects.len(), expected.len());
    for ((actual_region, actual_rect), (expected_region, expected_rect)) in
        rects.into_iter().zip(expected)
    {
        assert_eq!(actual_region, expected_region);
        assert_rect_near(Some(actual_rect), expected_rect);
    }
}

#[test]
fn timeline_edit_preview_iterates_standard_handle_rects() {
    let rects = preview()
        .standard_handle_rects(mapper(), geometry())
        .collect::<Vec<_>>();

    let expected = [
        (
            TimelineEditHandle::LeadingEnd,
            Rect::from_min_max(Point::new(55.0, 0.0), Point::new(65.0, 10.0)),
        ),
        (
            TimelineEditHandle::TrailingStart,
            Rect::from_min_max(Point::new(95.0, 0.0), Point::new(105.0, 10.0)),
        ),
        (
            TimelineEditHandle::LeadingStart,
            Rect::from_min_max(Point::new(40.0, 70.0), Point::new(45.0, 80.0)),
        ),
        (
            TimelineEditHandle::TrailingEnd,
            Rect::from_min_max(Point::new(115.0, 70.0), Point::new(120.0, 80.0)),
        ),
        (
            TimelineEditHandle::LeadingOuterStart,
            Rect::from_min_max(Point::new(15.0, 35.0), Point::new(25.0, 45.0)),
        ),
        (
            TimelineEditHandle::TrailingOuterEnd,
            Rect::from_min_max(Point::new(135.0, 35.0), Point::new(145.0, 45.0)),
        ),
    ];
    assert_eq!(rects.len(), expected.len());
    for ((actual_handle, actual_rect), (expected_handle, expected_rect)) in
        rects.into_iter().zip(expected)
    {
        assert_eq!(actual_handle, expected_handle);
        assert_rect_near(Some(actual_rect), expected_rect);
    }
}

#[test]
fn timeline_edit_preview_omits_handles_outside_viewport() {
    let mapper = TimelineCoordinateMapper::new(
        TimelineViewport::new(0, 650, 0, 650_000, 0, 650_000_000),
        bounds(),
        NormalizedPixelSnap::None,
    );
    let geometry = preview()
        .handle_geometry(mapper, 10.0)
        .expect("visible handle geometry");

    assert_eq!(
        preview().handle_rect(mapper, geometry, TimelineEditHandle::TrailingOuterEnd),
        None
    );
}

#[test]
fn timeline_edit_preview_builds_standard_geometry_from_visible_selection() {
    let mapper = mapper();
    let handle_geometry = preview()
        .handle_geometry(mapper, 10.0)
        .expect("visible handle geometry");
    let region_geometry = preview()
        .region_geometry(mapper)
        .expect("visible region geometry");

    assert_eq!(handle_geometry.bounds, mapper.rect);
    assert_eq!(
        handle_geometry.selection_rect,
        region_geometry.selection_rect
    );
    assert_eq!(handle_geometry.handle_size, 10.0);
    assert_eq!(handle_geometry.clamped_handle_size(), 10.0);
}

#[test]
fn timeline_edit_handle_geometry_clamps_size_to_bounds() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(6.0, 4.0));
    let geometry = TimelineEditHandleGeometry::new(bounds, bounds, 10.0);

    assert_eq!(geometry.clamped_handle_size(), 4.0);
}
