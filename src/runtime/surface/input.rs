use super::state_sync::{
    PreparedWidgetStateSyncEvidence, PreparedWidgetStateSyncVeto, WidgetStateSyncEvidence,
};
use super::{
    SurfaceNode, SurfaceWidget, WidgetPath, WidgetStateSyncPolicy, node::SurfaceLayerChildKind,
};
use crate::{
    gui::types::Rect,
    widgets::{CompositionSample, WidgetId, WidgetInput, WidgetOutput},
};
use std::collections::HashMap;
use std::time::Instant;

pub(in crate::runtime) enum WidgetDispatchResult<Message> {
    NoOutput,
    UnmappedOutput,
    Message(Message),
}

impl<Message> SurfaceNode<Message> {
    pub(super) fn commit_widget_replacement_at_path(
        &mut self,
        widget_id: WidgetId,
        child_path: &[usize],
        successor: Option<&dyn crate::widgets::Widget>,
    ) -> (bool, WidgetDispatchResult<Message>) {
        let Some(widget) = self
            .find_widget_mut_at_path(child_path)
            .filter(|widget| widget.id() == widget_id)
        else {
            return (false, WidgetDispatchResult::NoOutput);
        };
        let Some(output) = widget.prepare_replacement(successor) else {
            return (true, WidgetDispatchResult::NoOutput);
        };
        let result = widget
            .dispatch_output(widget_id, output)
            .map(WidgetDispatchResult::Message)
            .unwrap_or(WidgetDispatchResult::UnmappedOutput);
        (true, result)
    }

    pub(super) fn commit_widget_replacement(
        &mut self,
        widget_id: WidgetId,
        successor: Option<&dyn crate::widgets::Widget>,
    ) -> (bool, WidgetDispatchResult<Message>) {
        let Some(widget) = self.find_widget_mut(widget_id) else {
            return (false, WidgetDispatchResult::NoOutput);
        };
        let Some(output) = widget.prepare_replacement(successor) else {
            return (true, WidgetDispatchResult::NoOutput);
        };
        let result = widget
            .dispatch_output(widget_id, output)
            .map(WidgetDispatchResult::Message)
            .unwrap_or(WidgetDispatchResult::UnmappedOutput);
        (true, result)
    }

    pub(super) fn widget_compatibility_at_path(
        &self,
        child_path: &[usize],
    ) -> Option<(&'static str, bool)> {
        self.find_widget_at_path(child_path).map(|widget| {
            (
                widget.compatibility_kind(),
                widget.revision_evidence().valid,
            )
        })
    }

    #[allow(dead_code)]
    pub(super) fn synchronize_widget_state_from_paths(
        &mut self,
        stateful_widget_order: &[WidgetId],
        current_paths: &HashMap<WidgetId, WidgetPath>,
        previous: &Self,
        previous_paths: &HashMap<WidgetId, WidgetPath>,
        policy: WidgetStateSyncPolicy,
    ) {
        self.synchronize_widget_state_from_paths_with_evidence(
            WidgetStateSyncEvidence {
                stateful_widget_order,
                current_paths,
                previous_paths,
                previous_widget_order: &[],
                current_widget_order: &[],
                retired_widget_ids: &[],
                policy,
            },
            previous,
        );
    }

    pub(super) fn synchronize_widget_state_from_paths_with_evidence(
        &mut self,
        evidence: WidgetStateSyncEvidence<'_>,
        previous: &Self,
    ) {
        for widget_id in evidence.stateful_widget_order {
            if evidence.retired_widget_ids.contains(widget_id) {
                continue;
            }
            if !evidence.previous_widget_order.is_empty()
                && (!has_unique_widget_id(evidence.previous_widget_order, *widget_id)
                    || !has_unique_widget_id(evidence.current_widget_order, *widget_id))
            {
                continue;
            }
            let Some(current_path) = evidence.current_paths.get(widget_id) else {
                continue;
            };
            let Some(previous_path) = evidence.previous_paths.get(widget_id) else {
                continue;
            };
            let Some(previous_widget) = previous
                .find_widget_at_path(previous_path.as_slice())
                .filter(|widget| widget.id() == *widget_id)
            else {
                continue;
            };
            let Some(current_widget) = self
                .find_widget_mut_at_path(current_path.as_slice())
                .filter(|widget| widget.id() == *widget_id)
            else {
                continue;
            };
            if !current_widget.revision_evidence().valid
                || !previous_widget.revision_evidence().valid
                || current_widget.compatibility_kind() != previous_widget.compatibility_kind()
            {
                continue;
            }
            current_widget
                .widget_object_mut_runtime()
                .synchronize_from_previous(previous_widget.widget_object());
            if evidence.policy.clears_retained_hover_for(*widget_id) {
                current_widget
                    .widget_object_mut_runtime()
                    .common_mut()
                    .state
                    .hovered = false;
            }
        }
    }

