use super::*;
use crate::layout::{ContainerKind, LayoutNode, SlotChild, Vector2};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub(in crate::runtime) struct SurfaceRuntimeProjection {
    pub(in crate::runtime) layout_root: LayoutNode,
    pub(in crate::runtime) traversal: SurfaceTraversalIndex,
}

impl<Message> UiSurface<Message> {
    pub(in crate::runtime) fn runtime_projection(&self) -> SurfaceRuntimeProjection {
        let mut traversal = SurfaceTraversalIndex {
            widget_paint_order: Vec::new(),
            focusable_widget_order: Vec::new(),
            keyboard_focus_order: Vec::new(),
            pointer_hit_order: Vec::new(),
            wheel_hit_order: Vec::new(),
            widget_paths: HashMap::new(),
            container_hover_suppression: BTreeSet::new(),
            styled_container_order: Vec::new(),
            scroll_container_order: Vec::new(),
            widget_clip_ancestors: BTreeMap::new(),
            container_clip_ancestors: BTreeMap::new(),
            scroll_content_by_container: BTreeMap::new(),
        };
        let layout_root =
            self.root
                .project_runtime(&mut Vec::new(), &mut Vec::new(), &mut traversal);
        SurfaceRuntimeProjection {
            layout_root,
            traversal,
        }
    }
}

impl<Message> SurfaceNode<Message> {
    pub(super) fn layout_node(&self) -> LayoutNode {
        match self {
            Self::Container(container) => LayoutNode::container(
                container.id,
                container.policy.clone(),
                container
                    .children
                    .iter()
                    .map(|child| SlotChild::new(child.slot, child.child.layout_node()))
                    .collect(),
            ),
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
                let is_scroll = container.policy.kind == ContainerKind::ScrollView;
                if !scroll_stack.is_empty() {
                    traversal
                        .container_clip_ancestors
                        .insert(container.id, scroll_stack.clone());
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
                let children = container
                    .children
                    .iter()
                    .enumerate()
                    .map(|(child_index, child)| {
                        child_path.push(child_index);
                        let child_layout =
                            child
                                .child
                                .project_runtime(scroll_stack, child_path, traversal);
                        child_path.pop();
                        SlotChild::new(child.slot, child_layout)
                    })
                    .collect();
                if is_scroll {
                    scroll_stack.pop();
                }
                LayoutNode::container(container.id, container.policy.clone(), children)
            }
            Self::Widget(widget) => {
                traversal.widget_paint_order.push(widget.id());
                traversal
                    .widget_paths
                    .entry(widget.id())
                    .or_insert_with(|| child_path.clone());
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
                if widget.suppresses_container_hover() {
                    traversal.container_hover_suppression.insert(widget.id());
                }
                if !scroll_stack.is_empty() {
                    traversal
                        .widget_clip_ancestors
                        .insert(widget.id(), scroll_stack.clone());
                }
                widget.layout_node()
            }
            Self::Overlay(overlay) => LayoutNode::widget(overlay.id, Vector2::new(0.0, 0.0)),
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
}
