use super::*;
use crate::{
    layout::ContainerKind,
    runtime::{ClipAncestors, SurfaceChild, SurfaceNode, WidgetMessageMapper, WidgetPath},
    widgets::{ButtonWidget, TextWidget, WidgetSizing},
};

#[test]
fn widget_path_uses_inline_storage_for_common_shallow_paths() {
    let shallow = WidgetPath::from_slice(&[1, 2, 3, 4]);
    assert!(shallow.is_inline());
    assert_eq!(shallow.as_slice(), &[1, 2, 3, 4]);

    let deep = WidgetPath::from_slice(&[1, 2, 3, 4, 5]);
    assert!(!deep.is_inline());
    assert_eq!(deep.as_slice(), &[1, 2, 3, 4, 5]);
}

#[test]
fn clip_ancestors_use_inline_storage_for_common_scroll_depths() {
    let shallow = ClipAncestors::from_slice(&[10, 20]);
    assert!(shallow.is_inline());
    assert_eq!(shallow.as_slice(), &[10, 20]);

    let deep = ClipAncestors::from_slice(&[10, 20, 30]);
    assert!(!deep.is_inline());
    assert_eq!(deep.as_slice(), &[10, 20, 30]);
}

#[test]
fn traversal_stats_presize_clipped_container_ancestors() {
    let surface = UiSurface::new(SurfaceNode::container(
        1,
        crate::layout::ContainerPolicy {
            kind: ContainerKind::ScrollView,
            ..Default::default()
        },
        vec![SurfaceChild::new(
            crate::layout::SlotParams::fill(),
            SurfaceNode::container(
                2,
                crate::layout::ContainerPolicy::default(),
                vec![SurfaceChild::new(
                    crate::layout::SlotParams::fill(),
                    SurfaceNode::container(
                        3,
                        crate::layout::ContainerPolicy::default(),
                        Vec::<SurfaceChild<()>>::new(),
                    ),
                )],
            ),
        )],
    ));

    let stats = surface.root.runtime_traversal_stats();
    let mut index = SurfaceTraversalIndex::with_stats(stats);

    assert_eq!(stats.clipped_containers, 2);
    assert!(index.container_clip_ancestors.capacity() >= 2);

    index.clear_for_stats(stats);

    assert!(index.container_clip_ancestors.capacity() >= 2);
}

#[test]
fn traversal_tracks_only_widgets_that_need_state_synchronization() {
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::column(
        1,
        0.0,
        vec![
            SurfaceChild::fill(SurfaceNode::static_widget(TextWidget::new(
                10,
                "Stateless label",
                WidgetSizing::fixed(crate::layout::Vector2::new(120.0, 20.0)),
            ))),
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(
                    20,
                    "Stateful button",
                    WidgetSizing::fixed(crate::layout::Vector2::new(120.0, 28.0)),
                ),
                WidgetMessageMapper::none(),
            )),
        ],
    ));

    let stats = surface.root.runtime_traversal_stats();
    let index = surface.runtime_traversal_index();

    assert_eq!(stats.widgets, 2);
    assert_eq!(stats.stateful_widgets, 1);
    assert_eq!(index.widget_paint_order, vec![10, 20]);
    assert_eq!(index.stateful_widget_order, vec![20]);
}

#[test]
fn traversal_index_clear_for_stats_grows_reused_storage_to_requested_capacity() {
    let mut index = SurfaceTraversalIndex::with_stats(SurfaceTraversalStats {
        widgets: 4,
        stateful_widgets: 4,
        styled_hoverable_containers: 1,
        scroll_containers: 1,
        clipped_containers: 1,
        max_depth: 1,
        max_scroll_depth: 1,
    });

    index.clear_for_stats(SurfaceTraversalStats {
        widgets: 96,
        stateful_widgets: 24,
        styled_hoverable_containers: 12,
        scroll_containers: 8,
        clipped_containers: 16,
        max_depth: 4,
        max_scroll_depth: 2,
    });

    assert!(index.widget_paint_order.capacity() >= 96);
    assert!(index.focusable_widget_order.capacity() >= 96);
    assert!(index.keyboard_focus_order.capacity() >= 96);
    assert!(index.pointer_hit_order.capacity() >= 96);
    assert!(index.wheel_hit_order.capacity() >= 96);
    assert!(index.wheel_target_order.capacity() >= 104);
    assert!(index.stateful_widget_order.capacity() >= 24);
    assert!(index.widget_paths.capacity() >= 96);
    assert!(index.container_hover_suppression.capacity() >= 96);
    assert!(index.styled_container_order.capacity() >= 12);
    assert!(index.scroll_container_order.capacity() >= 8);
    assert!(index.widget_clip_ancestors.capacity() >= 96);
    assert!(index.container_clip_ancestors.capacity() >= 16);
    assert!(index.scroll_content_by_container.capacity() >= 8);
}
