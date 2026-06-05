use super::super::{
    CanvasInvalidation, CanvasLayer, CanvasLayerOrder, CanvasLayerParts,
    CanvasSelectionAffordanceHitTestParts, CanvasSelectionAffordancePaintParts,
    CanvasSelectionAffordanceStyle, CanvasSelectionBodyHandleHitTestParts,
    CanvasSelectionBodyHandlePaintParts, CanvasSelectionBodyHandleParts,
    CanvasSelectionBodyHandleStyle, CanvasSelectionEdgeHitTestParts,
    CanvasSelectionEdgeVisualPaintParts, CanvasSelectionEdgeVisualStyle, CanvasSelectionGeometry,
    CanvasSelectionPaintStyle, CanvasSelectionTrailingControlHitTestParts,
    CanvasSelectionTrailingControlPaintParts, CanvasSelectionTrailingControlStyle, DragHandle,
    DragHandleRole, canvas_layer_at_point, canvas_selection_body_handle_rect,
    canvas_selection_edge_handles, canvas_selection_edge_visual_rect, canvas_selection_rect,
    canvas_selection_trailing_control_rect, drag_handle_at_point,
    horizontal_resize_edge_bracket_rects, horizontal_resize_edge_handles,
    horizontal_resize_edge_visual_rect, horizontal_resize_handles,
};
use crate::{
    gui::{
        range::{IndexViewport, IndexViewportScope, NormalizedRange},
        types::{Point, Rect, Rgba8},
    },
    runtime::PaintPrimitive,
};

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
fn horizontal_resize_handles_prioritize_edges_over_body() {
    let rect = Rect::from_min_max(Point::new(20.0, 30.0), Point::new(120.0, 70.0));
    let handles = horizontal_resize_handles(rect, 12.0, 99).expect("handles");

    assert_eq!(handles[0].role, DragHandleRole::Body);
    assert_eq!(handles[0].capture_token, 99);
    assert_eq!(
        handles[1].rect,
        Rect::from_min_max(Point::new(20.0, 30.0), Point::new(32.0, 70.0))
    );
    assert_eq!(
        handles[2].rect,
        Rect::from_min_max(Point::new(108.0, 30.0), Point::new(120.0, 70.0))
    );
    assert_eq!(
        drag_handle_at_point(&handles, Point::new(25.0, 50.0)).map(|handle| handle.role),
        Some(DragHandleRole::Start)
    );
    assert_eq!(
        drag_handle_at_point(&handles, Point::new(60.0, 50.0)).map(|handle| handle.role),
        Some(DragHandleRole::Body)
    );
    assert_eq!(
        drag_handle_at_point(&handles, Point::new(116.0, 50.0)).map(|handle| handle.role),
        Some(DragHandleRole::End)
    );
}

#[test]
fn horizontal_resize_edges_clamp_to_half_width() {
    let rect = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(10.0, 20.0));
    let handles = horizontal_resize_edge_handles(rect, 12.0, 7).expect("handles");

    assert_eq!(
        handles[0].rect,
        Rect::from_min_max(Point::new(0.0, 0.0), Point::new(5.0, 20.0))
    );
    assert_eq!(
        handles[1].rect,
        Rect::from_min_max(Point::new(5.0, 0.0), Point::new(10.0, 20.0))
    );
    assert_eq!(horizontal_resize_edge_handles(rect, 0.0, 7), None);
}

#[test]
fn horizontal_resize_edge_visual_rect_supports_insets() {
    let rect = Rect::from_min_max(Point::new(20.0, 30.0), Point::new(120.0, 70.0));

    assert_eq!(
        horizontal_resize_edge_visual_rect(rect, DragHandleRole::Start, 5.0, 2.0, 4.0),
        Some(Rect::from_min_max(
            Point::new(22.0, 34.0),
            Point::new(27.0, 66.0)
        ))
    );
    assert_eq!(
        horizontal_resize_edge_visual_rect(rect, DragHandleRole::End, 5.0, 2.0, 4.0),
        Some(Rect::from_min_max(
            Point::new(113.0, 34.0),
            Point::new(118.0, 66.0)
        ))
    );
    assert_eq!(
        horizontal_resize_edge_visual_rect(rect, DragHandleRole::Body, 5.0, 2.0, 4.0),
        None
    );
}

