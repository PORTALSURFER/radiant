use radiant::prelude::*;

use super::super::{
    model::PianoNote,
    paint::{blend_color, push_rect, push_stroke, translucent},
    widget::PianoRollWidget,
};

pub(super) fn append_note(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    note: PianoNote,
    theme: &ThemeTokens,
) {
    let rect = widget.note_rect(grid, note);
    let selected = widget.selected_note == Some(note.id);
    push_rect(
        primitives,
        widget.common.id,
        rect,
        note_fill(note, selected, theme),
    );
    push_stroke(
        primitives,
        widget.common.id,
        rect,
        note_stroke(selected, theme),
        1.0,
    );
    append_resize_handle(widget, primitives, rect, theme);
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
