use super::super::{
    geometry::cell_rect,
    paint::{push_rect, push_stroke, translucent},
    widget::ModulationMatrixWidget,
};
use radiant::prelude::*;

pub(super) fn append_overlay_guides(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    matrix: Rect,
    theme: &ThemeTokens,
) {
    append_cursor_guide(widget, primitives, matrix, theme);
    append_hovered_cell(widget, primitives, matrix, theme);
}

fn append_cursor_guide(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    matrix: Rect,
    theme: &ThemeTokens,
) {
    if let Some(position) = widget.hover_position
        && matrix.contains(position)
    {
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(
                Point::new(position.x, matrix.min.y),
                Point::new(position.x + 1.0, matrix.max.y),
            ),
            translucent(theme.text_muted, 70),
        );
    }
}

fn append_hovered_cell(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    matrix: Rect,
    theme: &ThemeTokens,
) {
    if let Some(cell) = widget.hover_cell {
        push_stroke(
            primitives,
            widget.common.id,
            cell_rect(matrix, cell),
            translucent(theme.highlight_cyan, 190),
            2.0,
        );
    }
}
