use crate::gui::panel::{
    SplitPaneAssignedRow, SplitPaneAssignedRowParts, SplitPaneAssignment, SplitPaneAssignmentState,
    SplitPaneSidebarPanes, SplitPaneSidebarState, SplitPaneSlot, SplitPaneTreePanel,
    SplitPaneTreePanelIdentity,
};

#[test]
fn split_pane_slot_defaults_to_upper() {
    assert_eq!(SplitPaneSlot::default(), SplitPaneSlot::Upper);
}

#[test]
fn split_pane_slot_exposes_stable_routing_ids() {
    assert_eq!(SplitPaneSlot::Upper.as_str(), "upper");
    assert_eq!(SplitPaneSlot::Lower.as_str(), "lower");
}

#[test]
fn split_pane_slot_selects_matching_value() {
    assert_eq!(
        SplitPaneSlot::Upper.select(&"leading", &"trailing"),
        &"leading"
    );
    assert_eq!(
        SplitPaneSlot::Lower.select(&"leading", &"trailing"),
        &"trailing"
    );
}

#[test]
fn split_pane_slot_selects_matching_value_mutably() {
    let mut upper = String::from("leading");
    let mut lower = String::from("trailing");

    SplitPaneSlot::Lower
        .select_mut(&mut upper, &mut lower)
        .push_str("-selected");

    assert_eq!(upper, "leading");
    assert_eq!(lower, "trailing-selected");
}

#[test]
fn split_pane_assigned_row_preserves_labels_and_assignments() {
    let row = SplitPaneAssignedRow::from_parts(SplitPaneAssignedRowParts {
        label: String::from("Inbox"),
        detail: String::from("ready"),
        selected: true,
        missing: false,
        assignment: SplitPaneAssignmentState::Upper,
    });

    assert_eq!(row.label, "Inbox");
    assert_eq!(row.detail, "ready");
    assert!(row.selected);
    assert!(!row.missing);
    assert_eq!(row.assignment_state(), SplitPaneAssignmentState::Upper);
    assert!(row.assigned_to_upper_pane);
    assert!(!row.assigned_to_lower_pane);
}

#[test]
fn split_pane_assignment_state_round_trips_compatibility_flags() {
    for (state, upper, lower) in [
        (SplitPaneAssignmentState::Free, false, false),
        (SplitPaneAssignmentState::Upper, true, false),
        (SplitPaneAssignmentState::Lower, false, true),
        (SplitPaneAssignmentState::Both, true, true),
    ] {
        let assignment = SplitPaneAssignment::from_state(state);

        assert_eq!(assignment.upper, upper);
        assert_eq!(assignment.lower, lower);
        assert_eq!(assignment.state(), state);
        assert_eq!(SplitPaneAssignmentState::from_flags(upper, lower), state);
    }
}

#[test]
fn split_pane_assigned_row_assigns_panes_without_exposing_flag_mutation() {
    let mut row = SplitPaneAssignedRow::new("Console", "idle", false, false)
        .with_assignment_state(SplitPaneAssignmentState::Upper);

    row.assign_to_pane(SplitPaneSlot::Lower);

    assert_eq!(row.assignment_state(), SplitPaneAssignmentState::Both);
}

#[test]
fn split_pane_tree_panel_defaults_to_empty_unassigned_panel() {
    let panel: SplitPaneTreePanel = SplitPaneTreePanel::default();

    assert_eq!(panel.identity.pane, SplitPaneSlot::Upper);
    assert!(!panel.assignment.active);
    assert!(!panel.assignment.has_item);
    assert!(panel.content.tree_rows.is_empty());
    assert_eq!(panel.content.focused_tree_row, None);
}

#[test]
fn split_pane_sidebar_state_routes_active_pane() {
    let mut sidebar: SplitPaneSidebarState = SplitPaneSidebarState {
        panes: SplitPaneSidebarPanes {
            active_pane: SplitPaneSlot::Lower,
            lower_pane: SplitPaneTreePanel {
                identity: SplitPaneTreePanelIdentity {
                    title: String::from("Lower"),
                    ..SplitPaneTreePanelIdentity::default()
                },
                ..SplitPaneTreePanel::default()
            },
            ..SplitPaneSidebarPanes::default()
        },
        ..SplitPaneSidebarState::default()
    };

    assert_eq!(sidebar.active_pane_model().identity.title, "Lower");
    sidebar.pane_mut(SplitPaneSlot::Upper).identity.title = String::from("Upper");
    sidebar.active_pane_model_mut().assignment.item_label = String::from("Active");

    assert_eq!(sidebar.panes.upper_pane.identity.title, "Upper");
    assert_eq!(sidebar.panes.lower_pane.assignment.item_label, "Active");
}
