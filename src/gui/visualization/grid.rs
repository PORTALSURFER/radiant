use crate::gui::types::{Point, Rect, Vector2};

/// Logical row/column address inside a dense visualization grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DenseGridCell {
    /// Zero-based row index.
    pub row: usize,
    /// Zero-based column index.
    pub column: usize,
}

impl DenseGridCell {
    /// Create a dense-grid cell address.
    pub const fn new(row: usize, column: usize) -> Self {
        Self { row, column }
    }
}

/// Named fields for constructing dense visualization grid geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DenseGridLayoutParts {
    /// Rect used as the grid projection area.
    pub rect: Rect,
    /// Number of logical rows in the grid.
    pub rows: usize,
    /// Number of logical columns in the grid.
    pub columns: usize,
}

/// Reusable geometry projector for dense row/column visualization grids.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DenseGridLayout {
    /// Rect used as the grid projection area.
    pub rect: Rect,
    /// Number of logical rows in the grid.
    pub rows: usize,
    /// Number of logical columns in the grid.
    pub columns: usize,
}

impl DenseGridLayout {
    /// Create dense grid geometry from a rect and logical row/column count.
    pub const fn new(rect: Rect, rows: usize, columns: usize) -> Self {
        Self {
            rect,
            rows,
            columns,
        }
    }

    /// Create dense grid geometry from named parts.
    pub const fn from_parts(parts: DenseGridLayoutParts) -> Self {
        Self {
            rect: parts.rect,
            rows: parts.rows,
            columns: parts.columns,
        }
    }

    /// Return whether this grid can produce finite cell geometry.
    pub fn is_valid(self) -> bool {
        self.rows > 0 && self.columns > 0 && self.rect.has_finite_positive_area()
    }

    /// Return the rect for a logical cell.
    pub fn cell_rect(self, cell: DenseGridCell) -> Option<Rect> {
        if !self.is_valid() || cell.row >= self.rows || cell.column >= self.columns {
            return None;
        }
        let width = self.rect.width() / self.columns as f32;
        let height = self.rect.height() / self.rows as f32;
        Some(Rect::from_min_size(
            Point::new(
                self.rect.min.x + cell.column as f32 * width,
                self.rect.min.y + cell.row as f32 * height,
            ),
            Vector2::new(width, height),
        ))
    }

    /// Return the logical cell containing a point.
    pub fn cell_at_position(self, position: Point) -> Option<DenseGridCell> {
        if !self.is_valid() || !self.rect.contains(position) {
            return None;
        }
        let cell_width = self.rect.width() / self.columns as f32;
        let cell_height = self.rect.height() / self.rows as f32;
        let column = ((position.x - self.rect.min.x) / cell_width) as usize;
        let row = ((position.y - self.rect.min.y) / cell_height) as usize;
        Some(DenseGridCell {
            row: row.min(self.rows - 1),
            column: column.min(self.columns - 1),
        })
    }
}