#[test]
fn horizontal_resize_edge_bracket_rects_project_edge_and_ticks() {
    let rect = Rect::from_min_max(Point::new(20.0, 30.0), Point::new(120.0, 70.0));

    assert_eq!(
        horizontal_resize_edge_bracket_rects(rect, DragHandleRole::Start, 2.0, 7.0),
        Some([
            Rect::from_min_max(Point::new(20.0, 30.0), Point::new(22.0, 70.0)),
            Rect::from_min_max(Point::new(20.0, 30.0), Point::new(27.0, 32.0)),
            Rect::from_min_max(Point::new(20.0, 68.0), Point::new(27.0, 70.0)),
        ])
    );
    assert_eq!(
        horizontal_resize_edge_bracket_rects(rect, DragHandleRole::End, 2.0, 7.0),
        Some([
            Rect::from_min_max(Point::new(118.0, 30.0), Point::new(120.0, 70.0)),
            Rect::from_min_max(Point::new(113.0, 30.0), Point::new(120.0, 32.0)),
            Rect::from_min_max(Point::new(113.0, 68.0), Point::new(120.0, 70.0)),
        ])
    );
    assert_eq!(
        horizontal_resize_edge_bracket_rects(rect, DragHandleRole::Body, 2.0, 7.0),
        None
    );
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

#[test]
fn canvas_selection_rect_projects_normalized_range() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));

    assert_eq!(
        canvas_selection_rect(bounds, 0.25, 0.75),
        Some(Rect::from_min_max(
            Point::new(60.0, 20.0),
            Point::new(160.0, 120.0)
        ))
    );
    assert_eq!(canvas_selection_rect(bounds, 0.75, 0.25), None);
    assert_eq!(
        canvas_selection_rect(bounds, f32::NAN, 0.5),
        Some(Rect::from_min_max(
            Point::new(10.0, 20.0),
            Point::new(110.0, 120.0)
        ))
    );
}

#[test]
fn canvas_selection_geometry_projects_common_affordances() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let geometry = CanvasSelectionGeometry::new(bounds, 0.2, 0.6).expect("geometry");

    assert_eq!(
        geometry.rect,
        Rect::from_min_max(Point::new(50.0, 20.0), Point::new(130.0, 120.0))
    );
    assert_eq!(
        geometry.body_handle_rect(7.0, 9.0, 0.28, 1.0),
        Some(Rect::from_min_max(
            Point::new(59.0, 20.0),
            Point::new(121.0, 27.0)
        ))
    );
    assert_eq!(
        geometry.trailing_control_rect(16.0, 0.0),
        Some(Rect::from_min_max(
            Point::new(114.0, 104.0),
            Point::new(130.0, 120.0)
        ))
    );
    assert!(geometry.body_handle_at_point(7.0, 9.0, 0.28, 1.0, Point::new(60.0, 24.0)));
    assert!(!geometry.body_handle_at_point(7.0, 9.0, 0.28, 1.0, Point::new(60.0, 40.0)));
    assert!(geometry.trailing_control_at_point(16.0, 0.0, Point::new(120.0, 110.0)));
    assert!(!geometry.trailing_control_at_point(16.0, 0.0, Point::new(120.0, 90.0)));
    assert_eq!(
        geometry.edge_visual_rect(bounds.top_edge_strip(22.0), DragHandleRole::End, 7.0, 0.0),
        Some(Rect::from_min_max(
            Point::new(126.5, 20.0),
            Point::new(133.5, 42.0)
        ))
    );
    assert_eq!(
        geometry.edge_at_point(
            bounds.top_edge_strip(22.0),
            Point::new(130.0, 24.0),
            7.0,
            0.0
        ),
        Some(DragHandleRole::End)
    );
}

