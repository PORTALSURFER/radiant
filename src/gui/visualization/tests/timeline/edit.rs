use crate::gui::{
    range::{NormalizedPixelSnap, NormalizedRange},
    types::{Point, Rect, Rgba8, Vector2},
    visualization::{
        TimelineCoordinateMapper, TimelineEditHandle, TimelineEditHandleGeometry,
        TimelineEditPreview, TimelineEditPreviewParts, TimelineEditRamp, TimelineEditRegion,
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

fn fill_rects(primitives: &[crate::runtime::PaintPrimitive]) -> Vec<(u64, Rect, Rgba8)> {
    primitives
        .iter()
        .filter_map(|primitive| primitive.fill_rect())
        .map(|fill| (fill.widget_id, fill.rect, fill.color))
        .collect()
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
fn timeline_edit_preview_pushes_standard_region_fills() {
    let mut primitives = Vec::new();
    let region_color = |region| match region {
        TimelineEditRegion::LeadingInner | TimelineEditRegion::TrailingInner => {
            Rgba8::new(20, 40, 60, 180)
        }
        TimelineEditRegion::LeadingOuter | TimelineEditRegion::TrailingOuter => {
            Rgba8::new(20, 40, 60, 96)
        }
    };

    preview().push_standard_region_fills(
        &mut primitives,
        7,
        mapper(),
        region_geometry(),
        region_color,
    );

    let fills = fill_rects(&primitives);
    assert_eq!(fills.len(), 4);
    assert_eq!(fills[0].0, 7);
    assert_rect_near(
        Some(fills[0].1),
        Rect::from_min_max(Point::new(40.0, 0.0), Point::new(60.0, 80.0)),
    );
    assert_eq!(fills[0].2, Rgba8::new(20, 40, 60, 180));
    assert_rect_near(
        Some(fills[2].1),
        Rect::from_min_max(Point::new(20.0, 0.0), Point::new(40.0, 80.0)),
    );
    assert_eq!(fills[2].2, Rgba8::new(20, 40, 60, 96));
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
fn timeline_edit_preview_pushes_standard_handle_fills() {
    let mut primitives = Vec::new();
    let handle_color = |handle| match handle {
        TimelineEditHandle::LeadingEnd | TimelineEditHandle::TrailingStart => {
            Rgba8::new(90, 120, 240, 220)
        }
        _ => Rgba8::new(90, 120, 240, 160),
    };

    preview().push_standard_handle_fills(&mut primitives, 9, mapper(), geometry(), handle_color);

    let fills = fill_rects(&primitives);
    assert_eq!(fills.len(), 6);
    assert_eq!(fills[0].0, 9);
    assert_rect_near(
        Some(fills[0].1),
        Rect::from_min_max(Point::new(55.0, 0.0), Point::new(65.0, 10.0)),
    );
    assert_eq!(fills[0].2, Rgba8::new(90, 120, 240, 220));
    assert_rect_near(
        Some(fills[4].1),
        Rect::from_min_max(Point::new(15.0, 35.0), Point::new(25.0, 45.0)),
    );
    assert_eq!(fills[4].2, Rgba8::new(90, 120, 240, 160));
}

#[test]
fn timeline_edit_handle_standard_order_prioritizes_inner_handles() {
    assert_eq!(
        TimelineEditHandle::standard_order(),
        [
            TimelineEditHandle::LeadingEnd,
            TimelineEditHandle::TrailingStart,
            TimelineEditHandle::LeadingStart,
            TimelineEditHandle::TrailingEnd,
            TimelineEditHandle::LeadingOuterStart,
            TimelineEditHandle::TrailingOuterEnd,
        ]
    );
}

#[test]
fn timeline_edit_preview_standard_handle_at_uses_standard_priority() {
    let preview = TimelineEditPreview::from_parts(TimelineEditPreviewParts {
        selection: Some(NormalizedRange::from_micros(200_000, 600_000)),
        leading_end_micros: Some(200_000),
        ..TimelineEditPreviewParts::default()
    });
    let mapper = mapper();
    let geometry = preview
        .handle_geometry(mapper, 10.0)
        .expect("visible handle geometry");

    assert_eq!(
        preview.standard_handle_at(mapper, geometry, Point::new(40.0, 5.0)),
        Some(TimelineEditHandle::LeadingEnd)
    );
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
fn timeline_edit_preview_builds_from_normalized_ramps() {
    let preview = TimelineEditPreview::from_normalized_ramps(
        NormalizedRange::from_micros(200_000, 600_000),
        Some(TimelineEditRamp::new(0.25, 0.5, Some(0.35))),
        Some(TimelineEditRamp::new(0.5, 0.25, Some(0.65))),
    );

    assert_eq!(
        preview.selection,
        Some(NormalizedRange::from_micros(200_000, 600_000))
    );
    assert_eq!(preview.leading_end_micros, Some(300_000));
    assert_eq!(preview.leading_inner_start_micros, Some(0));
    assert_eq!(preview.leading_curve_milli, Some(350));
    assert_eq!(preview.trailing_start_micros, Some(400_000));
    assert_eq!(preview.trailing_inner_end_micros, Some(700_000));
    assert_eq!(preview.trailing_curve_milli, Some(650));
}

#[test]
fn timeline_edit_ramp_from_length_has_no_outer_extension() {
    let preview = TimelineEditPreview::from_normalized_ramps(
        NormalizedRange::from_micros(200_000, 600_000),
        Some(TimelineEditRamp::from_length(0.25, None)),
        None,
    );

    assert_eq!(preview.leading_end_micros, Some(300_000));
    assert_eq!(preview.leading_inner_start_micros, Some(200_000));
    assert_eq!(preview.leading_curve_milli, None);
    assert_eq!(preview.trailing_start_micros, None);
}

#[test]
fn timeline_edit_handle_geometry_clamps_size_to_bounds() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(6.0, 4.0));
    let geometry = TimelineEditHandleGeometry::new(bounds, bounds, 10.0);

    assert_eq!(geometry.clamped_handle_size(), 4.0);
}
