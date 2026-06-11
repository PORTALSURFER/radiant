//! Split-pane workspace model for editor-style surfaces.

#[path = "split_workspace/model.rs"]
mod model;

use model::{WorkspaceState, assignment_label};
use radiant::gui::panel::{SplitPaneAssignedRow, SplitPaneSlot, SplitPaneTreePanel};
use radiant::prelude as ui;

#[derive(Clone, Debug, PartialEq, Eq)]
enum WorkspaceMessage {
    AssignSelectedTo(SplitPaneSlot),
    SelectRow(usize),
    ActivatePane(SplitPaneSlot),
}

fn main() -> radiant::Result {
    radiant::app(WorkspaceState::default())
        .title("Radiant Split Workspace")
        .size(760, 420)
        .min_size(560, 320)
        .view(project_surface)
        .update(update)
        .run()
}

fn project_surface(state: &mut WorkspaceState) -> ui::View<WorkspaceMessage> {
    ui::row([sidebar_view(state).width(240.0), panes_view(state).fill()])
        .padding(12.0)
        .spacing(10.0)
        .fill()
}

fn sidebar_view(state: &WorkspaceState) -> ui::View<WorkspaceMessage> {
    ui::column([
        ui::text(state.sidebar.chrome.header.clone())
            .height(28.0)
            .fill_width(),
        ui::list(
            state
                .sidebar
                .content
                .rows
                .as_slice()
                .iter()
                .cloned()
                .enumerate(),
            |(index, row)| assignment_row(index, row),
        )
        .fill_height(),
        ui::row([
            ui::button("Assign upper")
                .message(WorkspaceMessage::AssignSelectedTo(SplitPaneSlot::Upper))
                .fill_width(),
            ui::button("Assign lower")
                .message(WorkspaceMessage::AssignSelectedTo(SplitPaneSlot::Lower))
                .fill_width(),
        ])
        .spacing(8.0),
    ])
    .style(ui::WidgetStyle::default())
    .padding(10.0)
    .spacing(8.0)
    .fill_height()
}

fn assignment_row(index: usize, row: SplitPaneAssignedRow) -> ui::View<WorkspaceMessage> {
    let assignment = assignment_label(&row);
    let mut item = ui::list_row(
        index,
        [
            ui::button(row.label)
                .message(WorkspaceMessage::SelectRow(index))
                .fill_width(),
            ui::text(assignment).size(64.0, 24.0),
        ],
    );
    if row.selected {
        item = item.primary();
    }
    item
}

fn panes_view(state: &WorkspaceState) -> ui::View<WorkspaceMessage> {
    ui::column([
        pane_view(state.sidebar.pane(SplitPaneSlot::Upper).clone()).fill_height(),
        pane_view(state.sidebar.pane(SplitPaneSlot::Lower).clone()).fill_height(),
    ])
    .spacing(10.0)
    .fill()
}

fn pane_view(pane: SplitPaneTreePanel) -> ui::View<WorkspaceMessage> {
    let pane_id = pane.identity.pane;
    let mut view = ui::column([
        ui::row([
            ui::text(format!("{} pane", pane.identity.title))
                .height(26.0)
                .fill_width(),
            ui::button("Activate").message(WorkspaceMessage::ActivatePane(pane_id)),
        ])
        .fill_width()
        .spacing(8.0),
        ui::text(if pane.assignment.has_item {
            format!("Assigned: {}", pane.assignment.item_label)
        } else {
            String::from("No assignment")
        })
        .height(28.0)
        .fill_width(),
        ui::text(if pane.assignment.active {
            "This pane drives the active content surface"
        } else {
            "Inactive pane remains visible"
        })
        .height(24.0)
        .fill_width(),
    ])
    .style(ui::WidgetStyle::default())
    .padding(12.0)
    .spacing(8.0);
    if pane.assignment.active {
        view = view.primary();
    }
    view
}

fn update(state: &mut WorkspaceState, message: WorkspaceMessage) {
    match message {
        WorkspaceMessage::AssignSelectedTo(slot) => state.assign_selected_to(slot),
        WorkspaceMessage::SelectRow(index) => state.select_row(index),
        WorkspaceMessage::ActivatePane(slot) => state.activate_pane(slot),
    }
}