#[test]
fn canvas_selection_geometry_projects_viewport_clipped_range() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let viewport = IndexViewportScope::new(
        IndexViewport {
            start: 200,
            end: 600,
        },
        1_000,
        10,
    );
    let geometry = CanvasSelectionGeometry::from_viewport_range(
        bounds,
        viewport,
        NormalizedRange::from_fractions(0.3, 0.7),
    )
    .expect("visible geometry");

    assert!((geometry.start_fraction - 0.25).abs() <= f32::EPSILON);
    assert!((geometry.end_fraction - 1.0).abs() <= f32::EPSILON);
    assert!((geometry.rect.min.x - 60.0).abs() <= 0.001);
    assert_eq!(geometry.rect.min.y, 20.0);
    assert_eq!(geometry.rect.max.x, 210.0);
    assert_eq!(geometry.rect.max.y, 120.0);
    assert_eq!(
        CanvasSelectionGeometry::from_viewport_range(
            bounds,
            viewport,
            NormalizedRange::from_fractions(0.7, 0.8),
        ),
        None
    );
}

#[test]
fn canvas_selection_affordance_hit_test_uses_editor_priority_order() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let geometry = CanvasSelectionGeometry::new(bounds, 0.2, 0.6).expect("geometry");

    let trailing_over_body = CanvasSelectionAffordanceHitTestParts::new()
        .with_body(CanvasSelectionBodyHandleHitTestParts::new(
            Point::new(120.0, 110.0),
            100.0,
            0.0,
            0.0,
            1.0,
        ))
        .with_trailing_control(CanvasSelectionTrailingControlHitTestParts::new(
            Point::new(120.0, 110.0),
            16.0,
            0.0,
        ));

    assert_eq!(
        geometry.affordance_at_point(trailing_over_body),
        Some(DragHandleRole::TrailingControl)
    );

    let edge_over_body = CanvasSelectionAffordanceHitTestParts::new()
        .with_body(CanvasSelectionBodyHandleHitTestParts::new(
            Point::new(130.0, 24.0),
            90.0,
            0.0,
            0.0,
            1.0,
        ))
        .with_edge(CanvasSelectionEdgeHitTestParts::new(
            bounds.top_edge_strip(22.0),
            Point::new(130.0, 24.0),
            7.0,
            0.0,
        ));

    assert_eq!(
        geometry.affordance_at_point(edge_over_body),
        Some(DragHandleRole::End)
    );
    assert_eq!(
        geometry.affordance_at_point(CanvasSelectionAffordanceHitTestParts {
            edge: None,
            ..edge_over_body
        }),
        Some(DragHandleRole::Body)
    );
}

#[test]
fn canvas_selection_affordance_hit_test_can_check_single_groups() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let geometry = CanvasSelectionGeometry::new(bounds, 0.2, 0.6).expect("geometry");

    assert_eq!(
        geometry.affordance_at_point(CanvasSelectionAffordanceHitTestParts::new().with_edge(
            CanvasSelectionEdgeHitTestParts::new(
                bounds.top_edge_strip(22.0),
                Point::new(50.0, 24.0),
                7.0,
                0.0,
            )
        )),
        Some(DragHandleRole::Start)
    );
    assert_eq!(
        geometry.affordance_at_point(CanvasSelectionAffordanceHitTestParts::new().with_body(
            CanvasSelectionBodyHandleHitTestParts::new(Point::new(60.0, 24.0), 7.0, 9.0, 0.28, 1.0,)
        )),
        Some(DragHandleRole::Body)
    );
    assert_eq!(
        geometry.affordance_at_point(
            CanvasSelectionAffordanceHitTestParts::new().with_trailing_control(
                CanvasSelectionTrailingControlHitTestParts::new(
                    Point::new(120.0, 110.0),
                    16.0,
                    0.0,
                )
            )
        ),
        Some(DragHandleRole::TrailingControl)
    );
    assert_eq!(
        geometry.affordance_at_point(CanvasSelectionAffordanceHitTestParts::new().with_body(
            CanvasSelectionBodyHandleHitTestParts::new(Point::new(60.0, 40.0), 7.0, 9.0, 0.28, 1.0,)
        )),
        None
    );
}

