use super::{
    ClipAncestors, SurfaceContainerTraversalRecord, SurfaceTraversalIndex,
    SurfaceWidgetTraversalRecord, WidgetPath,
};

impl SurfaceTraversalIndex {
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
