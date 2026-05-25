use radiant::prelude::*;

use super::super::{
    model::PianoNote,
    paint::{blend_color, push_rect, push_stroke, rgba, translucent},
    widget::PianoRollWidget,
};

pub(crate) fn append_note(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    note: PianoNote,
    theme: &ThemeTokens,
) {
    let rect = widget.note_rect(grid, note);
    let selected = widget.note_is_selected(note.id);
    let fill = note_fill(note, selected, theme);
    push_rect(primitives, widget.common.id, rect, fill);
    push_stroke(
        primitives,
        widget.common.id,
        rect,
        note_stroke(selected, theme),
        if selected { 2.0 } else { 1.0 },
    );
    append_resize_handle(widget, primitives, rect, theme);
}

fn note_fill(note: PianoNote, selected: bool, theme: &ThemeTokens) -> Rgba8 {
    if selected {
        return velocity_alpha(theme.highlight_blue, note.velocity);
    }
    velocity_alpha(
        blend_color(
            theme.highlight_cyan,
            theme.highlight_blue,
            note.velocity * 0.45,
        ),
        note.velocity,
    )
}

fn velocity_alpha(color: Rgba8, velocity: f32) -> Rgba8 {
    let alpha = (51.0 + velocity.clamp(0.0, 1.0) * 204.0).round() as u8;
    rgba(color.r, color.g, color.b, alpha)
}

fn note_stroke(selected: bool, theme: &ThemeTokens) -> Rgba8 {
    if selected {
        theme.highlight_orange
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
