use radiant::prelude::*;

#[path = "paint/color.rs"]
mod color;
#[path = "paint/primitives.rs"]
mod primitives;

use super::super::model::{BINS, SpectralColumn};
use color::{rgba, spectrogram_color, translucent};
use primitives::{push_rect, push_stroke, push_text};

pub(super) struct SpectrogramPaintFrame<'a> {
    pub(super) widget_id: u64,
    pub(super) bounds: Rect,
    pub(super) plot: Rect,
    pub(super) frame: u64,
    pub(super) columns: &'a [SpectralColumn],
}

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
    append_spectrogram_cells(primitives, &frame);
    append_grid(primitives, frame.widget_id, frame.plot, theme);
    push_stroke(
        primitives,
        frame.widget_id,
        frame.plot,
        theme.border_emphasis,
        1.0,
    );
    append_labels(primitives, frame.widget_id, frame.plot, frame.frame, theme);
}

pub(super) fn append_hover(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    plot: Rect,
    x: f32,
    theme: &ThemeTokens,
) {
    push_rect(
        primitives,
        widget_id,
        Rect::from_min_max(Point::new(x, plot.min.y), Point::new(x + 2.0, plot.max.y)),
        translucent(theme.accent_mint, 180),
    );
}

fn append_spectrogram_cells(
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

fn append_grid(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    plot: Rect,
    theme: &ThemeTokens,
) {
    for fraction in [0.25, 0.5, 0.75] {
        let y = plot.max.y - plot.height() * fraction;
        push_rect(
            primitives,
            widget_id,
            Rect::from_min_max(Point::new(plot.min.x, y), Point::new(plot.max.x, y + 1.0)),
            translucent(theme.grid_soft, 150),
        );
    }
    for fraction in [0.25, 0.5, 0.75] {
        let x = plot.min.x + plot.width() * fraction;
        push_rect(
            primitives,
            widget_id,
            Rect::from_min_max(Point::new(x, plot.min.y), Point::new(x + 1.0, plot.max.y)),
            translucent(theme.grid_soft, 120),
        );
    }
}

fn append_labels(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    plot: Rect,
    frame: u64,
    theme: &ThemeTokens,
) {
    for (label, ratio) in [
        ("18k", 1.0),
        ("8k", 0.78),
        ("2k", 0.55),
        ("500", 0.32),
        ("40", 0.0),
    ] {
        append_frequency_label(primitives, widget_id, plot, label, ratio, theme);
    }
    push_text(
        primitives,
        widget_id,
        format!("frame {frame}"),
        Rect::from_min_max(
            Point::new(plot.max.x - 118.0, plot.max.y + 8.0),
            Point::new(plot.max.x, plot.max.y + 28.0),
        ),
        theme.text_muted,
        PaintTextAlign::Right,
    );
    push_text(
        primitives,
        widget_id,
        "synthetic realtime spectrum",
        Rect::from_min_max(
            Point::new(plot.min.x, plot.max.y + 8.0),
            Point::new(plot.min.x + 180.0, plot.max.y + 28.0),
        ),
        theme.text_muted,
        PaintTextAlign::Left,
    );
}

fn append_frequency_label(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    plot: Rect,
    label: &'static str,
    ratio: f32,
    theme: &ThemeTokens,
) {
    let y = plot.max.y - plot.height() * ratio;
    push_text(
        primitives,
        widget_id,
        label,
        Rect::from_min_max(
            Point::new(plot.min.x - 44.0, y - 9.0),
            Point::new(plot.min.x - 6.0, y + 11.0),
        ),
        theme.text_muted,
        PaintTextAlign::Right,
    );
}

#[cfg(test)]
pub(super) fn visible_bin_count() -> usize {
    super::super::model::COLUMNS * BINS
}
