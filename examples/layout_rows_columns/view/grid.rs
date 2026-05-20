use super::{fill_tile, fixed_size_tile, fixed_tile, panel};
use crate::model::LayoutDemoState;
use radiant::prelude as ui;

pub(super) fn main_grid_panel(state: &LayoutDemoState) -> ui::StateView<LayoutDemoState> {
    panel(
        "Dynamic split grid",
        ui::column(grid_rows(state))
            .fill_width()
            .fill_height()
            .spacing(grid_spacing(state)),
    )
    .fill_width()
    .fill_height()
}

fn grid_rows(state: &LayoutDemoState) -> Vec<ui::StateView<LayoutDemoState>> {
    (0..state.rows)
        .map(|row_index| {
            ui::row(grid_cells(state, row_index))
                .fill_width()
                .fill_height()
                .spacing(grid_spacing(state))
        })
        .collect()
}

fn grid_cells(state: &LayoutDemoState, row_index: usize) -> Vec<ui::StateView<LayoutDemoState>> {
    (0..state.columns)
        .map(|column_index| grid_cell(state, row_index, column_index))
        .collect()
}

fn grid_cell(
    state: &LayoutDemoState,
    row_index: usize,
    column_index: usize,
) -> ui::StateView<LayoutDemoState> {
    if grid_is_crowded(state) {
        return compact_grid_cell(state, row_index, column_index);
    }

    let title = format!("Cell {}.{}", row_index + 1, column_index + 1);
    let body = if state.show_nested {
        nested_layout(state.depth, row_index + column_index)
    } else if (row_index + column_index).is_multiple_of(2) {
        ui::column([
            fixed_tile("Fixed top", 32.0),
            fill_tile("Fill center"),
            fixed_tile("Fixed bottom", 32.0),
        ])
        .fill_width()
        .fill_height()
        .spacing(8.0)
    } else {
        ui::row([
            fixed_size_tile("Fixed left", 82.0, 42.0),
            fill_tile("Fill middle"),
            fixed_size_tile("Fixed right", 82.0, 42.0),
        ])
        .fill_width()
        .fill_height()
        .spacing(8.0)
    };

    panel(title, body).fill_width().fill_height()
}

fn compact_grid_cell(
    state: &LayoutDemoState,
    row_index: usize,
    column_index: usize,
) -> ui::StateView<LayoutDemoState> {
    let title = format!("C{}.{}", row_index + 1, column_index + 1);
    let summary = if state.show_nested {
        format!("{}x{} d{}", state.columns, state.rows, state.depth)
    } else {
        format!("{}x{} flat", state.columns, state.rows)
    };

    ui::column([
        ui::text(title).fill_width().height(16.0),
        ui::text(summary).fill_width().height(16.0),
    ])
    .style(ui::WidgetStyle {
        tone: ui::WidgetTone::Accent,
        prominence: ui::WidgetProminence::Subtle,
    })
    .fill_width()
    .fill_height()
    .padding(4.0)
    .spacing(2.0)
}

pub(crate) fn grid_is_crowded(state: &LayoutDemoState) -> bool {
    state.columns >= 4
        || state.rows >= 4
        || state.columns * state.rows >= 9
        || (state.show_nested && state.depth >= 2 && state.columns >= 3)
        || (state.show_nested && state.depth >= 2 && state.columns * state.rows >= 6)
}

pub(crate) fn grid_spacing(state: &LayoutDemoState) -> f32 {
    if grid_is_crowded(state) { 4.0 } else { 8.0 }
}

fn nested_layout(depth: usize, seed: usize) -> ui::StateView<LayoutDemoState> {
    if depth == 0 {
        return fill_tile(format!("Leaf fill {}", seed + 1));
    }

    if (depth + seed).is_multiple_of(2) {
        ui::row([
            fixed_size_tile(format!("W{}", depth), 54.0 + depth as f32 * 12.0, 40.0),
            nested_layout(depth - 1, seed + 1),
            fill_tile(format!("Fill R{}", depth)),
        ])
        .fill_width()
        .fill_height()
        .spacing(8.0)
    } else {
        ui::column([
            fixed_tile(format!("H{}", depth), 30.0 + depth as f32 * 4.0),
            nested_layout(depth - 1, seed + 1),
            fill_tile(format!("Fill C{}", depth)),
        ])
        .fill_width()
        .fill_height()
        .spacing(8.0)
    }
}
