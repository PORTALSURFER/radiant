use radiant::prelude::*;

use super::super::{
    geometry::{is_black_key, pitch_label, row_height_for},
    paint::{push_rect, push_stroke, push_text, rgba, translucent},
    widget::PianoRollWidget,
};

const LABEL_FONT_SIZE: f32 = 12.0;
const LABEL_BASELINE: f32 = 16.0;
const LABEL_LEFT_INSET: f32 = 6.0;
const LABEL_VERTICAL_PADDING: f32 = 3.0;
const MIN_LABEL_WIDTH: f32 = 24.0;
const COMPACT_KEY_STRIP_WIDTH: f32 = 12.0;

pub(crate) fn append_keyboard(
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
    if keyboard_paint_mode(keyboard, widget) == KeyboardPaintMode::Octaves {
        append_octave_keyboard(widget, primitives, keyboard, theme);
        return;
    }
    for row in 0..widget.viewport.row_count() {
        append_key_row(widget, primitives, keyboard, row, theme);
    }
}

pub(crate) fn append_selected_pitch_lane(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    grid: Rect,
    theme: &ThemeTokens,
) {
    let Some(pitch) = widget.selected_pitch else {
        return;
    };
    let keyboard = widget.keyboard_rect(bounds);
    append_keyboard_key_highlight(
        widget,
        primitives,
        keyboard,
        pitch,
        translucent(theme.highlight_blue, 110),
        translucent(theme.highlight_cyan, 220),
    );
    let row = widget.keyboard_pitch_rect(grid, pitch).clamp_to(grid);
    push_rect(
        primitives,
        widget.common.id,
        row,
        translucent(theme.highlight_blue, 30),
    );
    push_stroke(
        primitives,
        widget.common.id,
        row,
        translucent(theme.highlight_cyan, 115),
        1.0,
    );
}

pub(crate) fn append_keyboard_interaction(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let keyboard = widget.keyboard_rect(bounds);
    if let Some(pitch) = widget.hover_pitch {
        append_keyboard_key_highlight(
            widget,
            primitives,
            keyboard,
            pitch,
            translucent(
                theme.highlight_orange,
                if is_black_key(pitch) { 120 } else { 85 },
            ),
            translucent(theme.highlight_orange, 230),
        );
    }
    if let Some(pitch) = widget.active_pitch {
        append_keyboard_key_highlight(
            widget,
            primitives,
            keyboard,
            pitch,
            translucent(theme.highlight_orange, 180),
            translucent(theme.text_primary, 235),
        );
        let grid = widget.editor_rect(bounds);
        let row = widget.keyboard_pitch_rect(grid, pitch).clamp_to(grid);
        push_rect(
            primitives,
            widget.common.id,
            row,
            translucent(theme.highlight_orange, 72),
        );
        push_stroke(
            primitives,
            widget.common.id,
            row,
            translucent(theme.highlight_orange, 210),
            1.0,
        );
    }
}

fn append_keyboard_key_highlight(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    keyboard: Rect,
    pitch: i32,
    fill: Rgba8,
    stroke: Rgba8,
) {
    let row = widget
        .keyboard_pitch_rect(keyboard, pitch)
        .clamp_to(keyboard);
    let black_key = is_black_key(pitch);
    let key_rect = if black_key {
        Rect::from_min_max(
            row.min,
            Point::new(row.min.x + row.width() * 0.62, row.max.y),
        )
    } else {
        row
    };
    push_rect(primitives, widget.common.id, key_rect, fill);
    push_stroke(primitives, widget.common.id, key_rect, stroke, 1.0);
}

fn append_key_row(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    keyboard: Rect,
    row: usize,
    theme: &ThemeTokens,
) {
    let pitch = widget.viewport.pitch_end() - row as i32;
    let y = keyboard.min.y + row as f32 * row_height_for(keyboard, widget.viewport);
    let rect = Rect::from_min_max(
        Point::new(keyboard.min.x, y),
        Point::new(
            keyboard.max.x,
            y + row_height_for(keyboard, widget.viewport),
        ),
    );
    let black_key = is_black_key(pitch);
    let fill = if black_key {
        rgba(30, 34, 41, 255)
    } else {
        theme.surface_raised
    };
    let key_rect = if black_key {
        Rect::from_min_max(
            rect.min,
            Point::new(rect.min.x + rect.width() * 0.62, rect.max.y),
        )
    } else {
        rect
    };
    push_rect(primitives, widget.common.id, key_rect, fill);
    push_stroke(primitives, widget.common.id, key_rect, theme.border, 1.0);
    if key_label_fits(keyboard, key_rect, widget.viewport) {
        let label_rect = Rect::from_min_max(
            Point::new(key_rect.min.x + LABEL_LEFT_INSET, rect.min.y),
            Point::new(key_rect.max.x, rect.max.y),
        );
        push_text(
            primitives,
            widget.common.id,
            pitch_label(pitch),
            label_rect,
            theme.text_muted,
            PaintTextAlign::Left,
        );
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum KeyboardPaintMode {
    Notes,
    Octaves,
}

fn keyboard_paint_mode(keyboard: Rect, widget: &PianoRollWidget) -> KeyboardPaintMode {
    let row_height = row_height_for(keyboard, widget.viewport);
    let full_width = keyboard.width();
    if note_label_fits(row_height, full_width) {
        KeyboardPaintMode::Notes
    } else {
        KeyboardPaintMode::Octaves
    }
}

fn key_label_fits(
    keyboard: Rect,
    key_rect: Rect,
    viewport: super::super::model::PianoRollViewport,
) -> bool {
    let row_height = row_height_for(keyboard, viewport);
    let label_width = key_rect.width() - LABEL_LEFT_INSET;
    note_label_fits(row_height, label_width)
}

fn note_label_fits(row_height: f32, label_width: f32) -> bool {
    row_height >= LABEL_BASELINE + LABEL_VERTICAL_PADDING
        && row_height >= LABEL_FONT_SIZE + LABEL_VERTICAL_PADDING * 2.0
        && label_width >= MIN_LABEL_WIDTH
}

fn append_octave_keyboard(
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
        rgba(24, 27, 32, 255)
    } else {
        rgba(20, 23, 28, 255)
    };
    push_rect(primitives, widget.common.id, chunk, fill);
    push_stroke(
        primitives,
        widget.common.id,
        chunk,
        translucent(theme.border, 170),
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
            rgba(38, 42, 50, 255)
        } else {
            translucent(theme.text_primary, 160)
        };
        push_rect(primitives, widget.common.id, rect, color);
    }
    push_stroke(
        primitives,
        widget.common.id,
        key_strip,
        translucent(theme.border_emphasis, 150),
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
