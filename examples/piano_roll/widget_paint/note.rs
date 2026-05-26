use radiant::gui::visualization::{DragHandleRole, horizontal_resize_edge_visual_rect};
use radiant::prelude::*;

use super::super::{
    model::PianoNote,
    paint::{push_rect, push_rect_batch, push_stroke, push_stroke_batch},
    widget::PianoRollWidget,
};

const BATCHED_NOTE_FILL_THRESHOLD: usize = 16;

pub(crate) fn append_notes(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    notes: &[PianoNote],
    theme: &ThemeTokens,
) {
    if widget.selected_note_count() < BATCHED_NOTE_FILL_THRESHOLD {
        for note in notes {
            append_note(widget, primitives, grid, *note, theme);
        }
        return;
    }

    let mut selected_fills = Vec::new();
    for note in notes {
        let rect = widget.note_rect(grid, *note);
        if widget.note_is_selected(note.id) {
            selected_fills.push(rect);
        } else {
            push_rect(
                primitives,
                widget.common.id,
                rect,
                note_fill(*note, false, theme),
            );
        }
    }
    if let Some(note) = notes
        .iter()
        .copied()
        .find(|note| widget.note_is_selected(note.id))
    {
        push_rect_batch(
            primitives,
            widget.common.id,
            selected_fills,
            note_fill(note, true, theme),
        );
    }
    let mut selected_strokes = Vec::new();
    let mut selected_resize_handles = Vec::new();
    for note in notes {
        if widget.note_is_selected(note.id) {
            let rect = widget.note_rect(grid, *note);
            selected_strokes.push(rect);
            selected_resize_handles.push(resize_handle_rect(rect));
        } else {
            append_note_stroke_and_handle(widget, primitives, grid, *note, theme);
        }
    }
    push_stroke_batch(
        primitives,
        widget.common.id,
        selected_strokes,
        note_stroke(true, theme),
        2.0,
    );
    push_rect_batch(
        primitives,
        widget.common.id,
        selected_resize_handles,
        theme.text_primary.with_alpha(150),
    );
}

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
    append_note_stroke_and_handle(widget, primitives, grid, note, theme);
}

fn append_note_stroke_and_handle(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    note: PianoNote,
    theme: &ThemeTokens,
) {
    let rect = widget.note_rect(grid, note);
    let selected = widget.note_is_selected(note.id);
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
        theme
            .highlight_cyan
            .blend_opaque_toward(theme.highlight_blue, note.velocity * 0.45),
        note.velocity,
    )
}

fn velocity_alpha(color: Rgba8, velocity: f32) -> Rgba8 {
    let alpha = (51.0 + velocity.clamp(0.0, 1.0) * 204.0).round() as u8;
    Rgba8::new(color.r, color.g, color.b, alpha)
}

fn note_stroke(selected: bool, theme: &ThemeTokens) -> Rgba8 {
    if selected {
        theme.highlight_orange
    } else {
        theme.border_emphasis.with_alpha(145)
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
        resize_handle_rect(rect),
        theme.text_primary.with_alpha(150),
    );
}

fn resize_handle_rect(rect: Rect) -> Rect {
    horizontal_resize_edge_visual_rect(rect, DragHandleRole::End, 3.0, 2.0, 2.0)
        .unwrap_or_else(|| rect.empty_at_max())
}
