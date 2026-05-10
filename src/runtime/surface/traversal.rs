use super::*;
use crate::layout::ContainerKind;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub(in crate::runtime) struct SurfaceTraversalIndex {
    pub(in crate::runtime) widget_paint_order: Vec<WidgetId>,
    pub(in crate::runtime) focusable_widget_order: Vec<WidgetId>,
    pub(in crate::runtime) keyboard_focus_order: Vec<WidgetId>,
    pub(in crate::runtime) pointer_hit_order: Vec<WidgetId>,
    pub(in crate::runtime) wheel_hit_order: Vec<WidgetId>,
    pub(in crate::runtime) widget_paths: HashMap<WidgetId, Vec<usize>>,
    pub(in crate::runtime) container_hover_suppression: BTreeSet<WidgetId>,
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
        child_path: &mut Vec<usize>,
        index: &mut SurfaceTraversalIndex,
    ) {
        match self {
            Self::Container(container) => {
                let is_scroll = container.policy.kind == ContainerKind::ScrollView;
                if !scroll_stack.is_empty() {
                    index
                        .container_clip_ancestors
                        .insert(container.id, scroll_stack.clone());
                }
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
                }
                for (child_index, child) in container.children.iter().enumerate() {
                    child_path.push(child_index);
                    child
                        .child
                        .collect_runtime_index(scroll_stack, child_path, index);
                    child_path.pop();
                }
                if is_scroll {
                    scroll_stack.pop();
                }
            }
            Self::Widget(widget) => {
                index.widget_paint_order.push(widget.id());
                index
                    .widget_paths
                    .entry(widget.id())
                    .or_insert_with(|| child_path.clone());
                if widget.is_focusable() {
                    index.focusable_widget_order.push(widget.id());
                }
                if widget.is_keyboard_focusable() {
                    index.keyboard_focus_order.push(widget.id());
                }
                if widget.receives_pointer_hit_testing() {
                    index.pointer_hit_order.push(widget.id());
                }
                if widget.receives_wheel_input() {
                    index.wheel_hit_order.push(widget.id());
                }
                if widget.suppresses_container_hover() {
                    index.container_hover_suppression.insert(widget.id());
                }
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
        self.root
            .collect_runtime_index(&mut Vec::new(), &mut Vec::new(), &mut index);
        index
    }
}
