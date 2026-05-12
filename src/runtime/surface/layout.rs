use super::*;
use crate::layout::{ContainerKind, LayoutNode, SlotChild, Vector2};

pub(in crate::runtime) struct SurfaceRuntimeProjection {
    pub(in crate::runtime) layout_root: LayoutNode,
    pub(in crate::runtime) traversal: SurfaceTraversalIndex,
}

impl<Message> UiSurface<Message> {
    pub(in crate::runtime) fn runtime_projection(&self) -> SurfaceRuntimeProjection {
        let stats = self.root.runtime_traversal_stats();
        let mut traversal = SurfaceTraversalIndex::with_stats(stats);
        let layout_root = self.runtime_projection_into(&mut traversal, stats);
        SurfaceRuntimeProjection {
            layout_root,
            traversal,
        }
    }

    pub(in crate::runtime) fn runtime_projection_into(
        &self,
        traversal: &mut SurfaceTraversalIndex,
        stats: SurfaceTraversalStats,
    ) -> LayoutNode {
        traversal.clear_for_stats(stats);
        self.root.project_runtime(
            &mut Vec::with_capacity(stats.max_scroll_depth),
            &mut Vec::with_capacity(stats.max_depth),
            traversal,
        )
    }

    pub(in crate::runtime) fn runtime_projection_reusing_with_scratch(
        &self,
        traversal: &mut SurfaceTraversalIndex,
        scroll_stack: &mut Vec<NodeId>,
        child_path: &mut Vec<usize>,
    ) -> LayoutNode {
        traversal.clear_for_reuse();
        scroll_stack.clear();
        child_path.clear();
        self.root
            .project_runtime(scroll_stack, child_path, traversal)
    }
}

impl<Message> SurfaceNode<Message> {
    pub(super) fn layout_node(&self) -> LayoutNode {
        match self {
            Self::Container(container) => {
                let mut children = Vec::with_capacity(container.children.len());
                for child in &container.children {
                    children.push(SlotChild::new(child.slot, child.child.layout_node()));
                }
                LayoutNode::container(container.id, container.policy.clone(), children)
            }
            Self::Widget(widget) => widget.layout_node(),
            Self::Overlay(overlay) => LayoutNode::widget(overlay.id, Vector2::new(0.0, 0.0)),
        }
    }

    fn project_runtime(
        &self,
        scroll_stack: &mut Vec<NodeId>,
        child_path: &mut Vec<usize>,
        traversal: &mut SurfaceTraversalIndex,
    ) -> LayoutNode {
        self.project_runtime_inner(scroll_stack, child_path, traversal, true)
            .expect("layout-emitting runtime projection should return a layout node")
    }

    pub(in crate::runtime) fn project_runtime_index(
        &self,
        scroll_stack: &mut Vec<NodeId>,
        child_path: &mut Vec<usize>,
        traversal: &mut SurfaceTraversalIndex,
    ) {
        let _ = self.project_runtime_inner(scroll_stack, child_path, traversal, false);
    }

    fn project_runtime_inner(
        &self,
        scroll_stack: &mut Vec<NodeId>,
        child_path: &mut Vec<usize>,
        traversal: &mut SurfaceTraversalIndex,
        emit_layout: bool,
    ) -> Option<LayoutNode> {
        match self {
            Self::Container(container) => {
                let is_scroll = container.policy.kind == ContainerKind::ScrollView;
                if !scroll_stack.is_empty() {
                    traversal
                        .container_clip_ancestors
                        .insert(container.id, ClipAncestors::from_slice(scroll_stack));
                }
                if is_scroll {
                    scroll_stack.push(container.id);
                    traversal.scroll_container_order.push(container.id);
                    if let Some(content) = container.children.first() {
                        traversal
                            .scroll_content_by_container
                            .insert(container.id, content.child.id());
                    }
                }
                if container.style.is_some() && container.hoverable {
                    traversal.styled_container_order.push(container.id);
                }
                let mut children =
                    emit_layout.then(|| Vec::with_capacity(container.children.len()));
                for (child_index, child) in container.children.iter().enumerate() {
                    child_path.push(child_index);
                    let child_layout = child.child.project_runtime_inner(
                        scroll_stack,
                        child_path,
                        traversal,
                        emit_layout,
                    );
                    child_path.pop();
                    if let (Some(children), Some(child_layout)) = (&mut children, child_layout) {
                        children.push(SlotChild::new(child.slot, child_layout));
                    }
                }
                if is_scroll {
                    scroll_stack.pop();
                }
                children.map(|children| {
                    LayoutNode::container(container.id, container.policy.clone(), children)
                })
            }
            Self::Widget(widget) => {
                traversal.widget_paint_order.push(widget.id());
                traversal
                    .widget_paths
                    .entry(widget.id())
                    .or_insert_with(|| WidgetPath::from_slice(child_path));
                if widget.is_focusable() {
                    traversal.focusable_widget_order.push(widget.id());
                }
                if widget.is_keyboard_focusable() {
                    traversal.keyboard_focus_order.push(widget.id());
                }
                if widget.receives_pointer_hit_testing() {
                    traversal.pointer_hit_order.push(widget.id());
                }
                if widget.receives_wheel_input() {
                    traversal.wheel_hit_order.push(widget.id());
                }
                if widget.needs_state_synchronization() {
                    traversal.stateful_widget_order.push(widget.id());
                }
                if widget.suppresses_container_hover() {
                    traversal.container_hover_suppression.insert(widget.id());
                }
                if !scroll_stack.is_empty() {
                    traversal
                        .widget_clip_ancestors
                        .insert(widget.id(), ClipAncestors::from_slice(scroll_stack));
                }
                emit_layout.then(|| widget.layout_node())
            }
            Self::Overlay(overlay) => {
                emit_layout.then(|| LayoutNode::widget(overlay.id, Vector2::new(0.0, 0.0)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        layout::{SizeModeCross, SizeModeMain, SlotParams, Vector2, VirtualizationAxis},
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
                        constraints: crate::layout::Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    SurfaceNode::widget(
                        ButtonWidget::new(
                            10,
                            "Action",
                            WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
                        ),
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
                        constraints: crate::layout::Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    SurfaceNode::widget(
                        ButtonWidget::new(
                            10,
                            "Action",
                            WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
                        ),
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
                        ButtonWidget::new(
                            10,
                            "Action",
                            WidgetSizing::fixed(Vector2::new(96.0, 28.0)),
                        ),
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
}
