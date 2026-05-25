use radiant::prelude::*;

use super::{
    super::{
        model::PianoNote,
        paint::{push_rect, push_stroke, rgba, translucent},
        widget::PianoRollWidget,
    },
    grid::append_grid,
    note::append_note,
    velocity::append_velocity_drag_preview,
};

pub(crate) fn append_hover_guides(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    theme: &ThemeTokens,
) {
    if let Some(position) = widget.hover_position
        && grid.contains(position)
    {
        let x = widget.hover_cursor_x(grid, position);
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(Point::new(x, grid.min.y), Point::new(x + 1.0, grid.max.y)),
            translucent(theme.text_muted, 90),
        );
    }
    if let Some(note) = widget.hover_note.and_then(|id| widget.note_by_id(id)) {
        append_note_hover_effect(widget, primitives, grid, note, theme);
    }
}

pub(crate) fn append_time_selection(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    theme: &ThemeTokens,
) {
    if widget.moving_time_selection_clears_source()
        && let Some(source) = widget
            .moving_time_selection_source_rect(grid)
            .map(|rect| rect.clamp_to(grid))
        && source.width() >= 1.0
    {
        push_rect(primitives, widget.common.id, source, rgba(8, 12, 18, 255));
        append_grid(widget, primitives, grid, theme);
    }
    if let Some(selection) = widget.active_time_selection_rect(grid) {
        let rect = selection.clamp_to(grid);
        if rect.width() >= 1.0 {
            push_rect(
                primitives,
                widget.common.id,
                rect,
                translucent(theme.highlight_blue, 42),
            );
            push_stroke(
                primitives,
                widget.common.id,
                rect,
                translucent(theme.highlight_cyan, 215),
                1.5,
            );
        }
    }
    if let Some(cursor_x) = widget.edit_cursor_x(grid) {
        let x = cursor_x.clamp(grid.min.x, grid.max.x);
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(Point::new(x, grid.min.y), Point::new(x + 2.0, grid.max.y)),
            translucent(theme.text_primary, 210),
        );
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(
                Point::new(x + 2.0, grid.min.y),
                Point::new(x + 3.0, grid.max.y),
            ),
            translucent(theme.highlight_cyan, 145),
        );
    }
}

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
    let slice_notes = widget.time_slice_preview_notes(grid);
    if !slice_notes.is_empty() {
        append_time_slice_drag_preview(widget, primitives, grid, &slice_notes, theme);
        return;
    }
    append_note_drag_preview(widget, primitives, grid, position, theme);
}

fn append_marquee_preview(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    rect: Rect,
    theme: &ThemeTokens,
) {
    for note in &widget.notes {
        if rects_overlap(widget.note_rect(grid, *note), rect) {
            append_note_hover_effect(widget, primitives, grid, *note, theme);
        }
    }
    push_rect(
        primitives,
        widget.common.id,
        rect,
        translucent(theme.highlight_blue, 34),
    );
    push_stroke(
        primitives,
        widget.common.id,
        rect,
        translucent(theme.highlight_cyan, 220),
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
            translucent(theme.highlight_blue, 120),
        );
        push_stroke(
            primitives,
            widget.common.id,
            rect,
            translucent(theme.text_primary, 230),
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
            translucent(theme.text_primary, 220),
            2.0,
        );
    }
}

fn append_note_hover_effect(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    note: PianoNote,
    theme: &ThemeTokens,
) {
    let note_rect = widget.note_rect(grid, note);
    let tail_rect = hover_tail_rect(note_rect).clamp_to(grid);
    push_rect(
        primitives,
        widget.common.id,
        note_rect.clamp_to(grid),
        translucent(theme.highlight_orange, 90),
    );
    push_rect(
        primitives,
        widget.common.id,
        note_rect,
        translucent(theme.highlight_cyan, 72),
    );
    push_rect(
        primitives,
        widget.common.id,
        tail_rect,
        translucent(theme.highlight_cyan, 145),
    );
    push_stroke(
        primitives,
        widget.common.id,
        note_rect,
        theme.highlight_orange,
        2.0,
    );
}

fn hover_tail_rect(note_rect: Rect) -> Rect {
    let head_width = note_rect.width().clamp(0.0, 12.0);
    Rect::from_min_max(
        Point::new(note_rect.min.x + head_width, note_rect.min.y),
        note_rect.max,
    )
}

fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.min.x <= b.max.x && a.max.x >= b.min.x && a.min.y <= b.max.y && a.max.y >= b.min.y
}
