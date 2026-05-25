use crate::gui::panel::{
    AnchoredPanelRectParts, FloatingPanelDrag, FloatingPanelDragParts, FloatingPanelRectParts,
    anchored_panel_rect, anchored_panel_rect_from_parts, floating_panel_rect,
    floating_panel_rect_from_parts,
};
use crate::gui::types::{Point, Rect, Vector2};

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