#[test]
fn canvas_selection_geometry_pushes_common_affordance_fills() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let geometry = CanvasSelectionGeometry::new(bounds, 0.2, 0.6).expect("geometry");
    let mut primitives = Vec::<PaintPrimitive>::new();
    let color = Rgba8::new(1, 2, 3, 4);

    assert!(geometry.push_body_handle_fill(
        &mut primitives,
        42,
        CanvasSelectionBodyHandlePaintParts::new(7.0, 9.0, 0.28, 1.0, color),
    ));
    assert!(geometry.push_trailing_control_fill(
        &mut primitives,
        42,
        CanvasSelectionTrailingControlPaintParts::new(16.0, 0.0, color),
    ));
    assert!(geometry.push_edge_visual_fill(
        &mut primitives,
        42,
        CanvasSelectionEdgeVisualPaintParts::new(
            bounds.top_edge_strip(22.0),
            DragHandleRole::End,
            7.0,
            0.0,
            color,
        ),
    ));

    let fills = primitives
        .iter()
        .map(|primitive| primitive.fill_rect().expect("fill rect"))
        .collect::<Vec<_>>();
    assert_eq!(fills.len(), 3);
    assert_eq!(fills[0].widget_id, 42);
    assert_eq!(
        fills[0].rect,
        Rect::from_min_max(Point::new(59.0, 20.0), Point::new(121.0, 27.0))
    );
    assert_eq!(
        fills[1].rect,
        Rect::from_min_max(Point::new(114.0, 104.0), Point::new(130.0, 120.0))
    );
    assert_eq!(
        fills[2].rect,
        Rect::from_min_max(Point::new(126.5, 20.0), Point::new(133.5, 42.0))
    );
}

#[test]
fn canvas_selection_affordance_styles_build_matching_hit_test_and_paint_parts() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let point = Point::new(60.0, 24.0);
    let color = Rgba8::new(1, 2, 3, 4);

    let body = CanvasSelectionBodyHandleStyle::new(7.0, 9.0, 0.28, 1.0);
    assert_eq!(
        body.hit_test_parts(point),
        CanvasSelectionBodyHandleHitTestParts::new(point, 7.0, 9.0, 0.28, 1.0)
    );
    assert_eq!(
        body.paint_parts(color),
        CanvasSelectionBodyHandlePaintParts::new(7.0, 9.0, 0.28, 1.0, color)
    );

    let edge = CanvasSelectionEdgeVisualStyle::new(7.0, 0.0);
    assert_eq!(
        edge.hit_test_parts(bounds.top_edge_strip(22.0), point),
        CanvasSelectionEdgeHitTestParts::new(bounds.top_edge_strip(22.0), point, 7.0, 0.0)
    );
    assert_eq!(
        edge.paint_parts(bounds.top_edge_strip(22.0), DragHandleRole::End, color),
        CanvasSelectionEdgeVisualPaintParts::new(
            bounds.top_edge_strip(22.0),
            DragHandleRole::End,
            7.0,
            0.0,
            color,
        )
    );

    let trailing = CanvasSelectionTrailingControlStyle::new(16.0, 0.0);
    assert_eq!(
        trailing.hit_test_parts(point),
        CanvasSelectionTrailingControlHitTestParts::new(point, 16.0, 0.0)
    );
    assert_eq!(
        trailing.paint_parts(color),
        CanvasSelectionTrailingControlPaintParts::new(16.0, 0.0, color)
    );
}

