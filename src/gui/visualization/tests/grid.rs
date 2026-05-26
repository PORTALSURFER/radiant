use super::super::{
    DenseGridCell, DenseGridLabelLayout, DenseGridLayout, DenseGridLayoutParts,
    DenseGridRasterLayout, DenseGridRasterLayoutParts, DenseGridRowOrigin,
};
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

#[test]
fn dense_grid_label_layout_projects_row_and_column_gutters() {
    let grid = DenseGridLayout::new(
        Rect::from_min_max(Point::new(100.0, 50.0), Point::new(300.0, 170.0)),
        3,
        4,
    );
    let labels = DenseGridLabelLayout::new(grid);

    assert_eq!(
        labels.row_label_rect(
            Rect::from_min_max(Point::new(20.0, 50.0), Point::new(92.0, 170.0)),
            1,
        ),
        Some(Rect::from_min_max(
            Point::new(20.0, 90.0),
            Point::new(92.0, 130.0)
        ))
    );
    assert_eq!(
        labels.column_label_rect(
            Rect::from_min_max(Point::new(100.0, 12.0), Point::new(300.0, 44.0)),
            2,
        ),
        Some(Rect::from_min_max(
            Point::new(200.0, 12.0),
            Point::new(250.0, 44.0)
        ))
    );
    assert_eq!(
        labels.row_label_rect(Rect::from_min_max(grid.rect.min, grid.rect.max), 3),
        None
    );
}

#[test]
fn dense_grid_raster_layout_projects_bottom_up_cells_with_bleed() {
    let layout = DenseGridRasterLayout::bottom_up(
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0)),
        4,
        2,
    )
    .with_horizontal_bleed(0.5)
    .with_vertical_bleed(0.5);

    assert_eq!(
        layout.cell_rect(DenseGridCell::new(0, 0)),
        Some(Rect::from_min_max(
            Point::new(10.0, 94.5),
            Point::new(110.5, 120.0)
        ))
    );
    assert_eq!(
        layout.cell_rect(DenseGridCell::new(3, 1)),
        Some(Rect::from_min_max(
            Point::new(110.0, 20.0),
            Point::new(210.0, 45.0)
        ))
    );
}

#[test]
fn dense_grid_raster_layout_sanitizes_bleed_and_rejects_invalid_cells() {
    let grid = DenseGridLayout::from_parts(DenseGridLayoutParts::new(
        Rect::from_min_max(Point::new(0.0, 0.0), Point::new(40.0, 40.0)),
        2,
        2,
    ));
    let layout = DenseGridRasterLayout::from_parts(DenseGridRasterLayoutParts {
        grid,
        row_origin: DenseGridRowOrigin::Top,
        horizontal_bleed: f32::NAN,
        vertical_bleed: -2.0,
    });

    assert_eq!(
        layout.cell_rect(DenseGridCell::new(0, 0)),
        Some(Rect::from_min_max(
            Point::new(0.0, 0.0),
            Point::new(20.0, 20.0)
        ))
    );
    assert_eq!(layout.cell_rect(DenseGridCell::new(2, 0)), None);
}
