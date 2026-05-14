use super::*;
use crate::gui::types::{Point, Rect, Vector2};

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
fn anchored_panel_rect_clamps_anchor_inside_inset_bounds() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 160.0));

    assert_eq!(
        anchored_panel_rect(
            bounds,
            Point::new(250.0, 0.0),
            Vector2::new(80.0, 40.0),
            8.0,
        ),
        Rect::from_min_max(Point::new(122.0, 28.0), Point::new(202.0, 68.0))
    );
}

#[test]
fn anchored_panel_rect_keeps_size_when_bounds_are_cramped() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 50.0));

    assert_eq!(
        anchored_panel_rect(
            bounds,
            Point::new(24.0, 32.0),
            Vector2::new(80.0, 40.0),
            8.0,
        ),
        Rect::from_min_max(Point::new(18.0, 28.0), Point::new(98.0, 68.0))
    );
}

#[test]
fn split_pane_assigned_row_preserves_labels_and_assignments() {
    let row =
        SplitPaneAssignedRow::new("Inbox", "ready", true, false).with_pane_assignment(true, false);

    assert_eq!(row.label, "Inbox");
    assert_eq!(row.detail, "ready");
    assert!(row.selected);
    assert!(!row.missing);
    assert!(row.assigned_to_upper_pane);
    assert!(!row.assigned_to_lower_pane);
}

#[test]
fn split_pane_tree_panel_defaults_to_empty_unassigned_panel() {
    let panel: SplitPaneTreePanel = SplitPaneTreePanel::default();

    assert_eq!(panel.pane, SplitPaneSlot::Upper);
    assert!(!panel.active);
    assert!(!panel.has_item);
    assert!(panel.tree_rows.is_empty());
    assert_eq!(panel.focused_tree_row, None);
}

#[test]
fn split_pane_sidebar_state_routes_active_pane() {
    let mut sidebar: SplitPaneSidebarState = SplitPaneSidebarState {
        active_pane: SplitPaneSlot::Lower,
        lower_pane: SplitPaneTreePanel {
            title: String::from("Lower"),
            ..SplitPaneTreePanel::default()
        },
        ..SplitPaneSidebarState::default()
    };

    assert_eq!(sidebar.active_pane_model().title, "Lower");
    sidebar.pane_mut(SplitPaneSlot::Upper).title = String::from("Upper");
    sidebar.active_pane_model_mut().item_label = String::from("Active");

    assert_eq!(sidebar.upper_pane.title, "Upper");
    assert_eq!(sidebar.lower_pane.item_label, "Active");
}
