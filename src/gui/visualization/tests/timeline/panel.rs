use super::super::super::TimelinePanelLayout;
use crate::gui::types::{Point, Rect};

#[test]
fn timeline_panel_layout_splits_header_ruler_and_lanes() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(310.0, 220.0));

    let layout = TimelinePanelLayout::new(bounds, 64.0, 24.0);

    assert_eq!(
        layout.header,
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(74.0, 220.0))
    );
    assert_eq!(
        layout.ruler,
        Rect::from_min_max(Point::new(74.0, 20.0), Point::new(310.0, 44.0))
    );
    assert_eq!(
        layout.lanes,
        Rect::from_min_max(Point::new(74.0, 44.0), Point::new(310.0, 220.0))
    );
}

#[test]
fn timeline_panel_layout_saturates_invalid_or_oversized_chrome() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 70.0));

    let layout = TimelinePanelLayout::new(bounds, f32::INFINITY, 80.0);

    assert_eq!(
        layout.header,
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(10.0, 70.0))
    );
    assert_eq!(
        layout.ruler,
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 70.0))
    );
    assert_eq!(
        layout.lanes,
        Rect::from_min_max(Point::new(10.0, 70.0), Point::new(110.0, 70.0))
    );
}
