use super::{UiSurface, WidgetPath};
use crate::widgets::WidgetId;
use std::collections::HashMap;

impl<Message> UiSurface<Message> {
    pub(in crate::runtime) fn synchronize_widget_state_from_paths(
        &mut self,
        previous: &Self,
        stateful_widget_order: &[WidgetId],
        current_paths: &HashMap<WidgetId, WidgetPath>,
        previous_paths: &HashMap<WidgetId, WidgetPath>,
    ) {
        self.root.synchronize_widget_state_from_paths(
            stateful_widget_order,
            current_paths,
            &previous.root,
            previous_paths,
        );
    }
}
