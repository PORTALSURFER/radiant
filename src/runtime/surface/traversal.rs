use super::*;
use crate::layout::ContainerKind;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Default)]
pub(in crate::runtime) struct SurfaceTraversalStats {
    pub(in crate::runtime) widgets: usize,
    pub(in crate::runtime) stateful_widgets: usize,
    pub(in crate::runtime) scroll_containers: usize,
    pub(in crate::runtime) clipped_containers: usize,
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
    pub(in crate::runtime) stateful_widget_order: Vec<WidgetId>,
    pub(in crate::runtime) widget_paths: HashMap<WidgetId, WidgetPath>,
    pub(in crate::runtime) container_hover_suppression: HashSet<WidgetId>,
    pub(in crate::runtime) styled_container_order: Vec<NodeId>,
    pub(in crate::runtime) scroll_container_order: Vec<NodeId>,
    pub(in crate::runtime) widget_clip_ancestors: HashMap<WidgetId, ClipAncestors>,
    pub(in crate::runtime) container_clip_ancestors: HashMap<NodeId, ClipAncestors>,
    pub(in crate::runtime) scroll_content_by_container: HashMap<NodeId, NodeId>,
}

impl SurfaceTraversalIndex {
    pub(in crate::runtime) fn with_stats(stats: SurfaceTraversalStats) -> Self {
        Self {
            widget_paint_order: Vec::with_capacity(stats.widgets),
            focusable_widget_order: Vec::with_capacity(stats.widgets),
            keyboard_focus_order: Vec::with_capacity(stats.widgets),
            pointer_hit_order: Vec::with_capacity(stats.widgets),
            wheel_hit_order: Vec::with_capacity(stats.widgets),
            stateful_widget_order: Vec::with_capacity(stats.stateful_widgets),
            widget_paths: HashMap::with_capacity(stats.widgets),
            container_hover_suppression: HashSet::with_capacity(stats.widgets),
            styled_container_order: Vec::with_capacity(stats.styled_hoverable_containers),
            scroll_container_order: Vec::with_capacity(stats.scroll_containers),
            widget_clip_ancestors: HashMap::with_capacity(if stats.scroll_containers == 0 {
                0
            } else {
                stats.widgets
            }),
            container_clip_ancestors: HashMap::with_capacity(stats.clipped_containers),
            scroll_content_by_container: HashMap::with_capacity(stats.scroll_containers),
        }
    }

    pub(in crate::runtime) fn clear_for_stats(&mut self, stats: SurfaceTraversalStats) {
        self.widget_paint_order.clear();
        reserve_vec_capacity(&mut self.widget_paint_order, stats.widgets);
        self.focusable_widget_order.clear();
        reserve_vec_capacity(&mut self.focusable_widget_order, stats.widgets);
        self.keyboard_focus_order.clear();
        reserve_vec_capacity(&mut self.keyboard_focus_order, stats.widgets);
        self.pointer_hit_order.clear();
        reserve_vec_capacity(&mut self.pointer_hit_order, stats.widgets);
        self.wheel_hit_order.clear();
        reserve_vec_capacity(&mut self.wheel_hit_order, stats.widgets);
        self.stateful_widget_order.clear();
        reserve_vec_capacity(&mut self.stateful_widget_order, stats.stateful_widgets);
        self.widget_paths.clear();
        reserve_map_capacity(&mut self.widget_paths, stats.widgets);
        self.container_hover_suppression.clear();
        reserve_set_capacity(&mut self.container_hover_suppression, stats.widgets);
        self.styled_container_order.clear();
        reserve_vec_capacity(
            &mut self.styled_container_order,
            stats.styled_hoverable_containers,
        );
        self.scroll_container_order.clear();
        reserve_vec_capacity(&mut self.scroll_container_order, stats.scroll_containers);
        self.widget_clip_ancestors.clear();
        let clip_capacity = if stats.scroll_containers == 0 {
            0
        } else {
            stats.widgets
        };
        reserve_map_capacity(&mut self.widget_clip_ancestors, clip_capacity);
        self.container_clip_ancestors.clear();
        reserve_map_capacity(&mut self.container_clip_ancestors, stats.clipped_containers);
        self.scroll_content_by_container.clear();
        reserve_map_capacity(
            &mut self.scroll_content_by_container,
            stats.scroll_containers,
        );
    }

    pub(in crate::runtime) fn clear_for_reuse(&mut self) {
        self.widget_paint_order.clear();
        self.focusable_widget_order.clear();
        self.keyboard_focus_order.clear();
        self.pointer_hit_order.clear();
        self.wheel_hit_order.clear();
        self.stateful_widget_order.clear();
        self.widget_paths.clear();
        self.container_hover_suppression.clear();
        self.styled_container_order.clear();
        self.scroll_container_order.clear();
        self.widget_clip_ancestors.clear();
        self.container_clip_ancestors.clear();
        self.scroll_content_by_container.clear();
    }
}

fn reserve_vec_capacity<T>(values: &mut Vec<T>, desired_capacity: usize) {
    if desired_capacity > values.capacity() {
        values.reserve(desired_capacity);
    }
}

fn reserve_map_capacity<K, V>(values: &mut HashMap<K, V>, desired_capacity: usize)
where
    K: Eq + Hash,
{
    if desired_capacity > values.capacity() {
        values.reserve(desired_capacity);
    }
}

fn reserve_set_capacity<T>(values: &mut HashSet<T>, desired_capacity: usize)
where
    T: Eq + Hash,
{
    if desired_capacity > values.capacity() {
        values.reserve(desired_capacity);
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
                if scroll_depth > 0 {
                    stats.clipped_containers += 1;
                }
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
            Self::Widget(widget) => {
                stats.widgets += 1;
                if widget.needs_state_synchronization() {
                    stats.stateful_widgets += 1;
                }
            }
            Self::Overlay(_) => {}
        }
    }
}

impl<Message> UiSurface<Message> {
    #[cfg(test)]
    pub(in crate::runtime) fn runtime_traversal_index(&self) -> SurfaceTraversalIndex {
        let stats = self.root.runtime_traversal_stats();
        let mut index = SurfaceTraversalIndex::with_stats(stats);
        self.root.project_runtime_index(
            &mut Vec::with_capacity(stats.max_scroll_depth),
            &mut Vec::with_capacity(stats.max_depth),
            &mut index,
        );
        index
    }
}

#[cfg(test)]
#[path = "traversal/tests.rs"]
mod tests;
