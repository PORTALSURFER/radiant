use super::*;
use crate::{
    layout::{ContainerKind, ContainerPolicy, NodeId, SplitPanePolicy},
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
    let mut index = SurfaceTraversalIndex::<()>::with_stats(stats);

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

fn runtime_owned_split(
    id: NodeId,
    first: SurfaceNode<()>,
    second: SurfaceNode<()>,
) -> SurfaceNode<()> {
    let policy = SplitPanePolicy {
        axis: crate::layout::SplitPaneAxis::Horizontal,
        initial_ratio: 0.5,
        divider_extent: 8.0,
        first_min_extent: 0.0,
        second_min_extent: 0.0,
    };
    SurfaceNode::container(
        id,
        ContainerPolicy {
            kind: ContainerKind::SplitPane,
            split_pane: policy,
            ..ContainerPolicy::default()
        },
        vec![SurfaceChild::fill(first), SurfaceChild::fill(second)],
    )
    .with_split_pane_runtime_mode(Some(
        crate::gui::layout_core::SplitPaneRuntimeMode::RuntimeOwned {
            collapse_policy: None,
        },
    ))
    .with_layout_capabilities(
        crate::gui::layout_core::runtime_owned_split_pane_capabilities(policy, None),
    )
}

#[test]
fn runtime_split_focus_candidates_follow_flat_and_nested_focusable_boundaries() {
    let flat = runtime_owned_split(
        1,
        SurfaceNode::column(
            2,
            0.0,
            vec![
                SurfaceChild::fill(SurfaceNode::widget(
                    ButtonWidget::new(
                        10,
                        "first",
                        WidgetSizing::fixed(crate::layout::Vector2::new(80.0, 28.0)),
                    ),
                    WidgetMessageMapper::none(),
                )),
                SurfaceChild::fill(SurfaceNode::static_widget(TextWidget::new(
                    11,
                    "non-focusable",
                    WidgetSizing::fixed(crate::layout::Vector2::new(80.0, 20.0)),
                ))),
            ],
        ),
        SurfaceNode::widget(
            ButtonWidget::new(
                12,
                "second",
                WidgetSizing::fixed(crate::layout::Vector2::new(80.0, 28.0)),
            ),
            WidgetMessageMapper::none(),
        ),
    );
    let flat_projection = UiSurface::new(flat).runtime_projection();
    assert_eq!(flat_projection.traversal.keyboard_focus_order, vec![10, 12]);
    assert_eq!(
        flat_projection
            .traversal
            .keyboard_focus_order_candidates
            .iter()
            .map(|candidate| (candidate.target.container_id, candidate.widget_index))
            .collect::<Vec<_>>(),
        vec![(1, 1)]
    );

    let nested = runtime_owned_split(
        1,
        runtime_owned_split(
            4,
            SurfaceNode::widget(
                ButtonWidget::new(
                    5,
                    "inner first",
                    WidgetSizing::fixed(crate::layout::Vector2::new(80.0, 28.0)),
                ),
                WidgetMessageMapper::none(),
            ),
            SurfaceNode::static_widget(TextWidget::new(
                6,
                "inner non-focusable",
                WidgetSizing::fixed(crate::layout::Vector2::new(80.0, 20.0)),
            )),
        ),
        SurfaceNode::widget(
            ButtonWidget::new(
                7,
                "outer second",
                WidgetSizing::fixed(crate::layout::Vector2::new(80.0, 28.0)),
            ),
            WidgetMessageMapper::none(),
        ),
    );
    let nested_projection = UiSurface::new(nested).runtime_projection();
    assert_eq!(nested_projection.traversal.keyboard_focus_order, vec![5, 7]);
    assert_eq!(
        nested_projection
            .traversal
            .keyboard_focus_order_candidates
            .iter()
            .map(|candidate| (candidate.target.container_id, candidate.widget_index))
            .collect::<Vec<_>>(),
        vec![(4, 1), (1, 1)]
    );
}

#[test]
fn traversal_index_clear_for_stats_grows_reused_storage_to_requested_capacity() {
    let mut index = SurfaceTraversalIndex::<()>::with_stats(SurfaceTraversalStats {
        source_nodes: 0,
        widgets: 4,
        stateful_widgets: 4,
        styled_hoverable_containers: 1,
        scroll_containers: 1,
        clipped_containers: 1,
        split_pane_focus_order_candidates: 4,
        max_depth: 1,
        max_scroll_depth: 1,
    });

    index.clear_for_stats(SurfaceTraversalStats {
        source_nodes: 0,
        widgets: 96,
        stateful_widgets: 24,
        styled_hoverable_containers: 12,
        scroll_containers: 8,
        clipped_containers: 16,
        split_pane_focus_order_candidates: 32,
        max_depth: 4,
        max_scroll_depth: 2,
    });

    assert!(index.widget_paint_order.capacity() >= 96);
    assert!(index.focusable_widget_order.capacity() >= 96);
    assert!(index.keyboard_focus_order.capacity() >= 96);
    assert!(index.keyboard_focus_order_candidates.capacity() >= 32);
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
