use radiant::prelude::*;

use super::super::{
    geometry::{is_black_key, pitch_label, row_height_for},
    paint::{push_rect, push_text, rgba},
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
    for row in 0..widget.viewport.row_count() {
        append_key_row(widget, primitives, keyboard, row, theme);
    }
}

fn append_key_row(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    keyboard: Rect,
    row: usize,
    theme: &ThemeTokens,
) {
    let pitch = widget.viewport.pitch_end() - row as i32;
    let row_height = row_height_for(keyboard, widget.viewport);
    let y = keyboard.min.y + row as f32 * row_height;
    let rect = Rect::from_min_max(
        Point::new(keyboard.min.x, y),
        Point::new(keyboard.max.x, y + row_height),
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
