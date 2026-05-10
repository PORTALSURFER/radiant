use super::*;
use crate::layout::ContainerKind;
use std::collections::{BTreeMap, BTreeSet, HashMap};

const INLINE_WIDGET_PATH_LEN: usize = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::runtime) struct WidgetPath {
    inline: [usize; INLINE_WIDGET_PATH_LEN],
    len: u8,
    overflow: Option<Box<[usize]>>,
}

impl WidgetPath {
    pub(in crate::runtime) fn from_slice(path: &[usize]) -> Self {
        if path.len() <= INLINE_WIDGET_PATH_LEN {
            let mut inline = [0; INLINE_WIDGET_PATH_LEN];
            inline[..path.len()].copy_from_slice(path);
            return Self {
                inline,
                len: path.len() as u8,
                overflow: None,
            };
        }
        Self {
            inline: [0; INLINE_WIDGET_PATH_LEN],
            len: 0,
            overflow: Some(path.into()),
        }
    }

    pub(in crate::runtime) fn as_slice(&self) -> &[usize] {
        self.overflow
            .as_deref()
            .unwrap_or(&self.inline[..self.len as usize])
    }

    #[cfg(test)]
    fn is_inline(&self) -> bool {
        self.overflow.is_none()
    }
}

pub(in crate::runtime) struct SurfaceTraversalIndex {
    pub(in crate::runtime) widget_paint_order: Vec<WidgetId>,
    pub(in crate::runtime) focusable_widget_order: Vec<WidgetId>,
    pub(in crate::runtime) keyboard_focus_order: Vec<WidgetId>,
    pub(in crate::runtime) pointer_hit_order: Vec<WidgetId>,
    pub(in crate::runtime) wheel_hit_order: Vec<WidgetId>,
    pub(in crate::runtime) widget_paths: HashMap<WidgetId, WidgetPath>,
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
                    .or_insert_with(|| WidgetPath::from_slice(child_path));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_path_uses_inline_storage_for_common_shallow_paths() {
        let shallow = WidgetPath::from_slice(&[1, 2, 3, 4]);
        assert!(shallow.is_inline());
        assert_eq!(shallow.as_slice(), &[1, 2, 3, 4]);

        let deep = WidgetPath::from_slice(&[1, 2, 3, 4, 5]);
        assert!(!deep.is_inline());
        assert_eq!(deep.as_slice(), &[1, 2, 3, 4, 5]);
    }
}
