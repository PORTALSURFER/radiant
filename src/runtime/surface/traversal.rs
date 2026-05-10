use super::*;
use crate::layout::ContainerKind;
use std::collections::BTreeMap;

pub(in crate::runtime) struct SurfaceTraversalIndex {
    pub(in crate::runtime) widget_paint_order: Vec<WidgetId>,
    pub(in crate::runtime) styled_container_order: Vec<NodeId>,
    pub(in crate::runtime) scroll_container_order: Vec<NodeId>,
    pub(in crate::runtime) widget_clip_ancestors: BTreeMap<WidgetId, Vec<NodeId>>,
    pub(in crate::runtime) container_clip_ancestors: BTreeMap<NodeId, Vec<NodeId>>,
    pub(in crate::runtime) scroll_content_by_container: BTreeMap<NodeId, NodeId>,
}

impl<Message> SurfaceNode<Message> {
    fn collect_runtime_index(
        &self,
        scroll_stack: &mut Vec<NodeId>,
        index: &mut SurfaceTraversalIndex,
    ) {
        match self {
            Self::Container(container) => {
                let is_scroll = container.policy.kind == ContainerKind::ScrollView;
                if is_scroll {
                    scroll_stack.push(container.id);
                    index.scroll_container_order.push(container.id);
                    if let Some(content) = container.children.first() {
                        index
                            .scroll_content_by_container
                            .insert(container.id, content.child.id());
                    }
                }
                if container.style.is_some() && container.hoverable {
                    index.styled_container_order.push(container.id);
                    if !scroll_stack.is_empty() {
                        index
                            .container_clip_ancestors
                            .insert(container.id, scroll_stack.clone());
                    }
                }
                for child in &container.children {
                    child.child.collect_runtime_index(scroll_stack, index);
                }
                if is_scroll {
                    scroll_stack.pop();
                }
            }
            Self::Widget(widget) => {
                index.widget_paint_order.push(widget.id());
                if !scroll_stack.is_empty() {
                    index
                        .widget_clip_ancestors
                        .insert(widget.id(), scroll_stack.clone());
                }
            }
            Self::Overlay(_) => {}
        }
    }
}

impl<Message> UiSurface<Message> {
    pub(in crate::runtime) fn runtime_traversal_index(&self) -> SurfaceTraversalIndex {
        let mut index = SurfaceTraversalIndex {
            widget_paint_order: Vec::new(),
            styled_container_order: Vec::new(),
            scroll_container_order: Vec::new(),
            widget_clip_ancestors: BTreeMap::new(),
            container_clip_ancestors: BTreeMap::new(),
            scroll_content_by_container: BTreeMap::new(),
        };
        self.root.collect_runtime_index(&mut Vec::new(), &mut index);
        index
    }
}
