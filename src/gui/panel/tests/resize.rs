use crate::{
    gui::{panel::*, types::Point},
    widgets::DragHandleMessage,
};

#[test]
fn panel_resize_drag_grows_from_trailing_edges() {
    let horizontal = PanelResizeDrag::new(PanelResizeEdge::Right, Point::new(100.0, 0.0), 240.0);
    let vertical = PanelResizeDrag::new(PanelResizeEdge::Bottom, Point::new(0.0, 100.0), 120.0);

    assert_eq!(
        horizontal.size_at(Point::new(140.0, 0.0), 48.0, 420.0),
        280.0
    );
    assert_eq!(vertical.size_at(Point::new(0.0, 140.0), 48.0, 420.0), 160.0);
}

#[test]
fn panel_resize_drag_grows_from_leading_edges() {
    let horizontal = PanelResizeDrag::new(PanelResizeEdge::Left, Point::new(100.0, 0.0), 240.0);
    let vertical = PanelResizeDrag::new(PanelResizeEdge::Top, Point::new(0.0, 100.0), 120.0);

    assert_eq!(
        horizontal.size_at(Point::new(60.0, 0.0), 48.0, 420.0),
        280.0
    );
    assert_eq!(vertical.size_at(Point::new(0.0, 60.0), 48.0, 420.0), 160.0);
}

#[test]
fn panel_resize_drag_clamps_size() {
    let drag = PanelResizeDrag::new(PanelResizeEdge::Right, Point::new(100.0, 0.0), 240.0);

    assert_eq!(drag.size_at(Point::new(-300.0, 0.0), 48.0, 420.0), 48.0);
    assert_eq!(drag.size_at(Point::new(500.0, 0.0), 48.0, 420.0), 420.0);
}

#[test]
fn update_panel_resize_drag_manages_drag_lifecycle() {
    let mut drag = None;

    assert_eq!(
        update_panel_resize_drag(
            &mut drag,
            DragHandleMessage::started(Point::new(100.0, 0.0)),
            PanelResizeEdge::Right,
            240.0,
            48.0,
            420.0,
        ),
        None
    );
    assert!(drag.is_some());

    assert_eq!(
        update_panel_resize_drag(
            &mut drag,
            DragHandleMessage::Moved {
                position: Point::new(140.0, 0.0)
            },
            PanelResizeEdge::Right,
            240.0,
            48.0,
            420.0,
        ),
        Some(280.0)
    );
    assert!(drag.is_some());

    assert_eq!(
        update_panel_resize_drag(
            &mut drag,
            DragHandleMessage::Ended {
                position: Point::new(200.0, 0.0)
            },
            PanelResizeEdge::Right,
            240.0,
            48.0,
            420.0,
        ),
        Some(340.0)
    );
    assert_eq!(drag, None);
}

#[test]
fn update_panel_resize_drag_ignores_orphaned_motion() {
    let mut drag = None;

    assert_eq!(
        update_panel_resize_drag(
            &mut drag,
            DragHandleMessage::Moved {
                position: Point::new(140.0, 0.0)
            },
            PanelResizeEdge::Right,
            240.0,
            48.0,
            420.0,
        ),
        None
    );
}

#[test]
fn update_collapsible_panel_resize_drag_collapses_on_double_activate() {
    let mut drag = Some(PanelResizeDrag::new(
        PanelResizeEdge::Top,
        Point::new(0.0, 120.0),
        180.0,
    ));

    assert_eq!(
        update_collapsible_panel_resize_drag(
            &mut drag,
            DragHandleMessage::DoubleActivate {
                position: Point::new(0.0, 120.0)
            },
            PanelResizeEdge::Top,
            180.0,
            72.0,
            240.0,
            48.0,
        ),
        Some(72.0)
    );
    assert_eq!(drag, None);
}

#[test]
fn update_collapsible_panel_resize_drag_preserves_normal_resize_lifecycle() {
    let mut drag = None;

    assert_eq!(
        update_collapsible_panel_resize_drag(
            &mut drag,
            DragHandleMessage::started(Point::new(0.0, 120.0)),
            PanelResizeEdge::Top,
            148.0,
            72.0,
            240.0,
            72.0,
        ),
        None
    );
    assert!(drag.is_some());
    assert_eq!(
        update_collapsible_panel_resize_drag(
            &mut drag,
            DragHandleMessage::Moved {
                position: Point::new(0.0, 80.0)
            },
            PanelResizeEdge::Top,
            148.0,
            72.0,
            240.0,
            72.0,
        ),
        Some(188.0)
    );
    assert!(drag.is_some());
}