#[test]
fn canvas_selection_affordance_style_builds_group_hit_tests() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let point = Point::new(130.0, 24.0);

    let style = CanvasSelectionAffordanceStyle::new()
        .with_body(CanvasSelectionBodyHandleStyle::new(7.0, 9.0, 0.28, 1.0))
        .with_edge(CanvasSelectionEdgeVisualStyle::new(7.0, 0.0))
        .with_trailing_control(CanvasSelectionTrailingControlStyle::new(16.0, 0.0));

    assert_eq!(
        style.hit_test_parts(bounds.top_edge_strip(22.0), point),
        CanvasSelectionAffordanceHitTestParts::new()
            .with_body(CanvasSelectionBodyHandleHitTestParts::new(
                point, 7.0, 9.0, 0.28, 1.0
            ))
            .with_edge(CanvasSelectionEdgeHitTestParts::new(
                bounds.top_edge_strip(22.0),
                point,
                7.0,
                0.0,
            ))
            .with_trailing_control(CanvasSelectionTrailingControlHitTestParts::new(
                point, 16.0, 0.0,
            ))
    );

    let geometry = CanvasSelectionGeometry::new(bounds, 0.2, 0.6).expect("geometry");
    assert_eq!(
        style.affordance_at_point(geometry, bounds.top_edge_strip(22.0), point),
        Some(DragHandleRole::End)
    );
}

#[test]
fn canvas_selection_affordance_style_pushes_group_fills() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let edge_bounds = bounds.top_edge_strip(22.0);
    let geometry = CanvasSelectionGeometry::new(bounds, 0.2, 0.6).expect("geometry");
    let body_color = Rgba8::new(1, 2, 3, 4);
    let edge_color = Rgba8::new(5, 6, 7, 8);
    let trailing_color = Rgba8::new(9, 10, 11, 12);
    let style = CanvasSelectionAffordanceStyle::new()
        .with_body(CanvasSelectionBodyHandleStyle::new(7.0, 9.0, 0.28, 1.0))
        .with_edge(CanvasSelectionEdgeVisualStyle::new(7.0, 0.0))
        .with_trailing_control(CanvasSelectionTrailingControlStyle::new(16.0, 0.0));
    let mut primitives = Vec::new();

    assert_eq!(
        style.push_fills(
            &mut primitives,
            42,
            geometry,
            CanvasSelectionAffordancePaintParts::new(edge_bounds)
                .body_color(body_color)
                .edge_color(edge_color)
                .trailing_control_color(trailing_color),
        ),
        4
    );

    let fills = primitives
        .iter()
        .map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) => fill,
            primitive => panic!("expected fill rect, got {primitive:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(fills.len(), 4);
    assert_eq!(fills[0].widget_id, 42);
    assert_eq!(fills[0].color, edge_color);
    assert_eq!(fills[0].rect.min.x, 46.5);
    assert_eq!(fills[1].color, edge_color);
    assert_eq!(fills[1].rect.min.x, 126.5);
    assert_eq!(fills[2].color, body_color);
    assert_eq!(
        fills[2].rect,
        Rect::from_min_max(Point::new(59.0, 20.0), Point::new(121.0, 27.0))
    );
    assert_eq!(fills[3].color, trailing_color);
    assert_eq!(
        fills[3].rect,
        Rect::from_min_max(Point::new(114.0, 104.0), Point::new(130.0, 120.0))
    );
}

#[test]
fn canvas_selection_paint_style_derives_standard_colors() {
    let style = CanvasSelectionPaintStyle::new(Rgba8::new(20, 40, 60, 255))
        .fill_alpha(72)
        .cursor_alpha(210)
        .body_alpha(180)
        .edge_alpha(220)
        .trailing_control_alpha(230);

    assert_eq!(style.fill_color(), Rgba8::new(20, 40, 60, 72));
    assert_eq!(style.cursor_color(), Rgba8::new(20, 40, 60, 210));
    assert_eq!(style.body_color(), Rgba8::new(20, 40, 60, 180));
    assert_eq!(style.edge_color(), Rgba8::new(20, 40, 60, 220));
    assert_eq!(style.trailing_control_color(), Rgba8::new(20, 40, 60, 230));
}

