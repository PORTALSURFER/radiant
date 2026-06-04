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
            DragHandleMessage::Started {
                position: Point::new(100.0, 0.0)
            },
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
            DragHandleMessage::Started {
                position: Point::new(0.0, 120.0)
            },
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
