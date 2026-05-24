use radiant::prelude::*;

use super::{
    DESTINATION_COUNT, DESTINATIONS, MatrixCell, SOURCE_COUNT, SOURCES,
    geometry::{amount_bar_rect, cell_rect, destination_label_rect, source_label_rect},
    paint::{blend_color, push_rect, push_stroke, push_text, translucent},
    widget::ModulationMatrixWidget,
};

pub(super) fn append_labels(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    matrix: Rect,
    theme: &ThemeTokens,
) {
    for (source, label) in SOURCES.iter().enumerate() {
        let cell = cell_rect(
            matrix,
            MatrixCell {
                source,
                destination: 0,
            },
        );
        push_text(
            primitives,
            widget.common.id,
            *label,
            source_label_rect(bounds, matrix, cell),
            theme.text_muted,
            PaintTextAlign::Right,
        );
    }
    for (destination, label) in DESTINATIONS.iter().enumerate() {
        let cell = cell_rect(
            matrix,
            MatrixCell {
                source: 0,
                destination,
            },
        );
        push_text(
            primitives,
            widget.common.id,
            *label,
            destination_label_rect(matrix, cell),
            theme.text_muted,
            PaintTextAlign::Center,
        );
    }
}

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

pub(super) fn append_overlay_guides(
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

pub(super) fn append_activity_pulses(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    matrix: Rect,
    theme: &ThemeTokens,
) {
    for source in 0..SOURCE_COUNT {
        for destination in 0..DESTINATION_COUNT {
            append_activity_pulse(widget, primitives, matrix, source, destination, theme);
        }
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
    push_rect(
        primitives,
        widget.common.id,
        amount_bar_rect(rect, amount),
        if amount >= 0.0 {
            theme.highlight_cyan
        } else {
            theme.highlight_orange
        },
    );
    push_text(
        primitives,
        widget.common.id,
        format!("{:+.0}", amount * 100.0),
        rect,
        theme.text_primary,
        PaintTextAlign::Center,
    );
}

fn append_activity_pulse(
    widget: &ModulationMatrixWidget,
    primitives: &mut Vec<PaintPrimitive>,
    matrix: Rect,
    source: usize,
    destination: usize,
    theme: &ThemeTokens,
) {
    let amount = widget.amounts[source][destination];
    if amount.abs() < 0.20 {
        return;
    }
    let rect = cell_rect(
        matrix,
        MatrixCell {
            source,
            destination,
        },
    );
    let phase = (widget.activity_phase + source as f32 * 0.11 + destination as f32 * 0.07).fract();
    let x = rect.min.x + 8.0 + (rect.width() - 16.0) * phase;
    push_rect(
        primitives,
        widget.common.id,
        Rect::from_min_size(
            Point::new(x, rect.min.y + 7.0),
            Vector2::new(4.0, rect.height() - 14.0),
        ),
        translucent(theme.text_primary, 70),
    );
}

fn cell_fill(selected: bool, theme: &ThemeTokens) -> Rgba8 {
    if selected {
        blend_color(theme.surface_raised, theme.highlight_blue, 0.18)
    } else {
        theme.surface_base
    }
}
