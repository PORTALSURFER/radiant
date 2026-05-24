use radiant::prelude::*;

#[path = "paint/cells.rs"]
mod cells;
#[path = "paint/color.rs"]
mod color;
#[path = "paint/grid.rs"]
mod grid;
#[path = "paint/labels.rs"]
mod labels;
#[path = "paint/overlay.rs"]
mod overlay;
#[path = "paint/primitives.rs"]
mod primitives;

use super::super::model::SpectralColumn;
use color::rgba;
use primitives::{push_rect, push_stroke};

pub(super) struct SpectrogramPaintFrame<'a> {
    pub(super) widget_id: u64,
    pub(super) bounds: Rect,
    pub(super) plot: Rect,
    pub(super) frame: u64,
    pub(super) columns: &'a [SpectralColumn],
}

pub(super) fn append_base(
    primitives: &mut Vec<PaintPrimitive>,
    frame: SpectrogramPaintFrame<'_>,
    theme: &ThemeTokens,
) {
    push_rect(
        primitives,
        frame.widget_id,
        frame.bounds,
        theme.bg_secondary,
    );
    push_rect(
        primitives,
        frame.widget_id,
        frame.plot,
        rgba(7, 11, 18, 255),
    );
    cells::append_cells(primitives, &frame);
    grid::append_grid(primitives, frame.widget_id, frame.plot, theme);
    push_stroke(
        primitives,
        frame.widget_id,
        frame.plot,
        theme.border_emphasis,
        1.0,
    );
    labels::append_labels(primitives, frame.widget_id, frame.plot, frame.frame, theme);
}

pub(super) fn append_hover(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    plot: Rect,
    x: f32,
    theme: &ThemeTokens,
) {
    overlay::append_hover(primitives, widget_id, plot, x, theme);
}

#[cfg(test)]
pub(super) fn visible_bin_count() -> usize {
    super::super::model::COLUMNS * super::super::model::BINS
}
