use super::{UiSurface, WidgetDispatchResult, WidgetPath};
use crate::widgets::WidgetId;
use std::collections::{HashMap, HashSet};

pub(in crate::runtime) struct WidgetStateSyncEvidence<'a> {
    pub(in crate::runtime::surface) stateful_widget_order: &'a [WidgetId],
    pub(in crate::runtime::surface) current_paths: &'a HashMap<WidgetId, WidgetPath>,
    pub(in crate::runtime::surface) previous_paths: &'a HashMap<WidgetId, WidgetPath>,
    pub(in crate::runtime::surface) previous_widget_order: &'a [WidgetId],
    pub(in crate::runtime::surface) current_widget_order: &'a [WidgetId],
    pub(in crate::runtime::surface) retired_widget_ids: &'a [WidgetId],
    pub(in crate::runtime::surface) policy: WidgetStateSyncPolicy,
}

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
    pub(in crate::runtime) fn prepare_widget_replacements(
        &mut self,
        successor: &Self,
        previous_stateful_widget_order: &[WidgetId],
        previous_widget_order: &[WidgetId],
        current_widget_order: &[WidgetId],
        current_paths: &HashMap<WidgetId, WidgetPath>,
        previous_paths: &HashMap<WidgetId, WidgetPath>,
    ) -> (Vec<Message>, Vec<WidgetId>) {
        let mut messages = Vec::with_capacity(previous_stateful_widget_order.len());
        let mut retired_widget_ids = Vec::with_capacity(previous_stateful_widget_order.len());
        let mut visited = HashSet::with_capacity(previous_stateful_widget_order.len());

        for widget_id in previous_stateful_widget_order {
            if !visited.insert(*widget_id) {
                continue;
            }

            let previous_path = previous_paths.get(widget_id);
            let successor_widget = match (previous_path, current_paths.get(widget_id)) {
                (Some(previous_path), Some(current_path))
                    if has_unique_widget_id(previous_widget_order, *widget_id)
                        && has_unique_widget_id(current_widget_order, *widget_id) =>
                {
                    let previous_widget = self.find_widget_at_path(*widget_id, previous_path);
                    let current_widget = successor.find_widget_at_path(*widget_id, current_path);
                    match (previous_widget, current_widget) {
                        (Some(previous_widget), Some(current_widget))
                            if previous_widget.revision_evidence().valid
                                && current_widget.revision_evidence().valid
                                && previous_widget.compatibility_kind()
                                    == current_widget.compatibility_kind() =>
                        {
                            Some(current_widget.widget_object())
                        }
                        _ => None,
                    }
                }
                _ => None,
            };

            let (called, result) = match previous_path {
                Some(previous_path) => {
                    let prepared = self.root.prepare_widget_replacement_at_path(
                        *widget_id,
                        previous_path.as_slice(),
                        successor_widget,
                    );
                    if prepared.0 {
                        prepared
                    } else {
                        self.root
                            .prepare_widget_replacement(*widget_id, successor_widget)
                    }
                }
                None => self
                    .root
                    .prepare_widget_replacement(*widget_id, successor_widget),
            };
            if !called {
                continue;
            }

            if !matches!(result, WidgetDispatchResult::NoOutput) {
                retired_widget_ids.push(*widget_id);
            }
            if let WidgetDispatchResult::Message(message) = result {
                messages.push(message);
            }
        }

        (messages, retired_widget_ids)
    }

    pub(in crate::runtime) fn widget_compatibility_at_path(
        &self,
        path: &[usize],
    ) -> Option<(&'static str, bool)> {
        self.root.widget_compatibility_at_path(path)
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::runtime) fn synchronize_widget_state_from_paths_with_evidence(
        &mut self,
        previous: &Self,
        stateful_widget_order: &[WidgetId],
        current_paths: &HashMap<WidgetId, WidgetPath>,
        previous_paths: &HashMap<WidgetId, WidgetPath>,
        previous_widget_order: &[WidgetId],
        current_widget_order: &[WidgetId],
        retired_widget_ids: &[WidgetId],
        policy: WidgetStateSyncPolicy,
    ) {
        self.root.synchronize_widget_state_from_paths_with_evidence(
            WidgetStateSyncEvidence {
                stateful_widget_order,
                current_paths,
                previous_paths,
                previous_widget_order,
                current_widget_order,
                retired_widget_ids,
                policy,
            },
            &previous.root,
        );
    }
}

fn has_unique_widget_id(widget_order: &[WidgetId], widget_id: WidgetId) -> bool {
    let mut found = false;
    for candidate in widget_order {
        if *candidate != widget_id {
            continue;
        }
        if found {
            return false;
        }
        found = true;
    }
    found
}
