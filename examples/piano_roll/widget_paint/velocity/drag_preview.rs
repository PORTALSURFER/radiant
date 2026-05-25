use radiant::prelude::*;

use super::super::super::{
    drag::PianoDrag,
    paint::{push_rect, push_rect_batch, push_stroke, push_stroke_batch, translucent},
    widget::PianoRollWidget,
};
use super::BATCHED_VELOCITY_FILL_THRESHOLD;

pub(in crate::piano_roll::widget_paint) fn append_velocity_drag_preview(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    lane: Rect,
    theme: &ThemeTokens,
) -> bool {
    let Some(ids) = velocity_drag_ids(widget.drag.as_ref()) else {
        return false;
    };
    if ids.len() < BATCHED_VELOCITY_FILL_THRESHOLD {
        for note in &widget.notes {
            if SelectionSet::slice_contains(ids, &note.id) {
                let stem = widget
                    .velocity_preview_stem_rect(lane, *note)
                    .clamp_to(lane);
                let handle = widget.velocity_handle_rect(lane, *note).clamp_to(lane);
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
        return true;
    }
    let mut fills = Vec::with_capacity(ids.len().saturating_mul(2));
    let mut handles = Vec::with_capacity(ids.len());
    for note in &widget.notes {
        if SelectionSet::slice_contains(ids, &note.id) {
            let stem = widget
                .velocity_preview_stem_rect(lane, *note)
                .clamp_to(lane);
            let handle = widget.velocity_handle_rect(lane, *note).clamp_to(lane);
            fills.push(stem);
            fills.push(handle);
            handles.push(handle);
        }
    }
    push_rect_batch(
        primitives,
        widget.common.id,
        fills,
        translucent(theme.highlight_orange, 245),
    );
    push_stroke_batch(
        primitives,
        widget.common.id,
        handles,
        translucent(theme.text_primary, 230),
        1.0,
    );
    true
}

fn velocity_drag_ids(drag: Option<&PianoDrag>) -> Option<&[u32]> {
    match drag {
        Some(PianoDrag::Velocity { ids, .. } | PianoDrag::VelocityRelative { ids, .. }) => {
            Some(ids.as_slice())
        }
        _ => None,
    }
}
