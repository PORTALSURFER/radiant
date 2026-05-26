use radiant::prelude::*;

use super::{DESTINATION_COUNT, MatrixCell, SOURCE_COUNT};

fn matrix_grid(matrix: Rect) -> DenseGridLayout {
    DenseGridLayout::new(matrix, SOURCE_COUNT, DESTINATION_COUNT)
}

fn dense_cell(cell: MatrixCell) -> DenseGridCell {
    DenseGridCell::new(cell.source, cell.destination)
}

fn matrix_cell(cell: DenseGridCell) -> MatrixCell {
    MatrixCell {
        source: cell.row,
        destination: cell.column,
    }
    .clamped()
}

pub(crate) fn source_label_rect(bounds: Rect, matrix: Rect, source: usize) -> Rect {
    DenseGridLabelLayout::new(matrix_grid(matrix))
        .row_label_rect(source_label_bounds(bounds, matrix), source)
        .unwrap_or_else(|| matrix.empty_at_min())
}

pub(crate) fn destination_label_rect(matrix: Rect, destination: usize) -> Rect {
    DenseGridLabelLayout::new(matrix_grid(matrix))
        .column_label_rect(destination_label_bounds(matrix), destination)
        .unwrap_or_else(|| matrix.empty_at_min())
}

pub(crate) fn cell_rect(matrix: Rect, cell: MatrixCell) -> Rect {
    matrix_grid(matrix)
        .cell_rect(dense_cell(cell.clamped()))
        .unwrap_or_else(|| matrix.empty_at_min())
}

pub(crate) fn cell_at_position(matrix: Rect, position: Point) -> Option<MatrixCell> {
    matrix_grid(matrix)
        .cell_at_position(position)
        .map(matrix_cell)
}

pub(crate) fn amount_for_position(rect: Rect, position: Point) -> f32 {
    vertical_bipolar_value_at_point(rect, position)
}

pub(crate) fn amount_bar_rect(rect: Rect, amount: f32) -> Option<Rect> {
    vertical_bipolar_fill_rect(rect, amount, 12.0, 0.44)
}

fn source_label_bounds(bounds: Rect, matrix: Rect) -> Rect {
    Rect::from_min_max(
        Point::new(bounds.min.x + 12.0, matrix.min.y),
        Point::new(matrix.min.x - 8.0, matrix.max.y),
    )
}

fn destination_label_bounds(matrix: Rect) -> Rect {
    Rect::from_min_max(
        Point::new(matrix.min.x, matrix.min.y - 38.0),
        Point::new(matrix.max.x, matrix.min.y - 6.0),
    )
}
