use super::SurfaceTraversalStats;
use crate::{
    layout::NodeId,
    runtime::{ClipAncestors, WidgetPath},
    widgets::WidgetId,
};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

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

pub(in crate::runtime) struct SurfaceContainerTraversalRecord<'a> {
    pub(in crate::runtime) id: NodeId,
    pub(in crate::runtime) clipped_by: &'a [NodeId],
    pub(in crate::runtime) scroll_content: Option<NodeId>,
    pub(in crate::runtime) styled_hoverable: bool,
}

pub(in crate::runtime) struct SurfaceWidgetTraversalRecord<'a> {
    pub(in crate::runtime) id: WidgetId,
    pub(in crate::runtime) child_path: &'a [usize],
    pub(in crate::runtime) clipped_by: &'a [NodeId],
    pub(in crate::runtime) focusable: bool,
    pub(in crate::runtime) keyboard_focusable: bool,
    pub(in crate::runtime) receives_pointer_hit_testing: bool,
    pub(in crate::runtime) receives_wheel_input: bool,
    pub(in crate::runtime) needs_state_synchronization: bool,
    pub(in crate::runtime) suppresses_container_hover: bool,
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
            widget_clip_ancestors: HashMap::with_capacity(widget_clip_capacity(stats)),
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
        reserve_map_capacity(&mut self.widget_clip_ancestors, widget_clip_capacity(stats));
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

    pub(in crate::runtime) fn record_container(
        &mut self,
        record: SurfaceContainerTraversalRecord<'_>,
    ) {
        if !record.clipped_by.is_empty() {
            self.container_clip_ancestors
                .insert(record.id, ClipAncestors::from_slice(record.clipped_by));
        }
        if let Some(content) = record.scroll_content {
            self.scroll_container_order.push(record.id);
            self.scroll_content_by_container.insert(record.id, content);
        }
        if record.styled_hoverable {
            self.styled_container_order.push(record.id);
        }
    }

    pub(in crate::runtime) fn record_widget(&mut self, record: SurfaceWidgetTraversalRecord<'_>) {
        self.widget_paint_order.push(record.id);
        self.widget_paths
            .entry(record.id)
            .or_insert_with(|| WidgetPath::from_slice(record.child_path));
        if record.focusable {
            self.focusable_widget_order.push(record.id);
        }
        if record.keyboard_focusable {
            self.keyboard_focus_order.push(record.id);
        }
        if record.receives_pointer_hit_testing {
            self.pointer_hit_order.push(record.id);
        }
        if record.receives_wheel_input {
            self.wheel_hit_order.push(record.id);
        }
        if record.needs_state_synchronization {
            self.stateful_widget_order.push(record.id);
        }
        if record.suppresses_container_hover {
            self.container_hover_suppression.insert(record.id);
        }
        if !record.clipped_by.is_empty() {
            self.widget_clip_ancestors
                .insert(record.id, ClipAncestors::from_slice(record.clipped_by));
        }
    }
}

fn widget_clip_capacity(stats: SurfaceTraversalStats) -> usize {
    if stats.scroll_containers == 0 {
        0
    } else {
        stats.widgets
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_clip_capacity_is_zero_without_scroll_containers() {
        assert_eq!(
            widget_clip_capacity(SurfaceTraversalStats {
                widgets: 8,
                scroll_containers: 0,
                ..SurfaceTraversalStats::default()
            }),
            0
        );
    }

    #[test]
    fn widget_clip_capacity_tracks_widgets_when_scroll_containers_exist() {
        assert_eq!(
            widget_clip_capacity(SurfaceTraversalStats {
                widgets: 8,
                scroll_containers: 1,
                ..SurfaceTraversalStats::default()
            }),
            8
        );
    }

    #[test]
    fn traversal_records_route_to_expected_buckets() {
        let mut index = SurfaceTraversalIndex::with_stats(SurfaceTraversalStats {
            widgets: 1,
            stateful_widgets: 1,
            styled_hoverable_containers: 1,
            scroll_containers: 1,
            clipped_containers: 1,
            max_depth: 1,
            max_scroll_depth: 1,
        });

        index.record_container(SurfaceContainerTraversalRecord {
            id: 10,
            clipped_by: &[1],
            scroll_content: Some(11),
            styled_hoverable: true,
        });
        index.record_widget(SurfaceWidgetTraversalRecord {
            id: 20,
            child_path: &[0, 1],
            clipped_by: &[10],
            focusable: true,
            keyboard_focusable: true,
            receives_pointer_hit_testing: true,
            receives_wheel_input: true,
            needs_state_synchronization: true,
            suppresses_container_hover: true,
        });

        assert_eq!(index.scroll_container_order, vec![10]);
        assert_eq!(index.scroll_content_by_container.get(&10), Some(&11));
        assert_eq!(index.styled_container_order, vec![10]);
        assert_eq!(
            index
                .container_clip_ancestors
                .get(&10)
                .map(|path| path.as_slice()),
            Some(&[1][..])
        );
        assert_eq!(index.widget_paint_order, vec![20]);
        assert_eq!(index.focusable_widget_order, vec![20]);
        assert_eq!(index.keyboard_focus_order, vec![20]);
        assert_eq!(index.pointer_hit_order, vec![20]);
        assert_eq!(index.wheel_hit_order, vec![20]);
        assert_eq!(index.stateful_widget_order, vec![20]);
        assert!(index.container_hover_suppression.contains(&20));
        assert_eq!(
            index.widget_paths.get(&20).map(|path| path.as_slice()),
            Some(&[0, 1][..])
        );
        assert_eq!(
            index
                .widget_clip_ancestors
                .get(&20)
                .map(|path| path.as_slice()),
            Some(&[10][..])
        );
    }
}
