use crate::{
    layout::{Constraints, SizeModeCross, SizeModeMain, SlotParams, Vector2, VirtualizationAxis},
    runtime::{
        SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper,
        surface::{SurfaceTraversalIndex, SurfaceTraversalStats, WidgetPath},
    },
    widgets::{ButtonWidget, WidgetSizing},
};

#[test]
fn runtime_projection_matches_separate_layout_and_traversal_passes() {
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::virtual_scroll_area(
        1,
        SurfaceNode::column(
            2,
            4.0,
            vec![SurfaceChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(28.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                SurfaceNode::widget(
                    ButtonWidget::new(10, "Action", WidgetSizing::fixed(Vector2::new(96.0, 28.0))),
                    WidgetMessageMapper::none(),
                ),
            )],
        ),
        VirtualizationAxis::Vertical,
        16.0,
    ));

    let projection = surface.runtime_projection();
    let traversal = surface.runtime_traversal_index();

    assert_eq!(projection.layout_root, surface.layout_node());
    assert_eq!(
        projection.traversal.widget_paint_order,
        traversal.widget_paint_order
    );
    assert_eq!(
        projection.traversal.stateful_widget_order,
        traversal.stateful_widget_order
    );
    assert_eq!(projection.traversal.widget_paths, traversal.widget_paths);
    assert_eq!(
        projection.traversal.pointer_hit_order,
        traversal.pointer_hit_order
    );
    assert_eq!(
        projection.traversal.scroll_content_by_container,
        traversal.scroll_content_by_container
    );
}

#[test]
fn runtime_projection_reusing_clears_stale_traversal_without_shrinking_buffers() {
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::virtual_scroll_area(
        1,
        SurfaceNode::column(
            2,
            4.0,
            vec![SurfaceChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(28.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                SurfaceNode::widget(
                    ButtonWidget::new(10, "Action", WidgetSizing::fixed(Vector2::new(96.0, 28.0))),
                    WidgetMessageMapper::none(),
                ),
            )],
        ),
        VirtualizationAxis::Vertical,
        16.0,
    ));
    let mut traversal = SurfaceTraversalIndex::with_stats(SurfaceTraversalStats {
        widgets: 8,
        stateful_widgets: 8,
        scroll_containers: 2,
        clipped_containers: 0,
        styled_hoverable_containers: 0,
        max_depth: 4,
        max_scroll_depth: 2,
    });
    traversal.widget_paint_order.push(999);
    traversal.pointer_hit_order.push(999);
    traversal
        .widget_paths
        .insert(999, WidgetPath::from_slice(&[9]));
    let widget_order_capacity = traversal.widget_paint_order.capacity();
    let widget_path_capacity = traversal.widget_paths.capacity();

    let mut scroll_stack = Vec::new();
    let mut child_path = Vec::new();
    let layout_root = surface.runtime_projection_reusing_with_scratch(
        &mut traversal,
        &mut scroll_stack,
        &mut child_path,
    );

    assert_eq!(layout_root.id(), 1);
    assert_eq!(traversal.widget_paint_order, vec![10]);
    assert_eq!(traversal.stateful_widget_order, vec![10]);
    assert_eq!(traversal.pointer_hit_order, vec![10]);
    assert!(traversal.widget_paths.contains_key(&10));
    assert!(!traversal.widget_paths.contains_key(&999));
    assert!(traversal.widget_paint_order.capacity() >= widget_order_capacity);
    assert!(traversal.widget_paths.capacity() >= widget_path_capacity);
}

#[test]
fn runtime_projection_reusing_preserves_scratch_stack_capacity() {
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::virtual_scroll_area(
        1,
        SurfaceNode::column(
            2,
            4.0,
            vec![SurfaceChild::fill(SurfaceNode::row(
                3,
                0.0,
                vec![SurfaceChild::fill(SurfaceNode::widget(
                    ButtonWidget::new(10, "Action", WidgetSizing::fixed(Vector2::new(96.0, 28.0))),
                    WidgetMessageMapper::none(),
                ))],
            ))],
        ),
        VirtualizationAxis::Vertical,
        16.0,
    ));
    let mut traversal = SurfaceTraversalIndex::with_stats(SurfaceTraversalStats {
        widgets: 1,
        stateful_widgets: 1,
        scroll_containers: 1,
        clipped_containers: 1,
        styled_hoverable_containers: 0,
        max_depth: 4,
        max_scroll_depth: 1,
    });
    let mut scroll_stack = Vec::with_capacity(8);
    let mut child_path = Vec::with_capacity(8);
    let scroll_capacity = scroll_stack.capacity();
    let path_capacity = child_path.capacity();

    let layout_root = surface.runtime_projection_reusing_with_scratch(
        &mut traversal,
        &mut scroll_stack,
        &mut child_path,
    );

    assert_eq!(layout_root.id(), 1);
    assert_eq!(scroll_stack.capacity(), scroll_capacity);
    assert_eq!(child_path.capacity(), path_capacity);
    assert!(scroll_stack.is_empty());
    assert!(child_path.is_empty());
}
