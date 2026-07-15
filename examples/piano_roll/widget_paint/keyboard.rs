use radiant::prelude::*;
use radiant::runtime::PaintTextAlign;

use super::super::{
    geometry::{is_black_key, pitch_label, row_height_for},
    paint::{push_rect, push_stroke, push_text},
    widget::PianoRollWidget,
};

#[path = "keyboard/octave.rs"]
mod octave;

use octave::append_octave_keyboard;

const LABEL_BASELINE: f32 = 16.0;
const LABEL_LEFT_INSET: f32 = 6.0;
const LABEL_VERTICAL_PADDING: f32 = 3.0;
const MIN_LABEL_WIDTH: f32 = 24.0;

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
        Rgba8::new(17, 19, 23, 255),
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
        theme.highlight_blue.with_alpha(110),
        theme.highlight_cyan.with_alpha(220),
    );
    let row = widget.keyboard_pitch_rect(grid, pitch).clamp_to(grid);
    push_rect(
        primitives,
        widget.common.id,
        row,
        theme.highlight_blue.with_alpha(30),
    );
    push_stroke(
        primitives,
        widget.common.id,
        row,
        theme.highlight_cyan.with_alpha(115),
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
            theme
                .highlight_orange
                .with_alpha(if is_black_key(pitch) { 120 } else { 85 }),
            theme.highlight_orange.with_alpha(230),
        );
    }
    if let Some(pitch) = widget.active_pitch {
        append_keyboard_key_highlight(
            widget,
            primitives,
            keyboard,
            pitch,
            theme.highlight_orange.with_alpha(180),
            theme.text_primary.with_alpha(235),
        );
        let grid = widget.editor_rect(bounds);
        let row = widget.keyboard_pitch_rect(grid, pitch).clamp_to(grid);
        push_rect(
            primitives,
            widget.common.id,
            row,
            theme.highlight_orange.with_alpha(72),
        );
        push_stroke(
            primitives,
            widget.common.id,
            row,
            theme.highlight_orange.with_alpha(210),
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
        Rgba8::new(30, 34, 41, 255)
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
    row_height >= LABEL_BASELINE + LABEL_VERTICAL_PADDING && label_width >= MIN_LABEL_WIDTH
}
