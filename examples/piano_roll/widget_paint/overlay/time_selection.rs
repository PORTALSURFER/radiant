use radiant::prelude::*;

use super::super::super::{
    TOTAL_BEATS,
    geometry::{row_height_for, x_for_beat_view},
    paint::{push_rect, push_stroke, rgba, translucent},
    widget::PianoRollWidget,
};

pub(crate) fn append_time_selection(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    theme: &ThemeTokens,
) {
    if widget.moving_time_selection_clears_source()
        && let Some(source) = widget
            .moving_time_selection_source_rect(grid)
            .map(|rect| rect.clamp_to(grid))
        && source.width() >= 1.0
    {
        push_rect(primitives, widget.common.id, source, rgba(8, 12, 18, 255));
        append_source_mask_grid(widget, primitives, grid, source, theme);
    }
    if let Some(selection) = widget.active_time_selection_rect(grid) {
        let rect = selection.clamp_to(grid);
        if rect.width() >= 1.0 {
            push_rect(
                primitives,
                widget.common.id,
                rect,
                translucent(theme.highlight_blue, 42),
            );
            push_stroke(
                primitives,
                widget.common.id,
                rect,
                translucent(theme.highlight_cyan, 215),
                1.5,
            );
        }
    }
    if let Some(cursor_x) = widget.edit_cursor_x(grid) {
        let x = cursor_x.clamp(grid.min.x, grid.max.x);
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(Point::new(x, grid.min.y), Point::new(x + 2.0, grid.max.y)),
            translucent(theme.text_primary, 210),
        );
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(
                Point::new(x + 2.0, grid.min.y),
                Point::new(x + 3.0, grid.max.y),
            ),
            translucent(theme.highlight_cyan, 145),
        );
    }
}

fn append_source_mask_grid(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    source: Rect,
    theme: &ThemeTokens,
) {
    append_source_mask_pitch_lines(widget, primitives, grid, source, theme);
    append_source_mask_beat_lines(widget, primitives, grid, source, theme);
}

fn append_source_mask_pitch_lines(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    source: Rect,
    theme: &ThemeTokens,
) {
    for row in 0..=widget.viewport.row_count() {
        let y = grid.min.y + row as f32 * row_height_for(grid, widget.viewport);
        let line = Rect::from_min_max(
            Point::new(source.min.x, y),
            Point::new(source.max.x, y + 1.0),
        );
        if line.max.y < source.min.y || line.min.y > source.max.y {
            continue;
        }
        let color = if row % 12 == 0 {
            translucent(theme.grid_strong, 170)
        } else {
            translucent(theme.grid_soft, 105)
        };
        push_rect(primitives, widget.common.id, line.clamp_to(source), color);
    }
}

fn append_source_mask_beat_lines(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    source: Rect,
    theme: &ThemeTokens,
) {
    let first = (widget.viewport.beat_start * 4.0).floor().max(0.0) as usize;
    let last = (widget.viewport.beat_end() * 4.0)
        .ceil()
        .min(TOTAL_BEATS * 4.0) as usize;
    for beat in first..=last {
        let x = x_for_beat_view(grid, widget.viewport, beat as f32 / 4.0);
        let line = Rect::from_min_max(
            Point::new(x, source.min.y),
            Point::new(x + 1.0, source.max.y),
        );
        if line.max.x < source.min.x || line.min.x > source.max.x {
            continue;
        }
        push_rect(
            primitives,
            widget.common.id,
            line.clamp_to(source),
            source_mask_beat_line_color(beat, theme),
        );
    }
}

fn source_mask_beat_line_color(beat: usize, theme: &ThemeTokens) -> Rgba8 {
    if beat.is_multiple_of(16) {
        translucent(theme.grid_strong, 190)
    } else if beat.is_multiple_of(4) {
        translucent(theme.grid_strong, 125)
    } else {
        translucent(theme.grid_soft, 80)
    }
}
