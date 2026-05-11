use super::*;
use crate::layout::ContainerKind;
use std::collections::{HashMap, HashSet};

const INLINE_WIDGET_PATH_LEN: usize = 4;
const INLINE_CLIP_ANCESTOR_LEN: usize = 2;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::runtime) struct ClipAncestors {
    inline: [NodeId; INLINE_CLIP_ANCESTOR_LEN],
    len: u8,
    overflow: Option<Box<[NodeId]>>,
}

impl ClipAncestors {
    pub(in crate::runtime) fn from_slice(ancestors: &[NodeId]) -> Self {
        if ancestors.len() <= INLINE_CLIP_ANCESTOR_LEN {
            let mut inline = [0; INLINE_CLIP_ANCESTOR_LEN];
            inline[..ancestors.len()].copy_from_slice(ancestors);
            return Self {
                inline,
                len: ancestors.len() as u8,
                overflow: None,
            };
        }
        Self {
            inline: [0; INLINE_CLIP_ANCESTOR_LEN],
            len: 0,
            overflow: Some(ancestors.into()),
        }
    }

    pub(in crate::runtime) fn as_slice(&self) -> &[NodeId] {
        self.overflow
            .as_deref()
            .unwrap_or(&self.inline[..self.len as usize])
    }

    #[cfg(test)]
    fn is_inline(&self) -> bool {
        self.overflow.is_none()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(in crate::runtime) struct SurfaceTraversalStats {
    pub(in crate::runtime) widgets: usize,
    pub(in crate::runtime) scroll_containers: usize,
    pub(in crate::runtime) styled_hoverable_containers: usize,
    pub(in crate::runtime) max_depth: usize,
    pub(in crate::runtime) max_scroll_depth: usize,
}

pub(in crate::runtime) struct SurfaceTraversalIndex {
    pub(in crate::runtime) widget_paint_order: Vec<WidgetId>,
    pub(in crate::runtime) focusable_widget_order: Vec<WidgetId>,
    pub(in crate::runtime) keyboard_focus_order: Vec<WidgetId>,
    pub(in crate::runtime) pointer_hit_order: Vec<WidgetId>,
    pub(in crate::runtime) wheel_hit_order: Vec<WidgetId>,
    pub(in crate::runtime) widget_paths: HashMap<WidgetId, WidgetPath>,
    pub(in crate::runtime) container_hover_suppression: HashSet<WidgetId>,
    pub(in crate::runtime) styled_container_order: Vec<NodeId>,
    pub(in crate::runtime) scroll_container_order: Vec<NodeId>,
    pub(in crate::runtime) widget_clip_ancestors: HashMap<WidgetId, ClipAncestors>,
    pub(in crate::runtime) container_clip_ancestors: HashMap<NodeId, ClipAncestors>,
    pub(in crate::runtime) scroll_content_by_container: HashMap<NodeId, NodeId>,
}

impl SurfaceTraversalIndex {
    pub(in crate::runtime) fn new() -> Self {
        Self {
            widget_paint_order: Vec::new(),
            focusable_widget_order: Vec::new(),
            keyboard_focus_order: Vec::new(),
            pointer_hit_order: Vec::new(),
            wheel_hit_order: Vec::new(),
            widget_paths: HashMap::new(),
            container_hover_suppression: HashSet::new(),
            styled_container_order: Vec::new(),
            scroll_container_order: Vec::new(),
            widget_clip_ancestors: HashMap::new(),
            container_clip_ancestors: HashMap::new(),
            scroll_content_by_container: HashMap::new(),
        }
    }

    pub(in crate::runtime) fn with_stats(stats: SurfaceTraversalStats) -> Self {
        Self {
            widget_paint_order: Vec::with_capacity(stats.widgets),
            focusable_widget_order: Vec::with_capacity(stats.widgets),
            keyboard_focus_order: Vec::with_capacity(stats.widgets),
            pointer_hit_order: Vec::with_capacity(stats.widgets),
            wheel_hit_order: Vec::with_capacity(stats.widgets),
            widget_paths: HashMap::with_capacity(stats.widgets),
            container_hover_suppression: HashSet::with_capacity(stats.widgets),
            styled_container_order: Vec::with_capacity(stats.styled_hoverable_containers),
            scroll_container_order: Vec::with_capacity(stats.scroll_containers),
            widget_clip_ancestors: HashMap::with_capacity(if stats.scroll_containers == 0 {
                0
            } else {
                stats.widgets
            }),
            container_clip_ancestors: HashMap::new(),
            scroll_content_by_container: HashMap::with_capacity(stats.scroll_containers),
        }
    }
}

impl<Message> SurfaceNode<Message> {
    pub(in crate::runtime) fn runtime_traversal_stats(&self) -> SurfaceTraversalStats {
        let mut stats = SurfaceTraversalStats::default();
        self.collect_runtime_traversal_stats(0, 0, &mut stats);
        stats
    }

    fn collect_runtime_traversal_stats(
        &self,
        depth: usize,
        scroll_depth: usize,
        stats: &mut SurfaceTraversalStats,
    ) {
        stats.max_depth = stats.max_depth.max(depth);
        stats.max_scroll_depth = stats.max_scroll_depth.max(scroll_depth);
        match self {
            Self::Container(container) => {
                let is_scroll = container.policy.kind == ContainerKind::ScrollView;
                if is_scroll {
                    stats.scroll_containers += 1;
                }
                if container.style.is_some() && container.hoverable {
                    stats.styled_hoverable_containers += 1;
                }
                let child_scroll_depth = scroll_depth + usize::from(is_scroll);
                for child in &container.children {
                    child.child.collect_runtime_traversal_stats(
                        depth + 1,
                        child_scroll_depth,
                        stats,
                    );
                }
            }
            Self::Widget(_) => {
                stats.widgets += 1;
            }
            Self::Overlay(_) => {}
        }
    }

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
                        .insert(container.id, ClipAncestors::from_slice(scroll_stack));
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
                        .insert(widget.id(), ClipAncestors::from_slice(scroll_stack));
                }
            }
            Self::Overlay(_) => {}
        }
    }
}

impl<Message> UiSurface<Message> {
    pub(in crate::runtime) fn runtime_traversal_index(&self) -> SurfaceTraversalIndex {
        let stats = self.root.runtime_traversal_stats();
        let mut index = SurfaceTraversalIndex::with_stats(stats);
        self.root.collect_runtime_index(
            &mut Vec::with_capacity(stats.max_scroll_depth),
            &mut Vec::with_capacity(stats.max_depth),
            &mut index,
        );
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

    #[test]
    fn clip_ancestors_use_inline_storage_for_common_scroll_depths() {
        let shallow = ClipAncestors::from_slice(&[10, 20]);
        assert!(shallow.is_inline());
        assert_eq!(shallow.as_slice(), &[10, 20]);

        let deep = ClipAncestors::from_slice(&[10, 20, 30]);
        assert!(!deep.is_inline());
        assert_eq!(deep.as_slice(), &[10, 20, 30]);
    }
}
