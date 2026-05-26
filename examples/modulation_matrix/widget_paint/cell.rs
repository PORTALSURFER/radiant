use super::super::{
    MatrixCell,
    geometry::{amount_bar_rect, cell_rect},
    paint::{push_rect, push_stroke, push_text, translucent},
    widget::ModulationMatrixWidget,
};
use radiant::prelude::*;

pub(super) fn append_cell(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    matrix: Rect,
    cell: MatrixCell,
    theme: &ThemeTokens,
) {
    let rect = cell_rect(matrix, cell);
    let amount = widget.amounts[cell.source][cell.destination];
    let selected = widget.selected == cell;
    push_rect(
        primitives,
        widget.common.id,
        rect,
        cell_fill(selected, theme),
    );
    push_stroke(
        primitives,
        widget.common.id,
        rect,
        translucent(theme.border, 130),
        1.0,
    );
    append_zero_line(widget, primitives, rect, theme);
    if amount.abs() > 0.01 {
        append_amount_bar(widget, primitives, rect, amount, theme);
    }
}

fn append_zero_line(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    rect: Rect,
    theme: &ThemeTokens,
) {
    let center_y = rect.center().y;
    push_rect(
        primitives,
        widget.common.id,
        Rect::from_min_max(
            Point::new(rect.min.x + 8.0, center_y),
            Point::new(rect.max.x - 8.0, center_y + 1.0),
        ),
        translucent(theme.grid_soft, 140),
    );
}

fn append_amount_bar(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    rect: Rect,
    amount: f32,
    theme: &ThemeTokens,
) {
    if let Some(fill_rect) = amount_bar_rect(rect, amount) {
        push_rect(
            primitives,
            widget.common.id,
            fill_rect,
            if amount >= 0.0 {
                theme.highlight_cyan
            } else {
                theme.highlight_orange
            },
        );
    }
    push_text(
        primitives,
        widget.common.id,
        format!("{:+.0}", amount * 100.0),
        rect,
        theme.text_primary,
        PaintTextAlign::Center,
    );
}

fn cell_fill(selected: bool, theme: &ThemeTokens) -> Rgba8 {
    if selected {
        theme
            .surface_raised
            .blend_opaque_toward(theme.highlight_blue, 0.18)
    } else {
        theme.surface_base
    }
}
