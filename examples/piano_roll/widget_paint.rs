use radiant::prelude::*;

use super::{
    TOTAL_BEATS,
    geometry::{is_black_key, pitch_label, row_height_for, x_for_beat_view},
    model::PianoNote,
    paint::{blend_color, push_rect, push_stroke, push_text, rgba, translucent},
    widget::PianoRollWidget,
};

pub(super) fn append_editor_clip_start(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
) {
    primitives.push(PaintPrimitive::ClipStart(PaintClipStart {
        node_id: widget.common.id,
        rect: grid,
    }));
}

pub(super) fn append_editor_clip_end(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
) {
    primitives.push(PaintPrimitive::ClipEnd(PaintClipEnd {
        node_id: widget.common.id,
    }));
}

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

pub(super) fn append_selected_pitch_lane(
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

pub(super) fn append_keyboard_interaction(
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

pub(super) fn append_grid(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    theme: &ThemeTokens,
) {
    push_rect(primitives, widget.common.id, grid, rgba(8, 12, 18, 255));
    append_pitch_lines(widget, primitives, grid, theme);
    append_beat_lines(widget, primitives, grid, theme);
}

pub(super) fn append_note(
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

pub(super) fn append_velocity_lane(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    lane: Rect,
    theme: &ThemeTokens,
) {
    push_rect(primitives, widget.common.id, lane, rgba(10, 13, 18, 255));
    push_stroke(
        primitives,
        widget.common.id,
        lane,
        translucent(theme.border_emphasis, 150),
        1.0,
    );
    append_velocity_lane_grid(widget, primitives, grid, lane, theme);
    for note in &widget.notes {
        append_velocity_pillar(widget, primitives, lane, *note, theme);
    }
    push_text(
        primitives,
        widget.common.id,
        "Velocity",
        Rect::from_min_size(
            Point::new(lane.min.x + 8.0, lane.min.y + 6.0),
            Vector2::new(80.0, 18.0),
        ),
        theme.text_muted,
        PaintTextAlign::Left,
    );
}

pub(super) fn append_hover_guides(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    theme: &ThemeTokens,
) {
    if let Some(position) = widget.hover_position
        && grid.contains(position)
    {
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(
                Point::new(position.x, grid.min.y),
                Point::new(position.x + 1.0, grid.max.y),
            ),
            translucent(theme.text_muted, 90),
        );
    }
    if let Some(note) = widget.hover_note.and_then(|id| widget.note_by_id(id)) {
        append_note_hover_effect(widget, primitives, grid, note, theme);
    }
}

fn append_velocity_lane_grid(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    lane: Rect,
    theme: &ThemeTokens,
) {
    for row in 1..4 {
        let y = lane.min.y + lane.height() * row as f32 / 4.0;
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(Point::new(lane.min.x, y), Point::new(lane.max.x, y + 1.0)),
            translucent(theme.grid_soft, 70),
        );
    }
    let first = (widget.viewport.beat_start * 4.0).floor().max(0.0) as usize;
    let last = (widget.viewport.beat_end() * 4.0)
        .ceil()
        .min(TOTAL_BEATS * 4.0) as usize;
    for beat in first..=last {
        if !beat.is_multiple_of(4) {
            continue;
        }
        let x = x_for_beat_view(grid, widget.viewport, beat as f32 / 4.0);
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(Point::new(x, lane.min.y), Point::new(x + 1.0, lane.max.y)),
            translucent(theme.grid_strong, 95),
        );
    }
}

fn append_velocity_pillar(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    lane: Rect,
    note: PianoNote,
    theme: &ThemeTokens,
) {
    let stem = widget.velocity_preview_stem_rect(lane, note);
    let handle = widget.velocity_handle_rect(lane, note);
    if stem.max.x < lane.min.x || stem.min.x > lane.max.x {
        return;
    }
    let selected = widget.note_is_selected(note.id);
    let fill = if selected {
        translucent(theme.highlight_blue, 230)
    } else {
        translucent(theme.highlight_cyan, 175)
    };
    push_rect(primitives, widget.common.id, stem.clamp_to(lane), fill);
    push_rect(primitives, widget.common.id, handle.clamp_to(lane), fill);
    push_stroke(
        primitives,
        widget.common.id,
        handle.clamp_to(lane),
        translucent(theme.text_primary, if selected { 210 } else { 120 }),
        1.0,
    );
}

