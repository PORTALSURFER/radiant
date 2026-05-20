use super::super::{
    CanvasInvalidation, CanvasLayer, CanvasLayerOrder, CanvasLayerParts, DragHandle,
    DragHandleRole, canvas_layer_at_point, drag_handle_at_point,
};
use crate::gui::types::{Point, Rect};

#[test]
fn canvas_layer_hit_testing_prefers_topmost_interactive_layer() {
    let bounds = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
    let layers = [
        CanvasLayer::from_parts(CanvasLayerParts {
            id: String::from("base"),
            order: CanvasLayerOrder::Background,
            bounds,
            interactive: true,
        }),
        CanvasLayer::from_parts(CanvasLayerParts {
            id: String::from("paint"),
            order: CanvasLayerOrder::Content,
            bounds,
            interactive: false,
        }),
        CanvasLayer::from_parts(CanvasLayerParts {
            id: String::from("handle"),
            order: CanvasLayerOrder::Interaction,
            bounds: Rect::from_min_max(Point::new(40.0, 40.0), Point::new(60.0, 60.0)),
            interactive: true,
        }),
        CanvasLayer::from_parts(CanvasLayerParts {
            id: String::from("focus"),
            order: CanvasLayerOrder::Focus,
            bounds: Rect::from_min_max(Point::new(45.0, 45.0), Point::new(55.0, 55.0)),
            interactive: true,
        }),
    ];

    assert_eq!(
        canvas_layer_at_point(&layers, Point::new(50.0, 50.0)),
        Some("focus")
    );
    assert_eq!(
        canvas_layer_at_point(&layers, Point::new(20.0, 20.0)),
        Some("base")
    );
    assert_eq!(
        canvas_layer_at_point(&layers, Point::new(120.0, 20.0)),
        None
    );
}

#[test]
fn drag_handle_hit_testing_uses_reverse_paint_order_and_enabled_state() {
    let handles = [
        DragHandle::new(
            DragHandleRole::Body,
            Rect::from_min_max(Point::new(10.0, 10.0), Point::new(50.0, 30.0)),
            1,
        ),
        DragHandle::new(
            DragHandleRole::Start,
            Rect::from_min_max(Point::new(10.0, 10.0), Point::new(20.0, 30.0)),
            2,
        )
        .with_enabled(false),
        DragHandle::new(
            DragHandleRole::End,
            Rect::from_min_max(Point::new(40.0, 10.0), Point::new(50.0, 30.0)),
            3,
        ),
    ];

    assert_eq!(
        drag_handle_at_point(&handles, Point::new(45.0, 20.0)).map(|handle| handle.role),
        Some(DragHandleRole::End)
    );
    assert_eq!(
        drag_handle_at_point(&handles, Point::new(15.0, 20.0)).map(|handle| handle.role),
        Some(DragHandleRole::Body)
    );
    assert_eq!(drag_handle_at_point(&handles, Point::new(5.0, 20.0)), None);
}

#[test]
fn canvas_invalidation_splits_scene_and_interaction_rebuilds() {
    let interaction = CanvasInvalidation {
        interaction_changed: true,
        ..CanvasInvalidation::default()
    };
    let projection = CanvasInvalidation {
        projection_changed: true,
        ..CanvasInvalidation::default()
    };

    assert!(!interaction.requires_scene_rebuild());
    assert!(interaction.requires_interaction_overlay_rebuild());
    assert!(projection.requires_scene_rebuild());
    assert!(projection.requires_interaction_overlay_rebuild());
}
