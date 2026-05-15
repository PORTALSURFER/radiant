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
        match self {
            Self::Container(container) => {
                let is_scroll = begin_container_runtime(container, scroll_stack, traversal);
                let mut children = Vec::with_capacity(container.children.len());
                for (child_index, child) in container.children.iter().enumerate() {
                    child_path.push(child_index);
                    children.push(SlotChild::new(
                        child.slot,
                        child
                            .child
                            .project_runtime(scroll_stack, child_path, traversal),
                    ));
                    child_path.pop();
                }
                end_container_runtime(is_scroll, scroll_stack);
                LayoutNode::container(container.id, container.policy.clone(), children)
            }
            Self::Widget(widget) => {
                record_widget_runtime(widget, scroll_stack, child_path, traversal);
                widget.layout_node()
            }
            Self::Overlay(overlay) => LayoutNode::widget(overlay.id, Vector2::new(0.0, 0.0)),
        }
    }

    #[cfg(test)]
    pub(in crate::runtime) fn project_runtime_index(
        &self,
        scroll_stack: &mut Vec<NodeId>,
        child_path: &mut Vec<usize>,
        traversal: &mut SurfaceTraversalIndex,
    ) {
        self.collect_runtime_index(scroll_stack, child_path, traversal);
    }

    #[cfg(test)]
    fn collect_runtime_index(
        &self,
        scroll_stack: &mut Vec<NodeId>,
        child_path: &mut Vec<usize>,
        traversal: &mut SurfaceTraversalIndex,
    ) {
        match self {
            Self::Container(container) => {
                let is_scroll = begin_container_runtime(container, scroll_stack, traversal);
                for (child_index, child) in container.children.iter().enumerate() {
                    child_path.push(child_index);
                    child
                        .child
                        .collect_runtime_index(scroll_stack, child_path, traversal);
                    child_path.pop();
                }
                end_container_runtime(is_scroll, scroll_stack);
            }
            Self::Widget(widget) => {
                record_widget_runtime(widget, scroll_stack, child_path, traversal);
            }
            Self::Overlay(_) => {}
        }
    }
}

fn begin_container_runtime<Message>(
    container: &SurfaceContainer<Message>,
    scroll_stack: &mut Vec<NodeId>,
    traversal: &mut SurfaceTraversalIndex,
) -> bool {
    let is_scroll = container.policy.kind == ContainerKind::ScrollView;
    traversal.record_container(SurfaceContainerTraversalRecord {
        id: container.id,
        clipped_by: scroll_stack,
        scroll_content: if is_scroll {
            container.children.first().map(|content| content.child.id())
        } else {
            None
        },
        styled_hoverable: container.style.is_some() && container.hoverable,
    });
    if is_scroll {
        scroll_stack.push(container.id);
    }
    is_scroll
}

fn end_container_runtime(is_scroll: bool, scroll_stack: &mut Vec<NodeId>) {
    if is_scroll {
        scroll_stack.pop();
    }
}

fn record_widget_runtime<Message>(
    widget: &SurfaceWidget<Message>,
    scroll_stack: &[NodeId],
    child_path: &[usize],
    traversal: &mut SurfaceTraversalIndex,
) {
    traversal.record_widget(SurfaceWidgetTraversalRecord {
        id: widget.id(),
        child_path,
        clipped_by: scroll_stack,
        focusable: widget.is_focusable(),
        keyboard_focusable: widget.is_keyboard_focusable(),
        receives_pointer_hit_testing: widget.receives_pointer_hit_testing(),
        receives_wheel_input: widget.receives_wheel_input(),
        needs_state_synchronization: widget.needs_state_synchronization(),
        suppresses_container_hover: widget.suppresses_container_hover(),
    });
}

#[cfg(test)]
#[path = "layout/tests.rs"]
mod tests;
