use super::source::SourceMetadata;
use super::state_sync::{
    PreparedWidgetStateSyncEvidence, PreparedWidgetStateSyncLeafWitness,
    PreparedWidgetStateSyncVeto, PreparedWidgetStateSyncWitness, WidgetStateSyncEvidence,
};
use super::{
    SurfaceNode, SurfaceWidget, WidgetPath, WidgetStateSyncPolicy, node::SurfaceLayerChildKind,
};
use crate::{
    gui::types::Rect,
    widgets::{CompositionSample, WidgetId, WidgetInput, WidgetOutput},
};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

pub(in crate::runtime) enum ResolvedWidgetDispatchResult<Message> {
    NoOutput,
    UnmappedOutput,
    Message(Message),
}

pub(in crate::runtime) enum WidgetDispatchResult<Message> {
    Command(crate::application::CommandActivation),
    NoOutput,
    UnmappedOutput,
    Message(Message),
}

impl<Message> SurfaceNode<Message> {
    /// Capture source metadata for every retained node on one exact path.
    ///
    /// Keyed source evidence is shared through `Rc` records whose contents can
    /// be revised by the producer. Keeping these handles in the prepared
    /// witness lets post-sync admission observe such a mutation without
    /// traversing unrelated siblings.
    pub(super) fn source_metadata_path_at(
        &self,
        child_path: &[usize],
    ) -> Option<Vec<Option<Rc<SourceMetadata>>>> {
        let mut metadata = Vec::with_capacity(child_path.len() + 1);
        self.collect_source_metadata_path(child_path, &mut metadata)?;
        Some(metadata)
    }

