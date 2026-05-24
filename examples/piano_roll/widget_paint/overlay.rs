use radiant::prelude::*;

use super::super::{
    paint::{push_rect, push_stroke, translucent},
    widget::PianoRollWidget,
};

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
        push_stroke(
            primitives,
            widget.common.id,
            widget.note_rect(grid, note),
            translucent(theme.highlight_cyan, 190),
            2.0,
        );
    }
}