#[test]
fn canvas_selection_paint_style_builds_affordance_paint_parts() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let edge_bounds = bounds.top_edge_strip(22.0);
    let parts = CanvasSelectionPaintStyle::new(Rgba8::new(90, 120, 240, 255))
        .body_alpha(180)
        .edge_alpha(220)
        .trailing_control_alpha(235)
        .affordance_paint_parts(edge_bounds);

    assert_eq!(parts.edge_bounds, edge_bounds);
    assert_eq!(parts.body_color, Some(Rgba8::new(90, 120, 240, 180)));
    assert_eq!(parts.edge_color, Some(Rgba8::new(90, 120, 240, 220)));
    assert_eq!(
        parts.trailing_control_color,
        Some(Rgba8::new(90, 120, 240, 235))
    );
}

#[test]
fn canvas_selection_edge_handles_project_hit_targets() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let handles = canvas_selection_edge_handles(bounds, 0.25, 0.75, 18.0, 42).expect("handles");

    assert_eq!(handles[0].role, DragHandleRole::Start);
    assert_eq!(handles[0].capture_token, 42);
    assert_eq!(
        handles[0].rect,
        Rect::from_min_max(Point::new(51.0, 20.0), Point::new(69.0, 120.0))
    );
    assert_eq!(handles[1].role, DragHandleRole::End);
    assert_eq!(
        drag_handle_at_point(&handles, Point::new(160.0, 60.0)).map(|handle| handle.role),
        Some(DragHandleRole::End)
    );
    assert_eq!(
        canvas_selection_edge_handles(bounds, 0.25, 0.75, 0.0, 42),
        None
    );
}

#[test]
fn canvas_selection_edge_visual_rect_projects_inset_handle() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));

    assert_eq!(
        canvas_selection_edge_visual_rect(bounds, 0.25, 8.0, 16.0),
        Some(Rect::from_min_max(
            Point::new(56.0, 36.0),
            Point::new(64.0, 104.0)
        ))
    );
    assert_eq!(
        canvas_selection_edge_visual_rect(bounds, 0.25, 8.0, 60.0),
        None
    );
}

#[test]
fn canvas_selection_body_handle_rect_projects_inset_top_strip() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));

    assert_eq!(
        canvas_selection_body_handle_rect(CanvasSelectionBodyHandleParts {
            bounds,
            start_fraction: 0.2,
            end_fraction: 0.6,
            height: 7.0,
            end_inset: 9.0,
            max_end_inset_fraction: 0.28,
            min_width_after_inset: 1.0,
        }),
        Some(Rect::from_min_max(
            Point::new(59.0, 20.0),
            Point::new(121.0, 27.0)
        ))
    );
}

#[test]
fn canvas_selection_body_handle_rect_keeps_narrow_selection_movable() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));

    assert_eq!(
        canvas_selection_body_handle_rect(CanvasSelectionBodyHandleParts {
            bounds,
            start_fraction: 0.2,
            end_fraction: 0.205,
            height: 200.0,
            end_inset: 9.0,
            max_end_inset_fraction: 0.28,
            min_width_after_inset: 1.0,
        }),
        Some(Rect::from_min_max(
            Point::new(50.0, 20.0),
            Point::new(51.0, 120.0)
        ))
    );
}

#[test]
fn canvas_selection_trailing_control_rect_projects_bottom_square() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));

    assert_eq!(
        canvas_selection_trailing_control_rect(bounds, 0.2, 0.6, 16.0, 0.0),
        Some(Rect::from_min_max(
            Point::new(114.0, 104.0),
            Point::new(130.0, 120.0)
        ))
    );
    assert_eq!(
        canvas_selection_trailing_control_rect(bounds, 0.2, 0.6, 0.0, 0.0),
        None
    );
}
