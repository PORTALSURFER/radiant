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

impl DenseGridLayoutParts {
    /// Build dense grid layout parts from a rect and logical row/column count.
    pub const fn new(rect: Rect, rows: usize, columns: usize) -> Self {
        Self {
            rect,
            rows,
            columns,
        }
    }
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

/// Named fields for dense grid row and column label projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DenseGridLabelLayoutParts {
    /// Base dense grid projection.
    pub grid: DenseGridLayout,
}

impl DenseGridLabelLayoutParts {
    /// Build dense grid label layout parts.
    pub const fn new(grid: DenseGridLayout) -> Self {
        Self { grid }
    }
}

/// Reusable label geometry for row/column labels around dense visualization grids.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DenseGridLabelLayout {
    /// Base dense grid projection.
    pub grid: DenseGridLayout,
}

/// Vertical row ordering for dense raster-style grid projection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DenseGridRowOrigin {
    /// Logical row zero starts at the top of the grid.
    #[default]
    Top,
    /// Logical row zero starts at the bottom of the grid.
    Bottom,
}

/// Named fields for dense raster cell geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DenseGridRasterLayoutParts {
    /// Base dense grid projection.
    pub grid: DenseGridLayout,
    /// Vertical ordering for logical rows.
    pub row_origin: DenseGridRowOrigin,
    /// Extra horizontal coverage added to non-terminal cells to avoid raster seams.
    pub horizontal_bleed: f32,
    /// Extra vertical coverage added to non-terminal cells to avoid raster seams.
    pub vertical_bleed: f32,
}

impl DenseGridRasterLayoutParts {
    /// Build dense raster layout parts without seam bleed.
    pub const fn new(grid: DenseGridLayout) -> Self {
        Self {
            grid,
            row_origin: DenseGridRowOrigin::Top,
            horizontal_bleed: 0.0,
            vertical_bleed: 0.0,
        }
    }
}

/// Reusable dense raster cell projection for heatmaps, spectrograms, and matrices.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DenseGridRasterLayout {
    /// Base dense grid projection.
    pub grid: DenseGridLayout,
    /// Vertical ordering for logical rows.
    pub row_origin: DenseGridRowOrigin,
    /// Extra horizontal coverage added to non-terminal cells to avoid raster seams.
    pub horizontal_bleed: f32,
    /// Extra vertical coverage added to non-terminal cells to avoid raster seams.
    pub vertical_bleed: f32,
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

impl DenseGridLabelLayout {
    /// Build dense grid label layout from named parts.
    pub const fn from_parts(parts: DenseGridLabelLayoutParts) -> Self {
        Self { grid: parts.grid }
    }

    /// Build dense grid label layout.
    pub const fn new(grid: DenseGridLayout) -> Self {
        Self::from_parts(DenseGridLabelLayoutParts::new(grid))
    }

    /// Return the rect for one row label inside a caller-provided label gutter.
    pub fn row_label_rect(self, label_bounds: Rect, row: usize) -> Option<Rect> {
        let cell = self.grid.cell_rect(DenseGridCell::new(row, 0))?;
        Some(Rect::from_min_max(
            Point::new(label_bounds.min.x, cell.min.y),
            Point::new(label_bounds.max.x, cell.max.y),
        ))
    }

    /// Return the rect for one column label inside a caller-provided label gutter.
    pub fn column_label_rect(self, label_bounds: Rect, column: usize) -> Option<Rect> {
        let cell = self.grid.cell_rect(DenseGridCell::new(0, column))?;
        Some(Rect::from_min_max(
            Point::new(cell.min.x, label_bounds.min.y),
            Point::new(cell.max.x, label_bounds.max.y),
        ))
    }
}

impl DenseGridRasterLayout {
    /// Build raster layout from named parts.
    pub const fn from_parts(parts: DenseGridRasterLayoutParts) -> Self {
        Self {
            grid: parts.grid,
            row_origin: parts.row_origin,
            horizontal_bleed: parts.horizontal_bleed,
            vertical_bleed: parts.vertical_bleed,
        }
    }

    /// Build top-origin raster cell projection without seam bleed.
    pub const fn new(rect: Rect, rows: usize, columns: usize) -> Self {
        Self::from_parts(DenseGridRasterLayoutParts::new(DenseGridLayout::new(
            rect, rows, columns,
        )))
    }

    /// Build bottom-origin raster cell projection without seam bleed.
    pub const fn bottom_up(rect: Rect, rows: usize, columns: usize) -> Self {
        Self::new(rect, rows, columns).with_row_origin(DenseGridRowOrigin::Bottom)
    }

    /// Return this raster layout with a different row origin.
    pub const fn with_row_origin(mut self, row_origin: DenseGridRowOrigin) -> Self {
        self.row_origin = row_origin;
        self
    }

    /// Return this raster layout with horizontal seam bleed on non-terminal columns.
    pub const fn with_horizontal_bleed(mut self, bleed: f32) -> Self {
        self.horizontal_bleed = bleed;
        self
    }

    /// Return this raster layout with vertical seam bleed on non-terminal rows.
    pub const fn with_vertical_bleed(mut self, bleed: f32) -> Self {
        self.vertical_bleed = bleed;
        self
    }

    /// Return the rect for a logical raster cell.
    pub fn cell_rect(self, cell: DenseGridCell) -> Option<Rect> {
        if !self.grid.is_valid() || cell.row >= self.grid.rows || cell.column >= self.grid.columns {
            return None;
        }
        let width = self.grid.rect.width() / self.grid.columns as f32;
        let height = self.grid.rect.height() / self.grid.rows as f32;
        let horizontal_bleed = finite_nonnegative(self.horizontal_bleed);
        let vertical_bleed = finite_nonnegative(self.vertical_bleed);
        let x0 = self.grid.rect.min.x + cell.column as f32 * width;
        let x1 = if cell.column + 1 == self.grid.columns {
            self.grid.rect.max.x
        } else {
            x0 + width + horizontal_bleed
        };
        let (y0, y1) = match self.row_origin {
            DenseGridRowOrigin::Top => {
                let y0 = self.grid.rect.min.y + cell.row as f32 * height;
                let y1 = if cell.row + 1 == self.grid.rows {
                    self.grid.rect.max.y
                } else {
                    y0 + height + vertical_bleed
                };
                (y0, y1)
            }
            DenseGridRowOrigin::Bottom => {
                let y1 = self.grid.rect.max.y - cell.row as f32 * height;
                let y0 = if cell.row + 1 == self.grid.rows {
                    self.grid.rect.min.y
                } else {
                    (y1 - height - vertical_bleed).max(self.grid.rect.min.y)
                };
                (y0, y1)
            }
        };
        Some(Rect::from_min_max(Point::new(x0, y0), Point::new(x1, y1)))
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
