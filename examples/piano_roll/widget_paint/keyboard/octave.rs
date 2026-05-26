use radiant::prelude::*;

use super::super::super::{
    geometry::{is_black_key, pitch_label, row_height_for},
    paint::{push_rect, push_stroke, push_text},
    widget::PianoRollWidget,
};
use super::{LABEL_BASELINE, LABEL_LEFT_INSET, LABEL_VERTICAL_PADDING, MIN_LABEL_WIDTH};

const COMPACT_KEY_STRIP_WIDTH: f32 = 12.0;

pub(super) fn append_octave_keyboard(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    keyboard: Rect,
    theme: &ThemeTokens,
) {
    let key_strip = compact_key_strip_rect(keyboard);
    let label_column =
        Rect::from_min_max(keyboard.min, Point::new(key_strip.min.x, keyboard.max.y));
    for c_pitch in visible_octave_roots(widget) {
        append_octave_label_chunk(widget, primitives, keyboard, label_column, c_pitch, theme);
    }
    append_compact_key_stripes(widget, primitives, keyboard, key_strip, theme);
}

fn append_octave_label_chunk(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    keyboard: Rect,
    label_column: Rect,
    c_pitch: i32,
    theme: &ThemeTokens,
) {
    let chunk = octave_chunk_rect(widget, keyboard, c_pitch).clamp_to(label_column);
    if !chunk.has_finite_positive_area() {
        return;
    }
    let fill = if c_pitch.div_euclid(12) % 2 == 0 {
        Rgba8::new(24, 27, 32, 255)
    } else {
        Rgba8::new(20, 23, 28, 255)
    };
    push_rect(primitives, widget.common.id, chunk, fill);
    push_stroke(
        primitives,
        widget.common.id,
        chunk,
        theme.border.with_alpha(170),
        1.0,
    );
    if octave_label_fits(chunk) {
        push_text(
            primitives,
            widget.common.id,
            pitch_label(c_pitch),
            octave_label_rect(chunk),
            theme.text_muted,
            PaintTextAlign::Left,
        );
    }
}

fn append_compact_key_stripes(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    keyboard: Rect,
    key_strip: Rect,
    theme: &ThemeTokens,
) {
    for row in 0..widget.viewport.row_count() {
        let pitch = widget.viewport.pitch_end() - row as i32;
        let y = keyboard.min.y + row as f32 * row_height_for(keyboard, widget.viewport);
        let rect = Rect::from_min_max(
            Point::new(key_strip.min.x, y),
            Point::new(
                key_strip.max.x,
                y + row_height_for(keyboard, widget.viewport),
            ),
        )
        .clamp_to(key_strip);
        if !rect.has_finite_positive_area() {
            continue;
        }
        let color = if is_black_key(pitch) {
            Rgba8::new(38, 42, 50, 255)
        } else {
            theme.text_primary.with_alpha(160)
        };
        push_rect(primitives, widget.common.id, rect, color);
    }
    push_stroke(
        primitives,
        widget.common.id,
        key_strip,
        theme.border_emphasis.with_alpha(150),
        1.0,
    );
}

fn visible_octave_roots(widget: &PianoRollWidget) -> impl Iterator<Item = i32> {
    let first = widget.viewport.pitch_start.div_euclid(12) * 12;
    let last = widget.viewport.pitch_end().div_euclid(12) * 12;
    (first..=last).step_by(12)
}

fn octave_chunk_rect(widget: &PianoRollWidget, keyboard: Rect, c_pitch: i32) -> Rect {
    let low = c_pitch.max(widget.viewport.pitch_start);
    let high = (c_pitch + 11).min(widget.viewport.pitch_end());
    let top = widget.keyboard_pitch_rect(keyboard, high).min.y;
    let bottom = widget.keyboard_pitch_rect(keyboard, low).max.y;
    Rect::from_min_max(
        Point::new(keyboard.min.x, top),
        Point::new(keyboard.max.x, bottom),
    )
}

fn compact_key_strip_rect(keyboard: Rect) -> Rect {
    Rect::from_min_max(
        Point::new(
            (keyboard.max.x - COMPACT_KEY_STRIP_WIDTH).max(keyboard.min.x),
            keyboard.min.y,
        ),
        keyboard.max,
    )
}

fn octave_label_fits(chunk: Rect) -> bool {
    chunk.height() >= LABEL_BASELINE + LABEL_VERTICAL_PADDING
        && chunk.width() - LABEL_LEFT_INSET >= MIN_LABEL_WIDTH
}

fn octave_label_rect(chunk: Rect) -> Rect {
    let label_height = LABEL_BASELINE + LABEL_VERTICAL_PADDING;
    let y = (chunk.center().y - label_height * 0.5)
        .max(chunk.min.y)
        .min(chunk.max.y - label_height);
    Rect::from_min_size(
        Point::new(chunk.min.x + LABEL_LEFT_INSET, y),
        Vector2::new((chunk.width() - LABEL_LEFT_INSET).max(0.0), label_height),
    )
}
