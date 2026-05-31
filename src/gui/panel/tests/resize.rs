use crate::gui::{panel::*, types::Point};

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
