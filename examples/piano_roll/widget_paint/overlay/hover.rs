use radiant::prelude::*;

use super::super::super::{
    model::PianoNote,
    paint::{push_rect, push_stroke},
    widget::{NoteResizeEdge, PianoRollWidget},
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
        if let Some(line) = vertical_line_rect(grid, x, 1.0) {
            push_rect(
                primitives,
                widget.common.id,
                line,
                theme.text_muted.with_alpha(90),
            );
        }
    }
    if let Some(note) = widget.hover_note.and_then(|id| widget.note_by_id(id)) {
        append_note_hover_effect(widget, primitives, grid, note, theme);
        if let Some(edge) = widget.hover_note_resize_edge {
            append_note_resize_cursor(widget, primitives, grid, note, edge, theme);
        }
    }
}

fn append_note_resize_cursor(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    note: PianoNote,
    edge: NoteResizeEdge,
    theme: &ThemeTokens,
) {
    let rect = widget.note_rect(grid, note).clamp_to(grid);
    let color = theme.highlight_orange;
    let width = 2.0;
    let tick = 7.0_f32.min(rect.width().max(0.0));
    let x = match edge {
        NoteResizeEdge::Start => rect.min.x,
        NoteResizeEdge::End => rect.max.x - width,
    };
    let (tick_min_x, tick_max_x) = match edge {
        NoteResizeEdge::Start => (x, x + tick),
        NoteResizeEdge::End => (x + width - tick, x + width),
    };
    if let Some(line) = vertical_line_rect(rect, x, width) {
        push_rect(primitives, widget.common.id, line, color);
    }
    push_rect(
        primitives,
        widget.common.id,
        Rect::from_min_max(
            Point::new(tick_min_x, rect.min.y),
            Point::new(tick_max_x, rect.min.y + width),
        ),
        color,
    );
    push_rect(
        primitives,
        widget.common.id,
        Rect::from_min_max(
            Point::new(tick_min_x, rect.max.y - width),
            Point::new(tick_max_x, rect.max.y),
        ),
        color,
    );
}

pub(super) fn append_note_hover_effect(
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
        theme.highlight_orange.with_alpha(90),
    );
    push_rect(
        primitives,
        widget.common.id,
        note_rect,
        theme.highlight_cyan.with_alpha(72),
    );
    push_rect(
        primitives,
        widget.common.id,
        tail_rect,
        theme.highlight_cyan.with_alpha(145),
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