    pub(super) fn preflight_prepared_widget_state_sync(
        &self,
        evidence: &PreparedWidgetStateSyncEvidence<'_>,
        previous: &Self,
    ) -> Result<(), PreparedWidgetStateSyncVeto> {
        for widget_id in evidence.stateful_widget_order {
            if !has_unique_widget_id_prepared(evidence.previous_widget_order, *widget_id)
                || !has_unique_widget_id_prepared(evidence.current_widget_order, *widget_id)
            {
                return Err(PreparedWidgetStateSyncVeto::Ambiguous);
            }

            let previous_path = evidence
                .previous_paths
                .get(widget_id)
                .ok_or(PreparedWidgetStateSyncVeto::InvalidPath)?;
            let current_path = evidence
                .current_paths
                .get(widget_id)
                .ok_or(PreparedWidgetStateSyncVeto::InvalidPath)?;
            let previous_widget = previous
                .find_widget_at_path(previous_path.as_slice())
                .ok_or(PreparedWidgetStateSyncVeto::InvalidPath)?;
            let current_widget = self
                .find_widget_at_path(current_path.as_slice())
                .ok_or(PreparedWidgetStateSyncVeto::InvalidPath)?;

            if previous_widget.id() != *widget_id || current_widget.id() != *widget_id {
                return Err(PreparedWidgetStateSyncVeto::InvalidIdentity);
            }
            if !previous_widget.revision_evidence().valid
                || !current_widget.revision_evidence().valid
            {
                return Err(PreparedWidgetStateSyncVeto::InvalidRevision);
            }
            if previous_widget.compatibility_kind() != current_widget.compatibility_kind() {
                return Err(PreparedWidgetStateSyncVeto::Incompatible);
            }
            if !current_widget.supports_prepared_state_synchronization() {
                return Err(PreparedWidgetStateSyncVeto::Unsupported);
            }
        }
        Ok(())
    }

    pub(super) fn synchronize_prepared_widget_state(
        &mut self,
        evidence: &PreparedWidgetStateSyncEvidence<'_>,
        previous: &Self,
    ) -> Result<(), PreparedWidgetStateSyncVeto> {
        for widget_id in evidence.stateful_widget_order {
            let previous_path = evidence
                .previous_paths
                .get(widget_id)
                .ok_or(PreparedWidgetStateSyncVeto::InvalidPath)?;
            let current_path = evidence
                .current_paths
                .get(widget_id)
                .ok_or(PreparedWidgetStateSyncVeto::InvalidPath)?;
            let previous_widget = previous
                .find_widget_at_path(previous_path.as_slice())
                .ok_or(PreparedWidgetStateSyncVeto::InvalidPath)?;
            let current_widget = self
                .find_widget_mut_at_path(current_path.as_slice())
                .filter(|widget| widget.id() == *widget_id)
                .ok_or(PreparedWidgetStateSyncVeto::InvalidIdentity)?;

            current_widget
                .widget_object_mut_runtime()
                .synchronize_from_previous(previous_widget.widget_object());
            if evidence.policy.clears_retained_hover_for(*widget_id) {
                current_widget
                    .widget_object_mut_runtime()
                    .common_mut()
                    .state
                    .hovered = false;
            }
        }
        Ok(())
    }

