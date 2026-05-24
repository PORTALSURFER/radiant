use radiant::prelude::*;

use super::{
    LOW_PITCH, PITCH_ROWS, TOTAL_BEATS,
    geometry::{is_black_key, pitch_label, row_height, x_for_beat},
    model::PianoNote,
    paint::{blend_color, push_rect, push_stroke, push_text, rgba, translucent},
    widget::PianoRollWidget,
};

pub(super) fn append_keyboard(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let keyboard = widget.keyboard_rect(bounds);
    push_rect(
        primitives,
        widget.common.id,
        keyboard,
        rgba(17, 19, 23, 255),
    );
    for row in 0..PITCH_ROWS {
        append_key_row(widget, primitives, keyboard, row, theme);
    }
}

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

pub(super) fn append_note(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    note: PianoNote,
    theme: &ThemeTokens,
) {
    let rect = widget.note_rect(grid, note);
    let selected = widget.selected_note == Some(note.id);
    let fill = note_fill(note, selected, theme);
    push_rect(primitives, widget.common.id, rect, fill);
    push_stroke(
        primitives,
        widget.common.id,
        rect,
        note_stroke(selected, theme),
        1.0,
    );
    append_resize_handle(widget, primitives, rect, theme);
}

pub(super) fn append_hover_guides(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    theme: &ThemeTokens,
) {
    if let Some(position) = widget.hover_position
        && grid.contains(position)
    {
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(
                Point::new(position.x, grid.min.y),
                Point::new(position.x + 1.0, grid.max.y),
            ),
            translucent(theme.text_muted, 90),
        );
    }
    if let Some(note) = widget.hover_note.and_then(|id| widget.note_by_id(id)) {
        push_stroke(
            primitives,
            widget.common.id,
            widget.note_rect(grid, note),
            translucent(theme.highlight_cyan, 190),
            2.0,
        );
    }
}

fn append_key_row(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    keyboard: Rect,
    row: usize,
    theme: &ThemeTokens,
) {
    let pitch = LOW_PITCH + (PITCH_ROWS - 1 - row) as i32;
    let y = keyboard.min.y + row as f32 * row_height(keyboard);
    let rect = Rect::from_min_max(
        Point::new(keyboard.min.x, y),
        Point::new(keyboard.max.x, y + row_height(keyboard)),
    );
    let fill = if is_black_key(pitch) {
        rgba(33, 38, 45, 255)
    } else {
        theme.surface_raised
    };
    push_rect(primitives, widget.common.id, rect, fill);
    if pitch % 12 == 0 {
        push_text(
            primitives,
            widget.common.id,
            pitch_label(pitch),
            rect,
            theme.text_muted,
            PaintTextAlign::Center,
        );
    }
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

fn note_fill(note: PianoNote, selected: bool, theme: &ThemeTokens) -> Rgba8 {
    if selected {
        return theme.highlight_blue;
    }
    blend_color(
        theme.highlight_cyan,
        theme.highlight_blue,
        note.velocity * 0.45,
    )
}

fn note_stroke(selected: bool, theme: &ThemeTokens) -> Rgba8 {
    if selected {
        theme.border_emphasis
    } else {
        translucent(theme.border_emphasis, 145)
    }
}

fn append_resize_handle(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    rect: Rect,
    theme: &ThemeTokens,
) {
    push_rect(
        primitives,
        widget.common.id,
        Rect::from_min_max(
            Point::new(rect.max.x - 5.0, rect.min.y + 2.0),
            Point::new(rect.max.x - 2.0, rect.max.y - 2.0),
        ),
        translucent(theme.text_primary, 150),
    );
}
