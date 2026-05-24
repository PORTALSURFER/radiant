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
        anchored_panel_rect_from_parts(AnchoredPanelRectParts {
            bounds,
            anchor: Point::new(250.0, 0.0),
            size: Vector2::new(80.0, 40.0),
            inset: 8.0,
        }),
        Rect::from_min_max(Point::new(122.0, 28.0), Point::new(202.0, 68.0))
    );
}

#[test]
fn anchored_panel_rect_compatibility_helper_delegates_to_named_parts() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 160.0));
    let from_parts = anchored_panel_rect_from_parts(AnchoredPanelRectParts {
        bounds,
        anchor: Point::new(250.0, 0.0),
        size: Vector2::new(80.0, 40.0),
        inset: 8.0,
    });

    assert_eq!(
        anchored_panel_rect(
            bounds,
            Point::new(250.0, 0.0),
            Vector2::new(80.0, 40.0),
            8.0,
        ),
        from_parts
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
fn anchored_panel_rect_sanitizes_nonfinite_geometry_inputs() {
    let rect = anchored_panel_rect(
        Rect::from_min_max(
            Point::new(f32::NAN, 20.0),
            Point::new(f32::NAN, f32::INFINITY),
        ),
        Point::new(f32::NAN, 40.0),
        Vector2::new(f32::NAN, 24.0),
        f32::NAN,
    );

    assert_eq!(
        rect,
        Rect::from_min_max(Point::new(0.0, 20.0), Point::new(0.0, 44.0))
    );
    assert!(rect.min.x.is_finite());
    assert!(rect.min.y.is_finite());
    assert!(rect.max.x.is_finite());
    assert!(rect.max.y.is_finite());
}

#[test]
fn floating_panel_rect_clamps_origin_inside_bounds() {
    let bounds = Rect::from_min_max(Point::new(0.0, 40.0), Point::new(320.0, 220.0));

    assert_eq!(
        floating_panel_rect_from_parts(FloatingPanelRectParts {
            bounds,
            origin: Point::new(260.0, 10.0),
            size: Vector2::new(100.0, 80.0),
            inset: 12.0,
        }),
        Rect::from_min_max(Point::new(208.0, 52.0), Point::new(308.0, 132.0))
    );
}

#[test]
fn floating_panel_rect_compatibility_helper_delegates_to_named_parts() {
    let bounds = Rect::from_min_max(Point::new(0.0, 40.0), Point::new(320.0, 220.0));
    let from_parts = floating_panel_rect_from_parts(FloatingPanelRectParts {
        bounds,
        origin: Point::new(260.0, 10.0),
        size: Vector2::new(100.0, 80.0),
        inset: 12.0,
    });

    assert_eq!(
        floating_panel_rect(
            bounds,
            Point::new(260.0, 10.0),
            Vector2::new(100.0, 80.0),
            12.0,
        ),
        from_parts
    );
}

#[test]
fn floating_panel_drag_preserves_pointer_grab_offset() {
    let panel = Rect::from_min_size(Point::new(100.0, 80.0), Vector2::new(240.0, 180.0));
    let drag = FloatingPanelDrag::new(panel, Point::new(130.0, 96.0));

    assert_eq!(drag.grab_offset, Vector2::new(30.0, 16.0));
    assert_eq!(
        drag.origin_for_pointer(Point::new(210.0, 140.0)),
        Point::new(180.0, 124.0)
    );
}

#[test]
fn floating_panel_drag_supports_named_parts_construction() {
    let panel = Rect::from_min_size(Point::new(100.0, 80.0), Vector2::new(240.0, 180.0));
    let drag = FloatingPanelDrag::from_parts(FloatingPanelDragParts {
        panel_rect: panel,
        pointer: Point::new(130.0, 96.0),
    });

    assert_eq!(drag.grab_offset, Vector2::new(30.0, 16.0));
}

#[test]
fn floating_panel_drag_sanitizes_nonfinite_pointer_positions() {
    let panel = Rect::from_min_size(Point::new(100.0, 80.0), Vector2::new(240.0, 180.0));
    let drag = FloatingPanelDrag::new(panel, Point::new(f32::NAN, f32::INFINITY));

    assert_eq!(drag.grab_offset, Vector2::new(0.0, 0.0));
    assert_eq!(
        drag.origin_for_pointer(Point::new(f32::NAN, 140.0)),
        Point::new(0.0, 140.0)
    );
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
