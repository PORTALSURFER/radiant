use radiant::prelude::*;

use super::super::{
    TOTAL_BEATS,
    drag::PianoDrag,
    geometry::x_for_beat_view,
    model::PianoNote,
    paint::{push_rect, push_stroke, push_text, rgba, translucent},
    widget::PianoRollWidget,
};

pub(crate) fn append_velocity_lane(
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

pub(in crate::piano_roll::widget_paint) fn append_velocity_drag_preview(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    lane: Rect,
    theme: &ThemeTokens,
) -> bool {
    let Some(PianoDrag::Velocity { ids, .. }) = widget.drag.as_ref() else {
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