    fn collect_source_metadata_path(
        &self,
        child_path: &[usize],
        metadata: &mut Vec<Option<Rc<SourceMetadata>>>,
    ) -> Option<()> {
        metadata.push(self.source_metadata_handle());
        match (self, child_path.split_first()) {
            (Self::Widget(_), None) => Some(()),
            (Self::Scene(scene), _) if !scene.has_layers() => scene
                .base
                .collect_source_metadata_path(child_path, metadata),
            (Self::Scene(scene), Some((child_index, remaining_path))) => {
                if *child_index == 0 {
                    scene
                        .base
                        .collect_source_metadata_path(remaining_path, metadata)
                } else {
                    let (layer_index, child_kind) =
                        scene.ordered_layer_child_for_child(*child_index - 1)?;
                    match child_kind {
                        SurfaceLayerChildKind::Input => scene.layers[layer_index]
                            .input
                            .as_ref()?
                            .collect_source_metadata_path(remaining_path, metadata),
                        SurfaceLayerChildKind::Foreground => scene.layers[layer_index]
                            .node
                            .collect_source_metadata_path(remaining_path, metadata),
                    }
                }
            }
            (Self::Container(container), Some((child_index, remaining_path))) => container
                .children
                .get(*child_index)?
                .child
                .collect_source_metadata_path(remaining_path, metadata),
            (Self::FloatingLayer(layer), Some((child_index, remaining_path)))
                if layer.interactive =>
            {
                layer
                    .container
                    .children
                    .get(*child_index)?
                    .child
                    .collect_source_metadata_path(remaining_path, metadata)
            }
            _ => None,
        }
    }

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
    ) -> Result<PreparedWidgetStateSyncWitness, PreparedWidgetStateSyncVeto> {
        let mut entries = Vec::with_capacity(evidence.stateful_widget_order.len());
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
            if !previous_widget.supports_prepared_state_synchronization()
                || !current_widget.supports_prepared_state_synchronization()
            {
                return Err(PreparedWidgetStateSyncVeto::Unsupported);
            }
            let previous_cached = previous_widget.revision_evidence();
            let current_cached = current_widget.revision_evidence();
            let previous_live_revision = previous_widget.live_revision();
            let current_live_revision = current_widget.live_revision();
            if !previous_cached.valid
                || !current_cached.valid
                || !previous_widget.cached_revision_is_exact()
                || !current_widget.cached_revision_is_exact()
                || previous_cached.revision != previous_live_revision
                || current_cached.revision != current_live_revision
                || previous_cached.capabilities != previous_widget.live_capability_evidence()
                || current_cached.capabilities != current_widget.live_capability_evidence()
                || previous_cached.id != previous_widget.id()
                || current_cached.id != current_widget.id()
            {
                return Err(PreparedWidgetStateSyncVeto::InvalidRevision);
            }
            let previous_sources = previous
                .source_metadata_path_at(previous_path.as_slice())
                .ok_or(PreparedWidgetStateSyncVeto::InvalidPath)?;
            let current_sources = self
                .source_metadata_path_at(current_path.as_slice())
                .ok_or(PreparedWidgetStateSyncVeto::InvalidPath)?;
            let witness = PreparedWidgetStateSyncLeafWitness::capture(
                *widget_id,
                previous_path.clone(),
                current_path.clone(),
                previous_widget,
                current_widget,
                previous_sources,
                current_sources,
            );
            let previous_sources = previous
                .source_metadata_path_at(previous_path.as_slice())
                .ok_or(PreparedWidgetStateSyncVeto::InvalidPath)?;
            if !witness.previous_is_unchanged(previous_widget, &previous_sources) {
                return Err(PreparedWidgetStateSyncVeto::InvalidRevision);
            }
            entries.push(witness);
        }
        Ok(PreparedWidgetStateSyncWitness { entries })
    }

    pub(super) fn synchronize_prepared_widget_state(
        &mut self,
        evidence: &PreparedWidgetStateSyncEvidence<'_>,
        witness: &PreparedWidgetStateSyncWitness,
        previous: &Self,
    ) -> Result<(), PreparedWidgetStateSyncVeto> {
        if witness.entries.len() != evidence.stateful_widget_order.len() {
            return Err(PreparedWidgetStateSyncVeto::InvalidPath);
        }
        for entry in &witness.entries {
            let widget_id = entry.widget_id;
            let previous_path = &entry.previous_path;
            let current_path = &entry.current_path;
            let previous_widget = previous
                .find_widget_at_path(previous_path.as_slice())
                .filter(|widget| widget.id() == widget_id)
                .ok_or(PreparedWidgetStateSyncVeto::InvalidIdentity)?;
            let current_widget = self
                .find_widget_mut_at_path(current_path.as_slice())
                .filter(|widget| widget.id() == widget_id)
                .ok_or(PreparedWidgetStateSyncVeto::InvalidIdentity)?;

            current_widget
                .widget_object_mut_runtime()
                .synchronize_from_previous(previous_widget.widget_object());
            if evidence.policy.clears_retained_hover_for(widget_id) {
                current_widget
                    .widget_object_mut_runtime()
                    .common_mut()
                    .state
                    .hovered = false;
            }
        }
        Ok(())
    }

    pub(super) fn handle_input_with_environment(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
        environment: &crate::runtime::ResolvedEnvironment,
    ) -> Option<WidgetOutput> {
        self.find_widget_mut(widget_id).and_then(|widget| {
            widget.handle_input_with_environment(widget_id, bounds, input, environment)
        })
    }

    #[cfg(test)]
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

    pub(super) fn dispatch_pointer_event_at_path(
        &mut self,
        widget_id: WidgetId,
        child_path: &[usize],
        bounds: Rect,
        event: crate::gui::pointer_ingress::PointerEvent,
    ) -> Option<WidgetDispatchResult<Message>> {
        self.find_widget_mut_at_path(child_path)
            .filter(|widget| widget.id() == widget_id)
            .map(|widget| widget.dispatch_pointer_event(widget_id, bounds, event))
    }

    pub(super) fn dispatch_input_at_path_with_environment(
        &mut self,
        widget_id: WidgetId,
        child_path: &[usize],
        bounds: Rect,
        input: WidgetInput,
        environment: &crate::runtime::ResolvedEnvironment,
    ) -> Option<WidgetDispatchResult<Message>> {
        self.find_widget_mut_at_path(child_path)
            .filter(|widget| widget.id() == widget_id)
            .map(|widget| {
                widget.dispatch_input_with_environment(widget_id, bounds, input, environment)
            })
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

    pub(in crate::runtime) fn find_container_at_path(
        &self,
        child_path: &[usize],
    ) -> Option<&super::SurfaceContainer<Message>> {
        match (self, child_path.split_first()) {
            (Self::Container(container), None) => Some(container),
            (Self::FloatingLayer(layer), None) if layer.interactive => Some(&layer.container),
            (Self::Scene(scene), path) => {
                if !scene.has_layers() {
                    return scene.base.find_container_at_path(child_path);
                }

                let (child_index, remaining_path) = path?;
                if *child_index == 0 {
                    return scene.base.find_container_at_path(remaining_path);
                }

                let (layer_index, child_kind) =
                    scene.ordered_layer_child_for_child(*child_index - 1)?;
                match child_kind {
                    SurfaceLayerChildKind::Input => scene.layers[layer_index]
                        .input
                        .as_ref()?
                        .find_container_at_path(remaining_path),
                    SurfaceLayerChildKind::Foreground => scene.layers[layer_index]
                        .node
                        .find_container_at_path(remaining_path),
                }
            }
            (Self::Container(container), Some((child_index, remaining_path))) => container
                .children
                .get(*child_index)?
                .child
                .find_container_at_path(remaining_path),
            (Self::FloatingLayer(layer), Some((child_index, remaining_path)))
                if layer.interactive =>
            {
                layer
                    .container
                    .children
                    .get(*child_index)?
                    .child
                    .find_container_at_path(remaining_path)
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
