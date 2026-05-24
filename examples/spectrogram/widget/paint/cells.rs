use super::SpectrogramPaintFrame;
use super::color::spectrogram_color;
use super::primitives::push_rect;
use crate::model::{BINS, SpectralColumn};
use radiant::prelude::*;

struct ColumnPaint<'a> {
    column: &'a SpectralColumn,
    index: usize,
    count: usize,
}

#[derive(Clone, Copy)]
struct CellSize {
    width: f32,
    height: f32,
}

pub(super) fn append_cells(
    primitives: &mut Vec<PaintPrimitive>,
    frame: &SpectrogramPaintFrame<'_>,
) {
    if frame.columns.is_empty() {
        return;
    }

    let cell = CellSize {
        width: frame.plot.width() / frame.columns.len() as f32,
        height: frame.plot.height() / BINS as f32,
    };
    for (index, column) in frame.columns.iter().enumerate() {
        append_column_cells(
            primitives,
            frame.widget_id,
            frame.plot,
            ColumnPaint {
                column,
                index,
                count: frame.columns.len(),
            },
            cell,
        );
    }
}

fn append_column_cells(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    plot: Rect,
    column: ColumnPaint<'_>,
    cell: CellSize,
) {
    let x0 = plot.min.x + column.index as f32 * cell.width;
    let x1 = if column.index + 1 == column.count {
        plot.max.x
    } else {
        x0 + cell.width + 0.5
    };

    for (bin_index, energy) in column.column.bins.iter().enumerate() {
        let y1 = plot.max.y - bin_index as f32 * cell.height;
        let y0 = (y1 - cell.height - 0.5).max(plot.min.y);
        push_rect(
            primitives,
            widget_id,
            Rect::from_min_max(Point::new(x0, y0), Point::new(x1, y1)),
            spectrogram_color(*energy),
        );
    }
}
