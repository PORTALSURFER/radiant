use super::super::{HorizontalStripLayout, HorizontalStripLayoutParts};
use crate::gui::types::{Point, Rect};

#[test]
fn horizontal_strip_layout_projects_gapped_strips() {
    let layout = HorizontalStripLayout::new(
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(230.0, 120.0)),
        4,
        4.0,
    );

    assert_eq!(layout.strip_width(), 52.0);
    assert_eq!(
        layout.strip_rect(2),
        Some(Rect::from_min_max(
            Point::new(122.0, 20.0),
            Point::new(174.0, 120.0)
        ))
    );
    assert_eq!(layout.strip_rect(4), None);
}

#[test]
fn horizontal_strip_layout_hit_tests_visible_strips() {
    let layout = HorizontalStripLayout::from_parts(HorizontalStripLayoutParts::new(
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(230.0, 120.0)),
        4,
        4.0,
    ));

    assert_eq!(layout.strip_at_position(Point::new(35.0, 60.0)), Some(0));
    assert_eq!(layout.strip_at_position(Point::new(120.0, 60.0)), None);
    assert_eq!(layout.strip_at_position(Point::new(130.0, 60.0)), Some(2));
    assert_eq!(layout.strip_at_position(Point::new(231.0, 60.0)), None);
}

#[test]
fn horizontal_strip_layout_resolves_insertion_geometry() {
    let layout = HorizontalStripLayout::new(
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(230.0, 120.0)),
        4,
        4.0,
    );

    assert_eq!(layout.insertion_index_at(Point::new(10.0, 60.0)), 0);
    assert_eq!(layout.insertion_index_at(Point::new(75.0, 60.0)), 1);
    assert_eq!(layout.insertion_index_at(Point::new(230.0, 60.0)), 4);
    assert_eq!(
        layout.insertion_line_rect(2, 4.0, 6.0),
        Some(Rect::from_min_max(
            Point::new(118.0, 26.0),
            Point::new(122.0, 114.0)
        ))
    );
}

#[test]
fn horizontal_strip_layout_rejects_invalid_or_cramped_geometry() {
    let rect = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(20.0, 100.0));

    assert!(!HorizontalStripLayout::new(rect, 0, 4.0).is_valid());
    assert!(!HorizontalStripLayout::new(rect.empty_at_min(), 2, 4.0).is_valid());
    assert!(!HorizontalStripLayout::new(rect, 2, 40.0).is_valid());
    assert_eq!(HorizontalStripLayout::new(rect, 2, f32::NAN).gap(), 0.0);
}
