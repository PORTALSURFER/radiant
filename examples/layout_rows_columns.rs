//! Dynamic row and column panels showing fill behavior on both axes.

use radiant::prelude as ui;

const MIN_COLUMNS: usize = 1;
const MAX_COLUMNS: usize = 5;
const MIN_ROWS: usize = 1;
const MAX_ROWS: usize = 5;
const MIN_DEPTH: usize = 0;
const MAX_DEPTH: usize = 4;

#[derive(Clone, Debug)]
struct LayoutDemoState {
    show_sidebar: bool,
    show_nested: bool,
    columns: usize,
    rows: usize,
    depth: usize,
}

impl Default for LayoutDemoState {
    fn default() -> Self {
        Self {
            show_sidebar: true,
            show_nested: true,
            columns: 3,
            rows: 2,
            depth: 2,
        }
    }
}

fn main() -> radiant::Result {
    radiant::app(LayoutDemoState::default())
        .title("Radiant Rows and Columns")
        .size(860, 620)
        .min_size(620, 420)
        .view(project_surface)
        .run()
}

fn project_surface(state: &mut LayoutDemoState) -> ui::StateView<LayoutDemoState> {
    ui::column([header(), controls(state), playground(state)])
        .padding(16.0)
        .spacing(12.0)
        .fill_width()
        .fill_height()
}

fn header() -> ui::StateView<LayoutDemoState> {
    ui::column([
        ui::text("Rows and columns").size(240.0, 30.0),
        ui::text(
            "Dynamic splits rebuild the tree while fixed cells, fill cells, nested rows, and nested columns compete for space.",
        )
        .fill_width()
        .height(34.0),
    ])
    .spacing(4.0)
    .fill_width()
}

fn controls(state: &LayoutDemoState) -> ui::StateView<LayoutDemoState> {
    ui::column([
        ui::row([
            ui::checkbox(state.show_sidebar)
                .on_change(|state: &mut LayoutDemoState, show| state.show_sidebar = show)
                .size(22.0, 22.0),
            ui::text("Sidebar").size(82.0, 24.0),
            ui::checkbox(state.show_nested)
                .on_change(|state: &mut LayoutDemoState, show| state.show_nested = show)
                .size(22.0, 22.0),
            ui::text("Nested cells").size(112.0, 24.0),
            ui::text(format!("Grid: {} x {}", state.columns, state.rows))
                .fill_width()
                .height(24.0),
        ])
        .fill_width()
        .spacing(8.0),
        ui::row([
            control_button("Columns -", |state| {
                state.columns = state.columns.saturating_sub(1).max(MIN_COLUMNS);
            }),
            control_button("Columns +", |state| {
                state.columns = (state.columns + 1).min(MAX_COLUMNS);
            }),
            control_button("Rows -", |state| {
                state.rows = state.rows.saturating_sub(1).max(MIN_ROWS);
            }),
            control_button("Rows +", |state| {
                state.rows = (state.rows + 1).min(MAX_ROWS);
            }),
            control_button("Depth -", |state| {
                state.depth = state.depth.saturating_sub(1).max(MIN_DEPTH);
            }),
            control_button("Depth +", |state| {
                state.depth = (state.depth + 1).min(MAX_DEPTH);
            }),
        ])
        .fill_width()
        .spacing(8.0),
    ])
    .style(ui::WidgetStyle::default())
    .fill_width()
    .height(92.0)
    .padding(12.0)
    .spacing(10.0)
}

fn control_button(
    label: &'static str,
    apply: impl Fn(&mut LayoutDemoState) + Send + Sync + 'static,
) -> ui::StateView<LayoutDemoState> {
    ui::button(label)
        .on_click(apply)
        .min_size(86.0, 32.0)
        .preferred_size(108.0, 32.0)
        .fill_width()
}

fn playground(state: &LayoutDemoState) -> ui::StateView<LayoutDemoState> {
    let mut sections = Vec::new();
    if state.show_sidebar {
        sections.push(sidebar_panel(state));
    }
    sections.push(main_grid_panel(state));

    ui::row(sections)
        .style(ui::WidgetStyle::default())
        .fill_width()
        .fill_height()
        .padding(12.0)
        .spacing(12.0)
}

