use super::super::{
    geometry::cell_rect,
    paint::{push_rect, push_stroke},
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
        && let Some(line) = vertical_line_rect(matrix, position.x, 1.0)
    {
        push_rect(
            primitives,
            widget.common.id,
            line,
            theme.text_muted.with_alpha(70),
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
            theme.highlight_cyan.with_alpha(190),
            2.0,
        );
    }
}