fn hover_tail_rect(note_rect: Rect) -> Rect {
    let head_width = note_rect.width().clamp(0.0, 12.0);
    Rect::from_min_max(
        Point::new(note_rect.min.x + head_width, note_rect.min.y),
        note_rect.max,
    )
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

pub(super) fn append_drag_preview(
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
        return;
    }
    let preview_notes = widget.drag_preview_notes(grid, position);
    if preview_notes.is_empty() {
        return;
    }
    for note in preview_notes {
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

fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.min.x <= b.max.x && a.max.x >= b.min.x && a.min.y <= b.max.y && a.max.y >= b.min.y
}

fn append_velocity_drag_preview(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    lane: Rect,
    theme: &ThemeTokens,
) -> bool {
    let Some(super::drag::PianoDrag::Velocity { ids, .. }) = widget.drag.as_ref() else {
        return false;
    };
    for id in ids {
        if let Some(note) = widget.note_by_id(*id) {
            let stem = widget.velocity_preview_stem_rect(lane, note).clamp_to(lane);
            let handle = widget.velocity_handle_rect(lane, note).clamp_to(lane);
            push_rect(
                primitives,
                widget.common.id,
                stem,
                translucent(theme.highlight_orange, 240),
            );
            push_rect(
                primitives,
                widget.common.id,
                handle,
                translucent(theme.highlight_orange, 255),
            );
            push_stroke(
                primitives,
                widget.common.id,
                handle,
                translucent(theme.text_primary, 230),
                1.0,
            );
        }
    }
    true
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
    if !black_key || pitch % 12 == 0 {
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

fn append_pitch_lines(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    theme: &ThemeTokens,
) {
    for row in 0..=widget.viewport.row_count() {
        let y = grid.min.y + row as f32 * row_height_for(grid, widget.viewport);
        let color = if row % 12 == 0 {
            translucent(theme.grid_strong, 170)
        } else {
            translucent(theme.grid_soft, 105)
        };
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(Point::new(grid.min.x, y), Point::new(grid.max.x, y + 1.0)),
            color,
        );
    }
}

fn append_beat_lines(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    theme: &ThemeTokens,
) {
    let first = (widget.viewport.beat_start * 4.0).floor().max(0.0) as usize;
    let last = (widget.viewport.beat_end() * 4.0)
        .ceil()
        .min(TOTAL_BEATS * 4.0) as usize;
    for beat in first..=last {
        append_beat_line(widget, primitives, grid, beat, theme);
    }
}

fn append_beat_line(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    beat: usize,
    theme: &ThemeTokens,
) {
    let x = x_for_beat_view(grid, widget.viewport, beat as f32 / 4.0);
    push_rect(
        primitives,
        widget.common.id,
        Rect::from_min_max(Point::new(x, grid.min.y), Point::new(x + 1.0, grid.max.y)),
        beat_line_color(beat, theme),
    );
    if beat.is_multiple_of(4) && beat < (TOTAL_BEATS as usize * 4) {
        push_text(
            primitives,
            widget.common.id,
            format!("{}", beat / 4 + 1),
            Rect::from_min_size(
                Point::new(x + 4.0, grid.min.y - 24.0),
                Vector2::new(42.0, 18.0),
            ),
            theme.text_muted,
            PaintTextAlign::Left,
        );
    }
}

fn beat_line_color(beat: usize, theme: &ThemeTokens) -> Rgba8 {
    if beat.is_multiple_of(16) {
        translucent(theme.grid_strong, 190)
    } else if beat.is_multiple_of(4) {
        translucent(theme.grid_strong, 125)
    } else {
        translucent(theme.grid_soft, 80)
    }
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