    pub(super) fn handle_input(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<WidgetOutput> {
        self.find_widget_mut(widget_id)
            .and_then(|widget| widget.handle_input(widget_id, bounds, input))
    }

    pub(super) fn dispatch_input_at_path(
        &mut self,
        widget_id: WidgetId,
        child_path: &[usize],
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<WidgetDispatchResult<Message>> {
        self.find_widget_mut_at_path(child_path)
            .filter(|widget| widget.id() == widget_id)
            .map(|widget| widget.dispatch_input(widget_id, bounds, input))
    }

    pub(super) fn dispatch_focus_changed_at_path(
        &mut self,
        widget_id: WidgetId,
        child_path: &[usize],
        bounds: Rect,
        focused: bool,
        now: Instant,
    ) -> Option<WidgetDispatchResult<Message>> {
        self.find_widget_mut_at_path(child_path)
            .filter(|widget| widget.id() == widget_id)
            .map(|widget| widget.dispatch_focus_changed_at(widget_id, bounds, focused, now))
    }

    pub(super) fn dispatch_composition_sample_at_path(
        &mut self,
        widget_id: WidgetId,
        child_path: &[usize],
        sample: CompositionSample,
    ) -> Option<(WidgetDispatchResult<Message>, bool)> {
        self.find_widget_mut_at_path(child_path)
            .filter(|widget| widget.id() == widget_id)
            .map(|widget| widget.dispatch_composition_sample(widget_id, sample))
    }

    pub(super) fn dispatch_hidden_composition_update_at_path(
        &mut self,
        widget_id: WidgetId,
        child_path: &[usize],
        preedit: String,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> Option<(WidgetDispatchResult<Message>, bool)> {
        self.find_widget_mut_at_path(child_path)
            .filter(|widget| widget.id() == widget_id)
            .map(|widget| widget.dispatch_hidden_composition_update(widget_id, preedit, timestamp))
    }

    #[allow(dead_code)]
    pub(super) fn dispatch_pointer_capture_cancelled_at_path(
        &mut self,
        widget_id: WidgetId,
        child_path: &[usize],
        bounds: Rect,
    ) -> Option<WidgetDispatchResult<Message>> {
        self.dispatch_pointer_capture_cancelled_at_path_with_clock(
            widget_id,
            child_path,
            bounds,
            Instant::now(),
        )
    }

    pub(super) fn dispatch_pointer_capture_cancelled_at_path_with_clock(
        &mut self,
        widget_id: WidgetId,
        child_path: &[usize],
        bounds: Rect,
        now: Instant,
    ) -> Option<WidgetDispatchResult<Message>> {
        self.find_widget_mut_at_path(child_path)
            .filter(|widget| widget.id() == widget_id)
            .map(|widget| widget.dispatch_pointer_capture_cancelled_at(widget_id, bounds, now))
    }

    pub(super) fn dispatch_output(
        &self,
        widget_id: WidgetId,
        output: &WidgetOutput,
    ) -> Option<Message> {
        match self {
            Self::Scene(scene) => scene.base.dispatch_output(widget_id, output).or_else(|| {
                scene.ordered_layers().find_map(|layer| {
                    layer
                        .input
                        .as_ref()
                        .and_then(|input| input.dispatch_output(widget_id, output))
                        .or_else(|| layer.node.dispatch_output(widget_id, output))
                })
            }),
            Self::Container(container) => container
                .children
                .iter()
                .find_map(|child| child.child.dispatch_output(widget_id, output)),
            Self::Widget(widget) if widget.id() == widget_id => {
                widget.dispatch_output(widget_id, output.clone())
            }
            Self::Widget(_) => None,
            Self::Overlay(_) => None,
            Self::FloatingLayer(layer) if layer.interactive => layer
                .container
                .children
                .iter()
                .find_map(|child| child.child.dispatch_output(widget_id, output)),
            Self::FloatingLayer(_) => None,
        }
    }

    pub(super) fn find_widget(&self, widget_id: WidgetId) -> Option<&SurfaceWidget<Message>> {
        match self {
            Self::Scene(scene) => scene.base.find_widget(widget_id).or_else(|| {
                scene.ordered_layers().find_map(|layer| {
                    layer
                        .input
                        .as_ref()
                        .and_then(|input| input.find_widget(widget_id))
                        .or_else(|| layer.node.find_widget(widget_id))
                })
            }),
            Self::Container(container) => container
                .children
                .iter()
                .find_map(|child| child.child.find_widget(widget_id)),
            Self::Widget(widget) => (widget.id() == widget_id).then_some(widget),
            Self::Overlay(_) => None,
            Self::FloatingLayer(layer) if layer.interactive => layer
                .container
                .children
                .iter()
                .find_map(|child| child.child.find_widget(widget_id)),
            Self::FloatingLayer(_) => None,
        }
    }

    pub(super) fn find_widget_at_path(
        &self,
        child_path: &[usize],
    ) -> Option<&SurfaceWidget<Message>> {
        match (self, child_path.split_first()) {
            (Self::Widget(widget), None) => Some(widget),
            (Self::Scene(scene), path) => {
                if !scene.has_layers() {
                    return scene.base.find_widget_at_path(child_path);
                }

                let (child_index, remaining_path) = path?;
                if *child_index == 0 {
                    return scene.base.find_widget_at_path(remaining_path);
                }

                let (layer_index, child_kind) =
                    scene.ordered_layer_child_for_child(*child_index - 1)?;
                match child_kind {
                    SurfaceLayerChildKind::Input => scene.layers[layer_index]
                        .input
                        .as_ref()?
                        .find_widget_at_path(remaining_path),
                    SurfaceLayerChildKind::Foreground => scene.layers[layer_index]
                        .node
                        .find_widget_at_path(remaining_path),
                }
            }
            (Self::Container(container), Some((child_index, remaining_path))) => container
                .children
                .get(*child_index)?
                .child
                .find_widget_at_path(remaining_path),
            (Self::FloatingLayer(layer), Some((child_index, remaining_path)))
                if layer.interactive =>
            {
                layer
                    .container
                    .children
                    .get(*child_index)?
                    .child
                    .find_widget_at_path(remaining_path)
            }
            _ => None,
        }
    }

    pub(super) fn find_widget_mut(
        &mut self,
        widget_id: WidgetId,
    ) -> Option<&mut SurfaceWidget<Message>> {
        match self {
            Self::Scene(scene) => {
                if let Some(widget) = scene.base.find_widget_mut(widget_id) {
                    return Some(widget);
                }
                scene.layers.iter_mut().find_map(|layer| {
                    if let Some(input) = &mut layer.input
                        && let Some(widget) = input.find_widget_mut(widget_id)
                    {
                        return Some(widget);
                    }
                    layer.node.find_widget_mut(widget_id)
                })
            }
            Self::Container(container) => container
                .children
                .iter_mut()
                .find_map(|child| child.child.find_widget_mut(widget_id)),
            Self::Widget(widget) => (widget.id() == widget_id).then_some(widget),
            Self::Overlay(_) => None,
            Self::FloatingLayer(layer) if layer.interactive => layer
                .container
                .children
                .iter_mut()
                .find_map(|child| child.child.find_widget_mut(widget_id)),
            Self::FloatingLayer(_) => None,
        }
    }

    pub(super) fn find_widget_mut_at_path(
        &mut self,
        child_path: &[usize],
    ) -> Option<&mut SurfaceWidget<Message>> {
        match (self, child_path.split_first()) {
            (Self::Widget(widget), None) => Some(widget),
            (Self::Scene(scene), path) => {
                if !scene.has_layers() {
                    return scene.base.find_widget_mut_at_path(child_path);
                }

                let (child_index, remaining_path) = path?;
                if *child_index == 0 {
                    return scene.base.find_widget_mut_at_path(remaining_path);
                }

                let (layer_index, child_kind) =
                    scene.ordered_layer_child_for_child(*child_index - 1)?;
                match child_kind {
                    SurfaceLayerChildKind::Input => scene.layers[layer_index]
                        .input
                        .as_mut()?
                        .find_widget_mut_at_path(remaining_path),
                    SurfaceLayerChildKind::Foreground => scene.layers[layer_index]
                        .node
                        .find_widget_mut_at_path(remaining_path),
                }
            }
            (Self::Container(container), Some((child_index, remaining_path))) => container
                .children
                .get_mut(*child_index)?
                .child
                .find_widget_mut_at_path(remaining_path),
            (Self::FloatingLayer(layer), Some((child_index, remaining_path)))
                if layer.interactive =>
            {
                layer
                    .container
                    .children
                    .get_mut(*child_index)?
                    .child
                    .find_widget_mut_at_path(remaining_path)
            }
            _ => None,
        }
    }
}

fn has_unique_widget_id_prepared(widget_order: &[WidgetId], widget_id: WidgetId) -> bool {
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

#[cfg(test)]
#[path = "input/tests.rs"]
mod tests;
