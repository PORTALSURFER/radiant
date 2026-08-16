use crate::gui::panel::{
    SplitPaneAssignedRow, SplitPaneAssignedRowParts, SplitPaneAssignment, SplitPaneAssignmentState,
    SplitPaneAxis, SplitPaneCollapsePolicy, SplitPaneLayout, SplitPaneLayoutParts,
    SplitPaneSidebarPanes, SplitPaneSidebarState, SplitPaneSlot, SplitPaneTreePanel,
    SplitPaneTreePanelIdentity,
};
use crate::gui::types::{Point, Rect};

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

#[test]
fn split_pane_layout_resolves_horizontal_geometry() {
    let layout = SplitPaneLayout::from_parts(SplitPaneLayoutParts {
        bounds: Rect::from_min_max(Point::new(0.0, 10.0), Point::new(200.0, 110.0)),
        axis: SplitPaneAxis::Horizontal,
        ratio: 0.25,
        divider_extent: 8.0,
        first_min_extent: 40.0,
        second_min_extent: 60.0,
    });

    assert_eq!(
        layout.first,
        Rect::from_min_max(Point::new(0.0, 10.0), Point::new(48.0, 110.0))
    );
    assert_eq!(
        layout.divider,
        Rect::from_min_max(Point::new(48.0, 10.0), Point::new(56.0, 110.0))
    );
    assert_eq!(
        layout.second,
        Rect::from_min_max(Point::new(56.0, 10.0), Point::new(200.0, 110.0))
    );
    assert_eq!(layout.divider_extent, 8.0);
    assert!(layout.minima_satisfied);
}

#[test]
fn split_pane_layout_resolves_vertical_geometry() {
    let layout = SplitPaneLayout::new(
        Rect::from_min_max(Point::new(20.0, 30.0), Point::new(220.0, 150.0)),
        SplitPaneAxis::Vertical,
        0.4,
        20.0,
        30.0,
        40.0,
    );

    assert_eq!(
        layout.first,
        Rect::from_min_max(Point::new(20.0, 30.0), Point::new(220.0, 70.0))
    );
    assert_eq!(
        layout.divider,
        Rect::from_min_max(Point::new(20.0, 70.0), Point::new(220.0, 90.0))
    );
    assert_eq!(
        layout.second,
        Rect::from_min_max(Point::new(20.0, 90.0), Point::new(220.0, 150.0))
    );
    assert!(layout.minima_satisfied);
}

#[test]
fn split_pane_layout_clamps_ratio_and_enforces_minimums() {
    let low = SplitPaneLayout::from_parts(SplitPaneLayoutParts {
        bounds: Rect::from_size(200.0, 40.0),
        axis: SplitPaneAxis::Horizontal,
        ratio: -1.0,
        divider_extent: 20.0,
        first_min_extent: 60.0,
        second_min_extent: 50.0,
    });
    let high = SplitPaneLayout::from_parts(SplitPaneLayoutParts {
        ratio: 2.0,
        ..SplitPaneLayoutParts {
            bounds: Rect::from_size(200.0, 40.0),
            axis: SplitPaneAxis::Horizontal,
            ratio: 0.0,
            divider_extent: 20.0,
            first_min_extent: 60.0,
            second_min_extent: 50.0,
        }
    });

    assert_eq!(low.ratio, 0.0);
    assert_eq!(low.first.width(), 60.0);
    assert_eq!(low.second.width(), 120.0);
    assert!(low.minima_satisfied);
    assert_eq!(high.ratio, 1.0);
    assert_eq!(high.first.width(), 130.0);
    assert_eq!(high.second.width(), 50.0);
    assert!(high.minima_satisfied);
}

#[test]
fn split_pane_layout_clamps_divider_to_bounds() {
    let layout = SplitPaneLayout::from_parts(SplitPaneLayoutParts {
        bounds: Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 60.0)),
        axis: SplitPaneAxis::Horizontal,
        ratio: 0.5,
        divider_extent: 100.0,
        first_min_extent: 0.0,
        second_min_extent: 0.0,
    });

    assert_eq!(layout.divider_extent, 40.0);
    assert_eq!(
        layout.first,
        Rect::from_min_max(Point::new(10.0, 20.0), Point::new(10.0, 60.0))
    );
    assert_eq!(layout.divider, layout.bounds);
    assert_eq!(
        layout.second,
        Rect::from_min_max(Point::new(50.0, 20.0), Point::new(50.0, 60.0))
    );
    assert!(layout.minima_satisfied);
}

#[test]
fn split_pane_layout_reports_undersized_minimum_fallback() {
    let layout = SplitPaneLayout::from_parts(SplitPaneLayoutParts {
        bounds: Rect::from_size(100.0, 40.0),
        axis: SplitPaneAxis::Horizontal,
        ratio: 0.25,
        divider_extent: 20.0,
        first_min_extent: 60.0,
        second_min_extent: 60.0,
    });

    assert_eq!(layout.first.width(), 20.0);
    assert_eq!(layout.divider.width(), 20.0);
    assert_eq!(layout.second.width(), 60.0);
    assert!(!layout.minima_satisfied);
    assert_eq!(
        layout.first.union(layout.divider).union(layout.second),
        layout.bounds
    );
}

