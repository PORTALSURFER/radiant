use radiant::prelude::*;
use radiant::runtime::PaintTextAlign;

use super::super::{
    TOTAL_BEATS,
    geometry::{row_height_for, x_for_beat_view},
    paint::{push_rect, push_text},
    widget::PianoRollWidget,
};

pub(crate) fn append_grid(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    theme: &ThemeTokens,
) {
    push_rect(
        primitives,
        widget.common.id,
        grid,
        Rgba8::new(8, 12, 18, 255),
    );
    append_pitch_lines(widget, primitives, grid, theme);
    append_beat_lines(widget, primitives, grid, theme);
}

fn append_pitch_lines(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    theme: &ThemeTokens,
) {
    for row in 0..=widget.viewport.row_count() {
        let y = grid.min.y + row as f32 * row_height_for(grid, widget.viewport);
        let color = if row % 12 == 0 {
            theme.grid_strong.with_alpha(170)
        } else {
            theme.grid_soft.with_alpha(105)
        };
        if let Some(line) = horizontal_line_rect(grid, y, 1.0) {
            push_rect(primitives, widget.common.id, line, color);
        }
    }
}

fn append_beat_lines(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    theme: &ThemeTokens,
) {
    let first = (widget.viewport.beat_start * 4.0).floor().max(0.0) as usize;
    let last = (widget.viewport.beat_end() * 4.0)
        .ceil()
        .min(TOTAL_BEATS * 4.0) as usize;
    for beat in first..=last {
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
    let x = x_for_beat_view(grid, widget.viewport, beat as f32 / 4.0);
    if let Some(line) = vertical_line_rect(grid, x, 1.0) {
        push_rect(
            primitives,
            widget.common.id,
            line,
            beat_line_color(beat, theme),
        );
    }
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
        theme.grid_strong.with_alpha(190)
    } else if beat.is_multiple_of(4) {
        theme.grid_strong.with_alpha(125)
    } else {
        theme.grid_soft.with_alpha(80)
    }
}
