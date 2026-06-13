use super::{UiSurface, WidgetPath};
use crate::widgets::WidgetId;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::runtime) struct WidgetStateSyncPolicy {
    exclusive_pointer_capture: Option<WidgetId>,
    retained_hover_owner: Option<WidgetId>,
}

impl WidgetStateSyncPolicy {
    pub(in crate::runtime) fn exclusive_pointer_capture(widget_id: WidgetId) -> Self {
        Self {
            exclusive_pointer_capture: Some(widget_id),
            retained_hover_owner: Some(widget_id),
        }
    }

    pub(in crate::runtime) fn retained_hover_owner(widget_id: Option<WidgetId>) -> Self {
        Self {
            exclusive_pointer_capture: None,
            retained_hover_owner: widget_id,
        }
    }

    pub(in crate::runtime) fn clears_retained_hover_for(self, widget_id: WidgetId) -> bool {
        if let Some(captured) = self.exclusive_pointer_capture {
            return captured != widget_id;
        }
        self.retained_hover_owner != Some(widget_id)
    }
}

impl<Message> UiSurface<Message> {
    pub(in crate::runtime) fn synchronize_widget_state_from_paths(
        &mut self,
        previous: &Self,
        stateful_widget_order: &[WidgetId],
        current_paths: &HashMap<WidgetId, WidgetPath>,
        previous_paths: &HashMap<WidgetId, WidgetPath>,
        policy: WidgetStateSyncPolicy,
    ) {
        self.root.synchronize_widget_state_from_paths(
            stateful_widget_order,
            current_paths,
            &previous.root,
            previous_paths,
            policy,
        );
    }
}
