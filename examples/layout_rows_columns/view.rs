use super::*;
use crate::model::{LayoutDemoState, MAX_COLUMNS, MAX_DEPTH, MAX_ROWS, MIN_COLUMNS, MIN_ROWS};

#[path = "view/grid.rs"]
mod grid;

#[cfg(test)]
pub(super) use grid::{grid_is_crowded, grid_spacing};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum LayoutDemoMessage {
    SetSidebarVisible(bool),
    SetNestedVisible(bool),
    AdjustColumns(isize),
    AdjustRows(isize),
    AdjustDepth(isize),
}

pub(super) fn project_surface(state: &LayoutDemoState) -> ui::View<LayoutDemoMessage> {
    ui::column([header(), controls(state), playground(state)])
        .padding(16.0)
        .spacing(12.0)
        .fill_width()
        .fill_height()
}

fn header() -> ui::View<LayoutDemoMessage> {
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

fn controls(state: &LayoutDemoState) -> ui::View<LayoutDemoMessage> {
    ui::column([
        ui::row([
            ui::checkbox(state.show_sidebar)
                .message(LayoutDemoMessage::SetSidebarVisible)
                .size(22.0, 22.0),
            ui::text("Sidebar").size(82.0, 24.0),
            ui::checkbox(state.show_nested)
                .message(LayoutDemoMessage::SetNestedVisible)
                .size(22.0, 22.0),
            ui::text("Nested cells").size(112.0, 24.0),
            ui::text(format!("Grid: {} x {}", state.columns, state.rows))
                .fill_width()
                .height(24.0),
        ])
        .fill_width()
        .spacing(8.0),
        ui::row([
            control_button("Columns -", LayoutDemoMessage::AdjustColumns(-1)),
            control_button("Columns +", LayoutDemoMessage::AdjustColumns(1)),
            control_button("Rows -", LayoutDemoMessage::AdjustRows(-1)),
            control_button("Rows +", LayoutDemoMessage::AdjustRows(1)),
            control_button("Depth -", LayoutDemoMessage::AdjustDepth(-1)),
            control_button("Depth +", LayoutDemoMessage::AdjustDepth(1)),
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

fn control_button(label: &'static str, message: LayoutDemoMessage) -> ui::View<LayoutDemoMessage> {
    ui::button(label)
        .message(message)
        .min_size(86.0, 32.0)
        .preferred_size(108.0, 32.0)
        .fill_width()
}

fn playground(state: &LayoutDemoState) -> ui::View<LayoutDemoMessage> {
    let mut sections = Vec::new();
    if state.show_sidebar {
        sections.push(sidebar_panel(state));
    }
    sections.push(grid::main_grid_panel(state));

    ui::row(sections)
        .style(ui::WidgetStyle::default())
        .fill_width()
        .fill_height()
        .padding(12.0)
        .spacing(12.0)
}

fn sidebar_panel(state: &LayoutDemoState) -> ui::View<LayoutDemoMessage> {
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

fn metric_row(label: &'static str, value: usize) -> ui::View<LayoutDemoMessage> {
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

fn panel(
    title: impl Into<ui::TextContent>,
    content: ui::View<LayoutDemoMessage>,
) -> ui::View<LayoutDemoMessage> {
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

fn fixed_tile(label: impl Into<ui::TextContent>, height: f32) -> ui::View<LayoutDemoMessage> {
    ui::text(label)
        .style(ui::WidgetStyle::default())
        .fill_width()
        .height(height)
}

fn fixed_size_tile(
    label: impl Into<ui::TextContent>,
    width: f32,
    height: f32,
) -> ui::View<LayoutDemoMessage> {
    ui::text(label)
        .style(ui::WidgetStyle::default())
        .size(width, height)
}

fn fill_tile(label: impl Into<ui::TextContent>) -> ui::View<LayoutDemoMessage> {
    ui::text(label)
        .style(ui::WidgetStyle::default())
        .fill_width()
        .fill_height()
}

pub(super) fn update(state: &mut LayoutDemoState, message: LayoutDemoMessage) {
    match message {
        LayoutDemoMessage::SetSidebarVisible(show) => state.show_sidebar = show,
        LayoutDemoMessage::SetNestedVisible(show) => state.show_nested = show,
        LayoutDemoMessage::AdjustColumns(delta) => {
            state.columns = adjust_bounded(state.columns, delta, MIN_COLUMNS, MAX_COLUMNS);
        }
        LayoutDemoMessage::AdjustRows(delta) => {
            state.rows = adjust_bounded(state.rows, delta, MIN_ROWS, MAX_ROWS);
        }
        LayoutDemoMessage::AdjustDepth(delta) => {
            state.depth = adjust_bounded(state.depth, delta, 0, MAX_DEPTH);
        }
    }
}

fn adjust_bounded(value: usize, delta: isize, min: usize, max: usize) -> usize {
    value.saturating_add_signed(delta).clamp(min, max)
}
