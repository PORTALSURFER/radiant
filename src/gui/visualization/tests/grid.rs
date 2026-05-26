use super::super::{DenseGridCell, DenseGridLayout, DenseGridLayoutParts};
use crate::gui::types::{Point, Rect};

#[test]
fn dense_grid_layout_projects_cell_rects() {
    let layout = DenseGridLayout::new(
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 140.0)),
        3,
        4,
    );

    assert_eq!(
        layout.cell_rect(DenseGridCell::new(1, 2)),
        Some(Rect::from_min_max(
            Point::new(110.0, 60.0),
            Point::new(160.0, 100.0)
        ))
    );
    assert_eq!(layout.cell_rect(DenseGridCell::new(3, 0)), None);
}

#[test]
fn dense_grid_layout_hit_tests_points_and_clamps_outer_edge() {
    let layout = DenseGridLayout::from_parts(DenseGridLayoutParts {
        rect: Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 140.0)),
        rows: 3,
        columns: 4,
    });

    assert_eq!(
        layout.cell_at_position(Point::new(111.0, 61.0)),
        Some(DenseGridCell::new(1, 2))
    );
    assert_eq!(
        layout.cell_at_position(Point::new(210.0, 140.0)),
        Some(DenseGridCell::new(2, 3))
    );
    assert_eq!(layout.cell_at_position(Point::new(211.0, 140.0)), None);
}

#[test]
fn dense_grid_layout_rejects_empty_or_invalid_geometry() {
    let rect = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
    assert!(!DenseGridLayout::new(rect, 0, 4).is_valid());
    assert!(!DenseGridLayout::new(rect, 4, 0).is_valid());
    assert!(!DenseGridLayout::new(rect.empty_at_min(), 4, 4).is_valid());
}