#[test]
fn split_pane_layout_sanitizes_nonfinite_inputs() {
    let layout = SplitPaneLayout::from_parts(SplitPaneLayoutParts {
        bounds: Rect::from_min_max(Point::new(20.0, 30.0), Point::new(120.0, 70.0)),
        axis: SplitPaneAxis::Vertical,
        ratio: f32::NAN,
        divider_extent: f32::INFINITY,
        first_min_extent: f32::NEG_INFINITY,
        second_min_extent: f32::NAN,
    });

    assert_eq!(layout.ratio, 0.5);
    assert_eq!(layout.divider_extent, 0.0);
    assert_eq!(layout.first_min_extent, 0.0);
    assert_eq!(layout.second_min_extent, 0.0);
    assert!(layout.minima_satisfied);
    for rect in [layout.bounds, layout.first, layout.divider, layout.second] {
        assert!(rect.is_finite());
        assert!(rect.width() >= 0.0);
        assert!(rect.height() >= 0.0);
    }
}

#[test]
fn split_pane_collapse_targets_follow_minimums_and_quantization() {
    let parts = SplitPaneLayoutParts {
        bounds: Rect::from_size(200.0, 100.0),
        axis: SplitPaneAxis::Horizontal,
        ratio: 0.5,
        divider_extent: 8.0,
        first_min_extent: 40.0,
        second_min_extent: 60.0,
    };
    let first = super::super::split_pane_collapse_target(parts, SplitPaneCollapsePolicy::FirstPane)
        .expect("positive horizontal split has a first-pane target");
    let second =
        super::super::split_pane_collapse_target(parts, SplitPaneCollapsePolicy::SecondPane)
            .expect("positive horizontal split has a second-pane target");
    assert_eq!(first.ratio, 40.0 / 192.0);
    assert_eq!(first.selected_extent, 40.0);
    assert_eq!(second.ratio, 132.0 / 192.0);
    assert_eq!(second.selected_extent, 60.0);

    let vertical = super::super::split_pane_collapse_target(
        SplitPaneLayoutParts {
            bounds: Rect::from_size(100.0, 200.0),
            axis: SplitPaneAxis::Vertical,
            ..parts
        },
        SplitPaneCollapsePolicy::SecondPane,
    )
    .expect("positive vertical split has a second-pane target");
    assert_eq!(vertical.ratio, 132.0 / 192.0);
    assert_eq!(vertical.selected_extent, 60.0);

    let nonfinite = super::super::split_pane_collapse_target(
        SplitPaneLayoutParts {
            bounds: Rect::from_size(100.0, 40.0),
            divider_extent: f32::NAN,
            first_min_extent: f32::INFINITY,
            second_min_extent: f32::NEG_INFINITY,
            ..parts
        },
        SplitPaneCollapsePolicy::FirstPane,
    )
    .expect("nonfinite declared extents are sanitized by the split resolver");
    assert_eq!(nonfinite.ratio, 0.0);
    assert_eq!(nonfinite.selected_extent, 0.0);
}

#[test]
fn split_pane_collapse_targets_fail_closed_for_unsatisfied_minimums() {
    let undersized = SplitPaneLayoutParts {
        bounds: Rect::from_size(100.0, 40.0),
        divider_extent: 20.0,
        first_min_extent: 60.0,
        second_min_extent: 60.0,
        ..SplitPaneLayoutParts::default()
    };
    for policy in [
        SplitPaneCollapsePolicy::FirstPane,
        SplitPaneCollapsePolicy::SecondPane,
    ] {
        assert_eq!(
            super::super::split_pane_collapse_target(undersized, policy),
            None,
            "undersized split must not admit a collapse target"
        );
    }

    let selected_min_exceeds_capacity = SplitPaneLayoutParts {
        bounds: Rect::from_size(100.0, 40.0),
        divider_extent: 20.0,
        first_min_extent: 100.0,
        second_min_extent: 0.0,
        ..SplitPaneLayoutParts::default()
    };
    assert_eq!(
        super::super::split_pane_collapse_target(
            selected_min_exceeds_capacity,
            SplitPaneCollapsePolicy::FirstPane,
        ),
        None
    );
}

#[test]
fn split_pane_layout_rects_stay_normalized_nonoverlapping_and_cover_bounds() {
    for (axis, bounds, ratio, divider_extent, first_min_extent, second_min_extent) in [
        (
            SplitPaneAxis::Horizontal,
            Rect::from_min_max(Point::new(80.0, 20.0), Point::new(10.0, 140.0)),
            0.8,
            12.0,
            24.0,
            36.0,
        ),
        (
            SplitPaneAxis::Vertical,
            Rect::from_min_max(Point::new(30.0, 90.0), Point::new(210.0, 10.0)),
            0.2,
            16.0,
            48.0,
            32.0,
        ),
    ] {
        let layout = SplitPaneLayout::from_parts(SplitPaneLayoutParts {
            bounds,
            axis,
            ratio,
            divider_extent,
            first_min_extent,
            second_min_extent,
        });

        assert_eq!(
            layout.bounds,
            match axis {
                SplitPaneAxis::Horizontal =>
                    Rect::from_min_max(Point::new(10.0, 20.0), Point::new(80.0, 140.0),),
                SplitPaneAxis::Vertical =>
                    Rect::from_min_max(Point::new(30.0, 10.0), Point::new(210.0, 90.0),),
            }
        );
        for rect in [layout.first, layout.divider, layout.second] {
            assert!(rect.is_finite());
            assert!(rect.width() >= 0.0);
            assert!(rect.height() >= 0.0);
            assert!(rect.min.x >= layout.bounds.min.x);
            assert!(rect.min.y >= layout.bounds.min.y);
            assert!(rect.max.x <= layout.bounds.max.x);
            assert!(rect.max.y <= layout.bounds.max.y);
        }
        assert!(!layout.first.overlaps(layout.divider));
        assert!(!layout.divider.overlaps(layout.second));
        assert!(!layout.first.overlaps(layout.second));
        assert_eq!(
            layout.first.union(layout.divider).union(layout.second),
            layout.bounds
        );
    }
}
