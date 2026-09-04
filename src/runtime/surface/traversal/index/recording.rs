use super::{
    ClipAncestors, SurfaceContainerTraversalRecord, SurfaceSplitPaneFocusOrderCandidate,
    SurfaceTraversalIndex, SurfaceWidgetTraversalRecord, WheelHitTarget, WidgetPath,
};
use std::collections::hash_map::Entry;

impl<Message> SurfaceTraversalIndex<Message> {
    pub(in crate::runtime) fn record_container(
        &mut self,
        record: SurfaceContainerTraversalRecord<'_, Message>,
    ) {
        if !record.clipped_by.is_empty() {
            self.container_clip_ancestors
                .insert(record.id, ClipAncestors::from_slice(record.clipped_by));
        }
        if let Some(content) = record.scroll_content {
            self.scroll_container_order.push(record.id);
            self.wheel_target_order
                .push(WheelHitTarget::ScrollContainer(record.id));
            self.scroll_content_by_container.insert(record.id, content);
        }
        if record.styled_hoverable {
            self.styled_container_order.push(record.id);
        }
        if let Some(interaction) = record.layout_interaction {
            self.layout_interactions.push(interaction);
        }
        if let Some(split_pane_runtime) = record.split_pane_runtime {
            self.split_pane_runtime.push(split_pane_runtime);
        }
        if let Some(split_pane_divider) = record.split_pane_divider {
            self.split_pane_dividers.push(split_pane_divider);
        }
        if let Some(split_pane_ratio_action) = record.split_pane_ratio_action {
            self.split_pane_ratio_action_candidates
                .push(split_pane_ratio_action);
        }
        if let Some(registration) = record.virtual_layout {
            self.virtual_layout_registrations.push(registration);
        }
    }

    pub(in crate::runtime) fn record_widget(&mut self, record: SurfaceWidgetTraversalRecord<'_>) {
        self.widget_paint_order.push(record.id);
        if let Entry::Vacant(entry) = self.widget_paths.entry(record.id) {
            entry.insert(WidgetPath::from_slice(record.child_path));
            self.widget_membership.insert(
                record.id,
                [
                    record.focusable,
                    record.keyboard_focusable,
                    record.receives_pointer_hit_testing,
                    record.receives_wheel_input,
                    record.accepts_native_file_drop,
                    record.needs_state_synchronization,
                    record.suppresses_container_hover,
                ],
            );
        } else {
            self.duplicate_widget_ids.insert(record.id);
        }
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
            self.wheel_target_order
                .push(WheelHitTarget::Widget(record.id));
        }
        if record.accepts_native_file_drop {
            self.native_file_drop_hit_order.push(record.id);
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

    pub(in crate::runtime) fn record_split_pane_focus_order_candidate(
        &mut self,
        mut candidate: SurfaceSplitPaneFocusOrderCandidate,
    ) {
        candidate.widget_index = self.keyboard_focus_order.len();
        self.keyboard_focus_order_candidates.push(candidate);
    }
}