#[test]
fn panel_resize_state_updates_durable_size_and_drag_lifecycle() {
    let mut state = PanelResizeState::new(240.0);
    let constraints = PanelResizeConstraints::new(PanelResizeEdge::Right, 48.0, 420.0);

    assert_eq!(
        state.resize(
            DragHandleMessage::started(Point::new(100.0, 0.0)),
            constraints,
        ),
        None
    );
    assert_eq!(state.size(), 240.0);
    assert!(state.is_resizing());

    assert_eq!(
        state.resize(
            DragHandleMessage::Moved {
                position: Point::new(160.0, 0.0)
            },
            constraints,
        ),
        Some(300.0)
    );
    assert_eq!(state.size(), 300.0);
    assert!(state.is_resizing());

    assert_eq!(
        state.resize(
            DragHandleMessage::Ended {
                position: Point::new(1_000.0, 0.0)
            },
            constraints,
        ),
        Some(420.0)
    );
    assert_eq!(state.size(), 420.0);
    assert!(!state.is_resizing());
}

#[test]
fn panel_resize_constraints_named_edges_preserve_edge_and_normalize_bounds() {
    assert_eq!(
        PanelResizeConstraints::left(100.0, 40.0),
        PanelResizeConstraints {
            edge: PanelResizeEdge::Left,
            min_size: 100.0,
            max_size: 100.0,
        }
    );
    assert_eq!(
        PanelResizeConstraints::right(48.0, 420.0).edge,
        PanelResizeEdge::Right
    );
    assert_eq!(
        PanelResizeConstraints::top(48.0, 420.0).edge,
        PanelResizeEdge::Top
    );
    assert_eq!(
        PanelResizeConstraints::bottom(48.0, 420.0).edge,
        PanelResizeEdge::Bottom
    );
}

#[test]
fn collapsible_panel_resize_constraints_named_edges_preserve_collapse_target() {
    let constraints = CollapsiblePanelResizeConstraints::top(72.0, 240.0, 48.0);

    assert_eq!(constraints.resize.edge, PanelResizeEdge::Top);
    assert_eq!(constraints.resize.min_size, 72.0);
    assert_eq!(constraints.resize.max_size, 240.0);
    assert_eq!(constraints.collapsed_size, 72.0);
    assert_eq!(
        CollapsiblePanelResizeConstraints::right(48.0, 420.0, 96.0)
            .resize
            .edge,
        PanelResizeEdge::Right
    );
    assert_eq!(
        CollapsiblePanelResizeConstraints::left(48.0, 420.0, 96.0)
            .resize
            .edge,
        PanelResizeEdge::Left
    );
    assert_eq!(
        CollapsiblePanelResizeConstraints::bottom(48.0, 420.0, 96.0)
            .resize
            .edge,
        PanelResizeEdge::Bottom
    );
}

#[test]
fn panel_resize_state_toggles_collapsible_size_on_double_activate() {
    let mut state = PanelResizeState::new(180.0);
    let constraints =
        CollapsiblePanelResizeConstraints::new(PanelResizeEdge::Top, 72.0, 240.0, 48.0);

    assert_eq!(
        state.resize_collapsible(
            DragHandleMessage::DoubleActivate {
                position: Point::new(0.0, 120.0)
            },
            constraints,
        ),
        Some(72.0)
    );
    assert_eq!(state.size(), 72.0);
    assert!(!state.is_resizing());

    assert_eq!(
        state.resize_collapsible(
            DragHandleMessage::DoubleActivate {
                position: Point::new(0.0, 120.0)
            },
            constraints,
        ),
        Some(180.0)
    );
    assert_eq!(state.size(), 180.0);
    assert!(!state.is_resizing());
}

#[test]
fn panel_resize_state_restores_last_dragged_collapsible_size() {
    let mut state = PanelResizeState::new(180.0);
    let constraints =
        CollapsiblePanelResizeConstraints::new(PanelResizeEdge::Top, 72.0, 240.0, 48.0);

    state.resize_collapsible(
        DragHandleMessage::started(Point::new(0.0, 120.0)),
        constraints,
    );
    state.resize_collapsible(
        DragHandleMessage::Ended {
            position: Point::new(0.0, 80.0),
        },
        constraints,
    );
    assert_eq!(state.size(), 220.0);

    state.resize_collapsible(
        DragHandleMessage::DoubleActivate {
            position: Point::new(0.0, 120.0),
        },
        constraints,
    );
    assert_eq!(state.size(), 72.0);

    assert_eq!(
        state.resize_collapsible(
            DragHandleMessage::DoubleActivate {
                position: Point::new(0.0, 120.0)
            },
            constraints,
        ),
        Some(220.0)
    );
    assert_eq!(state.size(), 220.0);
}

#[test]
fn panel_resize_state_expands_to_max_when_no_expanded_size_is_known() {
    let mut state = PanelResizeState::new(72.0);
    let constraints =
        CollapsiblePanelResizeConstraints::new(PanelResizeEdge::Top, 72.0, 240.0, 72.0);

    assert_eq!(
        state.resize_collapsible(
            DragHandleMessage::DoubleActivate {
                position: Point::new(0.0, 120.0)
            },
            constraints,
        ),
        Some(240.0)
    );
    assert_eq!(state.size(), 240.0);
}
