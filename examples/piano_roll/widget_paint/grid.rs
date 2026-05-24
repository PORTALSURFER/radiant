use radiant::prelude::*;

use super::super::{
    PITCH_ROWS, TOTAL_BEATS,
    geometry::{row_height, x_for_beat},
    paint::{push_rect, push_text, rgba, translucent},
    widget::PianoRollWidget,
};

pub(super) fn append_grid(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    theme: &ThemeTokens,
) {
    push_rect(primitives, widget.common.id, grid, rgba(8, 12, 18, 255));
    append_pitch_lines(widget, primitives, grid, theme);
    append_beat_lines(widget, primitives, grid, theme);
}

fn append_pitch_lines(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    theme: &ThemeTokens,
) {
    for row in 0..=PITCH_ROWS {
        let y = grid.min.y + row as f32 * row_height(grid);
        let color = if row % 12 == 0 {
            translucent(theme.grid_strong, 170)
        } else {
            translucent(theme.grid_soft, 105)
        };
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(Point::new(grid.min.x, y), Point::new(grid.max.x, y + 1.0)),
            color,
        );
    }
}

fn append_beat_lines(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    theme: &ThemeTokens,
) {
    for beat in 0..=(TOTAL_BEATS as usize * 4) {
        append_beat_line(widget, primitives, grid, beat, theme);
    }
}

fn append_beat_line(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    beat: usize,
    theme: &ThemeTokens,
) {
    let x = x_for_beat(grid, beat as f32 / 4.0);
    push_rect(
        primitives,
        widget.common.id,
        Rect::from_min_max(Point::new(x, grid.min.y), Point::new(x + 1.0, grid.max.y)),
        beat_line_color(beat, theme),
    );
    if beat.is_multiple_of(4) && beat < (TOTAL_BEATS as usize * 4) {
        push_text(
            primitives,
            widget.common.id,
            format!("{}", beat / 4 + 1),
            Rect::from_min_size(
                Point::new(x + 4.0, grid.min.y - 24.0),
                Vector2::new(42.0, 18.0),
            ),
            theme.text_muted,
            PaintTextAlign::Left,
        );
    }
}

fn beat_line_color(beat: usize, theme: &ThemeTokens) -> Rgba8 {
    if beat.is_multiple_of(16) {
        translucent(theme.grid_strong, 190)
    } else if beat.is_multiple_of(4) {
        translucent(theme.grid_strong, 125)
    } else {
        translucent(theme.grid_soft, 80)
    }
}
