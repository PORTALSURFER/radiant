//! Split-pane workspace model for editor-style surfaces.

use radiant::gui::{
    panel::{SplitPaneAssignedRow, SplitPaneSidebarState, SplitPaneSlot, SplitPaneTreePanel},
    retained::RetainedVec,
};
use radiant::prelude as ui;

#[derive(Clone, Debug)]
struct WorkspaceState {
    sidebar: SplitPaneSidebarState,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        let rows = vec![
            SplitPaneAssignedRow::new("Scene", "ready", true, false)
                .with_pane_assignment(true, false),
            SplitPaneAssignedRow::new("Inspector", "editing", false, false)
                .with_pane_assignment(false, true),
            SplitPaneAssignedRow::new("Console", "idle", false, false),
        ];
        Self {
            sidebar: SplitPaneSidebarState {
                header: String::from("Workspace"),
                active_pane: SplitPaneSlot::Upper,
                selected_row: Some(0),
                rows: RetainedVec::from(rows),
                upper_pane: pane_model(SplitPaneSlot::Upper, "Upper", "Scene", true),
                lower_pane: pane_model(SplitPaneSlot::Lower, "Lower", "Inspector", false),
                ..SplitPaneSidebarState::default()
            },
        }
    }
}

impl WorkspaceState {
    fn select_row(&mut self, index: usize) {
        self.sidebar.selected_row = Some(index);
        for (row_index, row) in self.sidebar.rows.make_mut().iter_mut().enumerate() {
            row.selected = row_index == index;
        }
    }

    fn assign_selected_to(&mut self, pane: SplitPaneSlot) {
        let Some(index) = self.sidebar.selected_row else {
            return;
        };
        let Some(row) = self.sidebar.rows.get_mut(index) else {
            return;
        };
        match pane {
            SplitPaneSlot::Upper => row.assigned_to_upper_pane = true,
            SplitPaneSlot::Lower => row.assigned_to_lower_pane = true,
        }
        let label = row.label.clone();
        let detail = row.detail.clone();
        let active = self.sidebar.active_pane == pane;
        *self.sidebar.pane_mut(pane) = pane_model(
            pane,
            pane_label(pane),
            format!("{label} / {detail}"),
            active,
        );
    }

    fn activate_pane(&mut self, pane: SplitPaneSlot) {
        self.sidebar.active_pane = pane;
        self.sidebar.upper_pane.active = false;
        self.sidebar.lower_pane.active = false;
        self.sidebar.active_pane_model_mut().active = true;
    }
}

fn main() -> radiant::Result {
    radiant::app(WorkspaceState::default())
        .title("Radiant Split Workspace")
        .size(760, 420)
        .min_size(560, 320)
        .view(project_surface)
        .run()
}

fn project_surface(state: &mut WorkspaceState) -> ui::StateView<WorkspaceState> {
    ui::row([sidebar_view(state).width(240.0), panes_view(state).fill()])
        .padding(12.0)
        .spacing(10.0)
        .fill()
}

fn sidebar_view(state: &WorkspaceState) -> ui::StateView<WorkspaceState> {
    ui::column([
        ui::text(state.sidebar.header.clone())
            .height(28.0)
            .fill_width(),
        ui::list(
            state.sidebar.rows.as_slice().iter().cloned().enumerate(),
            |(index, row)| assignment_row(index, row),
        )
        .fill_height(),
        ui::row([
            ui::button("Assign upper")
                .on_click(|state: &mut WorkspaceState| {
                    state.assign_selected_to(SplitPaneSlot::Upper)
                })
                .fill_width(),
            ui::button("Assign lower")
                .on_click(|state: &mut WorkspaceState| {
                    state.assign_selected_to(SplitPaneSlot::Lower)
                })
                .fill_width(),
        ])
        .spacing(8.0),
    ])
    .style(ui::WidgetStyle::default())
    .padding(10.0)
    .spacing(8.0)
    .fill_height()
}

fn assignment_row(index: usize, row: SplitPaneAssignedRow) -> ui::StateView<WorkspaceState> {
    let assignment = assignment_label(&row);
    let mut item = ui::list_row(
        index,
        [
            ui::button(row.label)
                .on_click(move |state: &mut WorkspaceState| state.select_row(index))
                .fill_width(),
            ui::text(assignment).size(64.0, 24.0),
        ],
    );
    if row.selected {
        item = item.primary();
    }
    item
}

fn panes_view(state: &WorkspaceState) -> ui::StateView<WorkspaceState> {
    ui::column([
        pane_view(state.sidebar.pane(SplitPaneSlot::Upper).clone()).fill_height(),
        pane_view(state.sidebar.pane(SplitPaneSlot::Lower).clone()).fill_height(),
    ])
    .spacing(10.0)
    .fill()
}

fn pane_view(pane: SplitPaneTreePanel) -> ui::StateView<WorkspaceState> {
    let pane_id = pane.pane;
    let mut view = ui::column([
        ui::row([
            ui::text(format!("{} pane", pane.title))
                .height(26.0)
                .fill_width(),
            ui::button("Activate")
                .on_click(move |state: &mut WorkspaceState| state.activate_pane(pane_id)),
        ])
        .fill_width()
        .spacing(8.0),
        ui::text(if pane.has_item {
            format!("Assigned: {}", pane.item_label)
        } else {
            String::from("No assignment")
        })
        .height(28.0)
        .fill_width(),
        ui::text(if pane.active {
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
    if pane.active {
        view = view.primary();
    }
    view
}

fn pane_model(
    pane: SplitPaneSlot,
    title: impl Into<String>,
    item_label: impl Into<String>,
    active: bool,
) -> SplitPaneTreePanel {
    let item_label = item_label.into();
    SplitPaneTreePanel {
        pane,
        title: title.into(),
        item_detail: String::from("assigned"),
        has_item: !item_label.is_empty(),
        item_label,
        active,
        ..SplitPaneTreePanel::default()
    }
}

fn pane_label(pane: SplitPaneSlot) -> &'static str {
    match pane {
        SplitPaneSlot::Upper => "Upper",
        SplitPaneSlot::Lower => "Lower",
    }
}

fn assignment_label(row: &SplitPaneAssignedRow) -> String {
    match (
        row.assigned_to_upper_pane,
        row.assigned_to_lower_pane,
        row.missing,
    ) {
        (_, _, true) => String::from("missing"),
        (true, true, false) => String::from("both"),
        (true, false, false) => String::from("upper"),
        (false, true, false) => String::from("lower"),
        (false, false, false) => String::from("free"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_workspace_assigns_selected_rows_to_panes() {
        let mut state = WorkspaceState::default();
        state.select_row(2);
        state.assign_selected_to(SplitPaneSlot::Lower);
        state.activate_pane(SplitPaneSlot::Lower);

        assert_eq!(state.sidebar.selected_row, Some(2));
        assert!(state.sidebar.rows[2].assigned_to_lower_pane);
        assert_eq!(
            state.sidebar.active_pane_model().item_label,
            "Console / idle"
        );
        assert!(state.sidebar.lower_pane.active);
    }
}
