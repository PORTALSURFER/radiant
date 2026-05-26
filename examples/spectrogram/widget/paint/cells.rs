use super::SpectrogramPaintFrame;
use super::color::spectrogram_color;
use super::primitives::push_rect;
use crate::model::{BINS, SpectralColumn};
use radiant::prelude::*;

struct ColumnPaint<'a> {
    column: &'a SpectralColumn,
    index: usize,
}

pub(super) fn append_cells(
    primitives: &mut Vec<PaintPrimitive>,
    frame: &SpectrogramPaintFrame<'_>,
) {
    if frame.columns.is_empty() {
        return;
    }

    let raster = DenseGridRasterLayout::bottom_up(frame.plot, BINS, frame.columns.len())
        .with_horizontal_bleed(0.5)
        .with_vertical_bleed(0.5);
    for (index, column) in frame.columns.iter().enumerate() {
        append_column_cells(
            primitives,
            frame.widget_id,
            raster,
            ColumnPaint { column, index },
        );
    }
}

fn append_column_cells(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    raster: DenseGridRasterLayout,
    column: ColumnPaint<'_>,
) {
    for (bin_index, energy) in column.column.bins.iter().enumerate() {
        let Some(rect) = raster.cell_rect(DenseGridCell::new(bin_index, column.index)) else {
            continue;
        };
        push_rect(primitives, widget_id, rect, spectrogram_color(*energy));
    }
}
