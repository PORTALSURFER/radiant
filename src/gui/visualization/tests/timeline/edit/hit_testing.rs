use super::*;
use crate::gui::visualization::TimelineEditHandle;

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
