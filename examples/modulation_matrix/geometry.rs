use radiant::prelude::*;

use super::{DESTINATION_COUNT, MatrixCell, SOURCE_COUNT};

pub(crate) fn source_label_rect(bounds: Rect, matrix: Rect, cell: Rect) -> Rect {
    Rect::from_min_max(
        Point::new(bounds.min.x + 12.0, cell.min.y),
        Point::new(matrix.min.x - 8.0, cell.max.y),
    )
}

pub(crate) fn destination_label_rect(matrix: Rect, cell: Rect) -> Rect {
    Rect::from_min_max(
        Point::new(cell.min.x, matrix.min.y - 38.0),
        Point::new(cell.max.x, matrix.min.y - 6.0),
    )
}

pub(crate) fn cell_rect(matrix: Rect, cell: MatrixCell) -> Rect {
    let width = matrix.width() / DESTINATION_COUNT as f32;
    let height = matrix.height() / SOURCE_COUNT as f32;
    let x = matrix.min.x + cell.destination as f32 * width;
    let y = matrix.min.y + cell.source as f32 * height;
    Rect::from_min_size(Point::new(x, y), Vector2::new(width, height))
}

pub(crate) fn cell_at_position(matrix: Rect, position: Point) -> Option<MatrixCell> {
    if !matrix.contains(position) {
        return None;
    }
    let destination =
        ((position.x - matrix.min.x) / (matrix.width() / DESTINATION_COUNT as f32)) as usize;
    let source = ((position.y - matrix.min.y) / (matrix.height() / SOURCE_COUNT as f32)) as usize;
    Some(
        MatrixCell {
            source,
            destination,
        }
        .clamped(),
    )
}

pub(crate) fn amount_for_position(rect: Rect, position: Point) -> f32 {
    let ratio = ((rect.max.y - position.y) / rect.height().max(1.0)).clamp(0.0, 1.0);
    ratio * 2.0 - 1.0
}

pub(crate) fn amount_bar_rect(rect: Rect, amount: f32) -> Rect {
    let center = rect.center().y;
    let available = rect.height() * 0.44;
    if amount >= 0.0 {
        Rect::from_min_max(
            Point::new(rect.min.x + 12.0, center - available * amount),
            Point::new(rect.max.x - 12.0, center),
        )
    } else {
        Rect::from_min_max(
            Point::new(rect.min.x + 12.0, center),
            Point::new(rect.max.x - 12.0, center + available * amount.abs()),
        )
    }
}