fn sidebar_panel(state: &LayoutDemoState) -> ui::StateView<LayoutDemoState> {
    panel(
        "Fixed sidebar",
        ui::column([
            fixed_tile("Fixed header", 34.0),
            ui::column([
                metric_row("Columns", state.columns),
                metric_row("Rows", state.rows),
                metric_row("Depth", state.depth),
            ])
            .fill_width()
            .height(120.0)
            .spacing(8.0),
            fill_tile("Vertical fill"),
            fixed_tile("Fixed footer", 34.0),
        ])
        .fill_width()
        .fill_height()
        .spacing(8.0),
    )
    .size(210.0, 320.0)
}

fn metric_row(label: &'static str, value: usize) -> ui::StateView<LayoutDemoState> {
    ui::row([
        ui::text(label).fill_width().height(30.0),
        ui::text(value.to_string()).size(42.0, 30.0),
    ])
    .style(ui::WidgetStyle::default())
    .fill_width()
    .height(30.0)
    .padding_x(8.0)
    .spacing(8.0)
}

fn main_grid_panel(state: &LayoutDemoState) -> ui::StateView<LayoutDemoState> {
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
    } else if (row_index + column_index) % 2 == 0 {
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

fn grid_is_crowded(state: &LayoutDemoState) -> bool {
    state.columns >= 4
        || state.rows >= 4
        || state.columns * state.rows >= 9
        || (state.show_nested && state.depth >= 2 && state.columns >= 3)
        || (state.show_nested && state.depth >= 2 && state.columns * state.rows >= 6)
}

fn grid_spacing(state: &LayoutDemoState) -> f32 {
    if grid_is_crowded(state) { 4.0 } else { 8.0 }
}

fn nested_layout(depth: usize, seed: usize) -> ui::StateView<LayoutDemoState> {
    if depth == 0 {
        return fill_tile(format!("Leaf fill {}", seed + 1));
    }

    if (depth + seed) % 2 == 0 {
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

fn panel(
    title: impl Into<String>,
    content: ui::StateView<LayoutDemoState>,
) -> ui::StateView<LayoutDemoState> {
    ui::column([ui::text(title).fill_width().height(24.0), content])
        .style(ui::WidgetStyle {
            tone: ui::WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        })
        .fill_width()
        .fill_height()
        .padding(10.0)
        .spacing(8.0)
}

fn fixed_tile(label: impl Into<String>, height: f32) -> ui::StateView<LayoutDemoState> {
    ui::text(label)
        .style(ui::WidgetStyle::default())
        .fill_width()
        .height(height)
}

fn fixed_size_tile(
    label: impl Into<String>,
    width: f32,
    height: f32,
) -> ui::StateView<LayoutDemoState> {
    ui::text(label)
        .style(ui::WidgetStyle::default())
        .size(width, height)
}

fn fill_tile(label: impl Into<String>) -> ui::StateView<LayoutDemoState> {
    ui::text(label)
        .style(ui::WidgetStyle::default())
        .fill_width()
        .fill_height()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dense_grids_use_compact_cards() {
        let state = LayoutDemoState {
            columns: 4,
            rows: 5,
            depth: 1,
            ..LayoutDemoState::default()
        };

        assert!(grid_is_crowded(&state));
        assert_eq!(grid_spacing(&state), 4.0);
    }

    #[test]
    fn three_by_three_nested_grids_use_compact_cards() {
        let state = LayoutDemoState {
            columns: 3,
            rows: 3,
            depth: 2,
            show_nested: true,
            ..LayoutDemoState::default()
        };

        assert!(grid_is_crowded(&state));
        assert_eq!(grid_spacing(&state), 4.0);
    }

    #[test]
    fn three_column_deep_nested_grids_use_compact_cards() {
        let state = LayoutDemoState {
            columns: 3,
            rows: 1,
            depth: 2,
            show_nested: true,
            ..LayoutDemoState::default()
        };

        assert!(grid_is_crowded(&state));
        assert_eq!(grid_spacing(&state), 4.0);
    }

    #[test]
    fn sparse_grids_keep_full_nested_layouts() {
        let state = LayoutDemoState {
            columns: 2,
            rows: 2,
            depth: 2,
            ..LayoutDemoState::default()
        };

        assert!(!grid_is_crowded(&state));
        assert_eq!(grid_spacing(&state), 8.0);
    }
}
