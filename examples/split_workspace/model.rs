use radiant::gui::{
    panel::{
        SplitPaneAssignedRow, SplitPaneAssignedRowParts, SplitPaneAssignment,
        SplitPaneSidebarState, SplitPaneSlot, SplitPaneTreePanel,
    },
    retained::RetainedVec,
};

#[derive(Clone, Debug)]
pub(crate) struct WorkspaceState {
    pub(crate) sidebar: SplitPaneSidebarState,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        let rows = vec![
            assigned_row(
                "Scene",
                "ready",
                true,
                SplitPaneAssignment {
                    upper: true,
                    lower: false,
                },
            ),
            assigned_row(
                "Inspector",
                "editing",
                false,
                SplitPaneAssignment {
                    upper: false,
                    lower: true,
                },
            ),
            assigned_row("Console", "idle", false, SplitPaneAssignment::default()),
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
    pub(crate) fn select_row(&mut self, index: usize) {
        self.sidebar.selected_row = Some(index);
        for (row_index, row) in self.sidebar.rows.make_mut().iter_mut().enumerate() {
            row.selected = row_index == index;
        }
    }

    pub(crate) fn assign_selected_to(&mut self, pane: SplitPaneSlot) {
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

    pub(crate) fn activate_pane(&mut self, pane: SplitPaneSlot) {
        self.sidebar.active_pane = pane;
        self.sidebar.upper_pane.active = false;
        self.sidebar.lower_pane.active = false;
        self.sidebar.active_pane_model_mut().active = true;
    }
}

fn assigned_row(
    label: &str,
    detail: &str,
    selected: bool,
    assignment: SplitPaneAssignment,
) -> SplitPaneAssignedRow {
    SplitPaneAssignedRow::from_parts(SplitPaneAssignedRowParts {
        label: String::from(label),
        detail: String::from(detail),
        selected,
        missing: false,
        assignment,
    })
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

pub(crate) fn assignment_label(row: &SplitPaneAssignedRow) -> String {
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
