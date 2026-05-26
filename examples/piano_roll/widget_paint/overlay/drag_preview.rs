use radiant::prelude::*;

use super::super::super::{
    model::PianoNote,
    paint::{push_rect, push_stroke},
    widget::PianoRollWidget,
};
use super::{
    super::{note::append_note, velocity::append_velocity_drag_preview},
    hover::append_note_hover_effect,
};

pub(crate) fn append_drag_preview(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    velocity_lane: Rect,
    position: Point,
    theme: &ThemeTokens,
) {
    if append_velocity_drag_preview(widget, primitives, velocity_lane, theme) {
        return;
    }
    if let Some(rect) = widget.marquee_rect().map(|rect| rect.clamp_to(grid)) {
        append_marquee_preview(widget, primitives, grid, rect, theme);
        return;
    }
    if let Some(rect) = widget
        .velocity_marquee_rect()
        .map(|rect| rect.clamp_to(velocity_lane))
    {
        append_velocity_marquee_preview(widget, primitives, velocity_lane, rect, theme);
        return;
    }
    let slice_notes = widget.time_slice_preview_notes(grid);
    if !slice_notes.is_empty() {
        append_time_slice_drag_preview(widget, primitives, grid, &slice_notes, theme);
        return;
    }
    append_note_drag_preview(widget, primitives, grid, position, theme);
}

fn append_velocity_marquee_preview(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    lane: Rect,
    rect: Rect,
    theme: &ThemeTokens,
) {
    for id in widget.velocity_marquee_note_ids(lane, rect) {
        if let Some(note) = widget.note_by_id(id) {
            let handle = widget.velocity_handle_rect(lane, note).clamp_to(lane);
            push_rect(
                primitives,
                widget.common.id,
                handle,
                theme.highlight_orange.with_alpha(230),
            );
            push_stroke(
                primitives,
                widget.common.id,
                handle,
                theme.text_primary.with_alpha(230),
                1.0,
            );
        }
    }
    push_rect(
        primitives,
        widget.common.id,
        rect,
        theme.highlight_blue.with_alpha(34),
    );
    push_stroke(
        primitives,
        widget.common.id,
        rect,
        theme.highlight_cyan.with_alpha(220),
        2.0,
    );
}

fn append_marquee_preview(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    rect: Rect,
    theme: &ThemeTokens,
) {
    for note in &widget.notes {
        if widget.note_rect(grid, *note).intersects(rect) {
            append_note_hover_effect(widget, primitives, grid, *note, theme);
        }
    }
    push_rect(
        primitives,
        widget.common.id,
        rect,
        theme.highlight_blue.with_alpha(34),
    );
    push_stroke(
        primitives,
        widget.common.id,
        rect,
        theme.highlight_cyan.with_alpha(220),
        2.0,
    );
}

fn append_note_drag_preview(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    position: Point,
    theme: &ThemeTokens,
) {
    for note in widget.drag_preview_notes(grid, position) {
        let rect = widget.note_rect(grid, note);
        push_rect(
            primitives,
            widget.common.id,
            rect,
            theme.highlight_blue.with_alpha(120),
        );
        push_stroke(
            primitives,
            widget.common.id,
            rect,
            theme.text_primary.with_alpha(230),
            2.0,
        );
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(
                Point::new(rect.min.x, rect.min.y),
                Point::new(rect.min.x + 3.0, rect.max.y),
            ),
            theme.highlight_cyan,
        );
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(
                Point::new(rect.max.x - 3.0, rect.min.y),
                Point::new(rect.max.x, rect.max.y),
            ),
            theme.highlight_cyan,
        );
    }
}

fn append_time_slice_drag_preview(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    notes: &[PianoNote],
    theme: &ThemeTokens,
) {
    for note in notes {
        append_note(widget, primitives, grid, *note, theme);
        push_stroke(
            primitives,
            widget.common.id,
            widget.note_rect(grid, *note),
            theme.text_primary.with_alpha(220),
            2.0,
        );
    }
}
