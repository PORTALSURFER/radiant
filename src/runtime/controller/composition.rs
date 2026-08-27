//! Focus-pinned routing for backend-neutral composition samples.

use super::SurfaceRuntime;
use super::interaction_state::RuntimeManagedCompositionState;
use crate::gui::input::InputTimestamp;
use crate::runtime::{RuntimeBridge, WidgetDispatchResult};
use crate::widgets::{CompositionPhase, CompositionSample, WidgetId};

#[cfg(test)]
#[path = "composition/tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompositionWidgetDispatch {
    Handled { retained: bool },
    RetainedNoOutput,
    Unhandled,
}

impl CompositionWidgetDispatch {
    fn was_routed(self) -> bool {
        !matches!(self, Self::Unhandled)
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Route one composition sample through the currently focused opting-in
    /// widget.  A `Start` establishes one fixed owner; later samples never
    /// perform hit testing or rebind to a different widget.
    pub fn dispatch_composition_sample(&mut self, sample: CompositionSample) -> Option<WidgetId> {
        if !self.lifecycle.accepts_work() {
            return None;
        }
        if !sample.is_valid() {
            let active_widget_id = match self.interaction.composition.managed_composition {
                RuntimeManagedCompositionState::Active { widget_id }
                    if self.managed_composition_is_live(widget_id) =>
                {
                    Some(widget_id)
                }
                _ => None,
            };
            if let Some(widget_id) = active_widget_id {
                // Invalid evidence ends a live owner through the same
                // terminal clear-before-dispatch path as an explicit cancel.
                // Install the stale-continuation fence after that one
                // owner-directed synthetic cancel has been delivered.
                self.dispatch_managed_composition(
                    widget_id,
                    CompositionPhase::Cancel,
                    CompositionSample::cancel(),
                );
            }
            self.block_managed_composition();
            return None;
        }

        match (
            self.interaction.composition.managed_composition,
            sample.phase(),
        ) {
            (RuntimeManagedCompositionState::Blocked, CompositionPhase::Start) => {
                self.clear_managed_composition();
                self.dispatch_composition_start(sample)
            }
            (RuntimeManagedCompositionState::Blocked, CompositionPhase::Update) => None,
            (
                RuntimeManagedCompositionState::Blocked,
                CompositionPhase::Commit | CompositionPhase::Cancel,
            ) => {
                self.clear_managed_composition();
                None
            }
            (RuntimeManagedCompositionState::Active { .. }, CompositionPhase::Start) => None,
            (RuntimeManagedCompositionState::Active { widget_id }, phase) => {
                if !self.managed_composition_is_live(widget_id) {
                    self.block_managed_composition();
                    return None;
                }
                self.dispatch_managed_composition(widget_id, phase, sample)
            }
            (RuntimeManagedCompositionState::Idle, CompositionPhase::Start) => {
                self.dispatch_composition_start(sample)
            }
            (RuntimeManagedCompositionState::Idle, _) => {
                self.block_managed_composition();
                None
            }
        }
    }

    /// Alias that makes the focused routing boundary explicit for native
    /// adapters without creating a second ownership path.
    pub fn dispatch_focused_composition_sample(
        &mut self,
        sample: CompositionSample,
    ) -> Option<WidgetId> {
        self.dispatch_composition_sample(sample)
    }

    /// Route a native preedit update with explicitly hidden selection through
    /// the existing fixed composition owner.  This is crate-private so the
    /// public sample vocabulary remains limited to its four lifecycle variants.
    pub(crate) fn dispatch_hidden_composition_update(
        &mut self,
        preedit: String,
        timestamp: Option<InputTimestamp>,
    ) -> Option<WidgetId> {
        if !self.lifecycle.accepts_work() {
            return None;
        }
        let RuntimeManagedCompositionState::Active { widget_id } =
            self.interaction.composition.managed_composition
        else {
            self.block_managed_composition();
            return None;
        };
        if !self.managed_composition_is_live(widget_id) {
            self.block_managed_composition();
            return None;
        }
        self.dispatch_managed_hidden_composition(widget_id, preedit, timestamp)
    }

    pub(crate) fn managed_composition_is_active(&self) -> bool {
        matches!(
            self.interaction.composition.managed_composition,
            RuntimeManagedCompositionState::Active { .. }
        )
    }

    fn dispatch_composition_start(&mut self, sample: CompositionSample) -> Option<WidgetId> {
        let widget_id = self.interaction.focus.focused_widget()?;
        if !self.composition_widget_is_unique(widget_id)
            || !self.is_authoritative_focus_target(widget_id)
            || !self.composition_widget_is_admitting(widget_id)
        {
            self.block_managed_composition();
            return None;
        }

        // Install ownership before widget dispatch.  A mapped output may
        // synchronously reproject or attempt another sample; that reentrant
        // work must observe the incumbent rather than create a second owner.
        self.interaction.composition.managed_composition =
            RuntimeManagedCompositionState::Active { widget_id };
        let dispatch = self.dispatch_composition_to_widget(widget_id, sample);
        let Some(dispatch) = dispatch else {
            self.block_managed_composition();
            return None;
        };
        if !dispatch.was_routed() || !self.composition_owner_still_retained(widget_id) {
            self.block_managed_composition();
        }
        dispatch.was_routed().then_some(widget_id)
    }

    fn dispatch_managed_composition(
        &mut self,
        widget_id: WidgetId,
        phase: CompositionPhase,
        sample: CompositionSample,
    ) -> Option<WidgetId> {
        let terminal = matches!(phase, CompositionPhase::Commit | CompositionPhase::Cancel);
        if terminal {
            // A terminal must be invisible to reentrant refresh, focus, or
            // dispatch code before the owner receives it.
            self.clear_managed_composition();
        }
        let dispatch = self.dispatch_composition_to_widget(widget_id, sample)?;
        if !terminal
            && matches!(
                self.interaction.composition.managed_composition,
                RuntimeManagedCompositionState::Active {
                    widget_id: active_widget_id
                } if active_widget_id == widget_id
            )
            && !self.composition_owner_still_retained(widget_id)
        {
            self.block_managed_composition();
        }
        dispatch.was_routed().then_some(widget_id)
    }

    fn dispatch_managed_hidden_composition(
        &mut self,
        widget_id: WidgetId,
        preedit: String,
        timestamp: Option<InputTimestamp>,
    ) -> Option<WidgetId> {
        let dispatch = self.dispatch_hidden_composition_to_widget(widget_id, preedit, timestamp)?;
        if matches!(
            self.interaction.composition.managed_composition,
            RuntimeManagedCompositionState::Active {
                widget_id: active_widget_id
            } if active_widget_id == widget_id
        ) && !self.composition_owner_still_retained(widget_id)
        {
            self.block_managed_composition();
        }
        dispatch.was_routed().then_some(widget_id)
    }

    fn dispatch_composition_to_widget(
        &mut self,
        widget_id: WidgetId,
        sample: CompositionSample,
    ) -> Option<CompositionWidgetDispatch> {
        #[cfg(test)]
        {
            let managed_composition = self.interaction.composition.managed_composition;
            self.interaction
                .composition_dispatch_observations
                .push((sample.phase(), managed_composition));
        }
        let result = self.dispatch_surface_composition_sample(widget_id, sample)?;
        let retained = result.1;
        let dispatch = match result.0 {
            WidgetDispatchResult::Message(message) => {
                let outcome = self.dispatch_message(message);
                self.pending_input_command_outcome.merge(outcome);
                CompositionWidgetDispatch::Handled { retained }
            }
            WidgetDispatchResult::UnmappedOutput => {
                self.relayout();
                CompositionWidgetDispatch::Handled { retained }
            }
            WidgetDispatchResult::NoOutput if retained => {
                CompositionWidgetDispatch::RetainedNoOutput
            }
            WidgetDispatchResult::NoOutput => CompositionWidgetDispatch::Unhandled,
        };
        Some(dispatch)
    }

    fn dispatch_hidden_composition_to_widget(
        &mut self,
        widget_id: WidgetId,
        preedit: String,
        timestamp: Option<InputTimestamp>,
    ) -> Option<CompositionWidgetDispatch> {
        let result =
            self.dispatch_surface_hidden_composition_update(widget_id, preedit, timestamp)?;
        let retained = result.1;
        let dispatch = match result.0 {
            WidgetDispatchResult::Message(message) => {
                let outcome = self.dispatch_message(message);
                self.pending_input_command_outcome.merge(outcome);
                CompositionWidgetDispatch::Handled { retained }
            }
            WidgetDispatchResult::UnmappedOutput => {
                self.relayout();
                CompositionWidgetDispatch::Handled { retained }
            }
            WidgetDispatchResult::NoOutput if retained => {
                CompositionWidgetDispatch::RetainedNoOutput
            }
            WidgetDispatchResult::NoOutput => CompositionWidgetDispatch::Unhandled,
        };
        Some(dispatch)
    }

    fn composition_widget_is_unique(&self, widget_id: WidgetId) -> bool {
        let mut found = false;
        for candidate in &self.traversal.widgets.hit_order {
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

    fn composition_widget_is_admitting(&self, widget_id: WidgetId) -> bool {
        self.surface_widget(widget_id).is_some_and(|widget| {
            let common = widget.widget_object().common();
            widget.id() == widget_id
                && widget.is_focusable()
                && !common.state.disabled
                && !common.state.read_only
                && widget.accepts_composition_input()
        })
    }

    fn composition_owner_still_retained(&self, widget_id: WidgetId) -> bool {
        self.managed_composition_is_live(widget_id)
    }

    fn managed_composition_is_live(&self, widget_id: WidgetId) -> bool {
        self.composition_widget_is_unique(widget_id)
            && self.is_authoritative_focus_target(widget_id)
            && self.surface_widget(widget_id).is_some_and(|widget| {
                let common = widget.widget_object().common();
                widget.id() == widget_id
                    && !common.state.disabled
                    && !common.state.read_only
                    && widget.accepts_composition_input()
                    && widget.retains_managed_composition()
            })
    }

    pub(in crate::runtime::controller) fn validate_managed_composition_authority(
        &mut self,
    ) -> bool {
        match self.interaction.composition.managed_composition {
            RuntimeManagedCompositionState::Idle => true,
            RuntimeManagedCompositionState::Blocked => false,
            RuntimeManagedCompositionState::Active { widget_id }
                if self.managed_composition_is_live(widget_id) =>
            {
                true
            }
            RuntimeManagedCompositionState::Active { .. } => {
                self.block_managed_composition();
                false
            }
        }
    }

    pub(in crate::runtime::controller) fn clear_managed_composition_for_widget(
        &mut self,
        widget_id: WidgetId,
    ) {
        if matches!(
            self.interaction.composition.managed_composition,
            RuntimeManagedCompositionState::Active {
                widget_id: active_widget_id
            } if active_widget_id == widget_id
        ) {
            self.block_managed_composition();
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::runtime::controller) fn reconcile_managed_composition_after_refresh(
        &mut self,
        next_surface: &crate::runtime::UiSurface<Message>,
        previous_widget_order: &[WidgetId],
        current_widget_order: &[WidgetId],
        previous_paths: &std::collections::HashMap<WidgetId, crate::runtime::WidgetPath>,
        current_paths: &std::collections::HashMap<WidgetId, crate::runtime::WidgetPath>,
        retired_widget_ids: &[WidgetId],
        focused_widget_before_refresh: Option<WidgetId>,
    ) {
        let RuntimeManagedCompositionState::Active { widget_id } =
            self.interaction.composition.managed_composition
        else {
            return;
        };

        let Some(previous_path) = previous_paths.get(&widget_id) else {
            self.block_managed_composition();
            return;
        };
        let Some(current_path) = current_paths.get(&widget_id) else {
            self.block_managed_composition();
            return;
        };
        let Some((previous_kind, previous_valid)) = self
            .surface
            .widget_compatibility_at_path(previous_path.as_slice())
        else {
            self.block_managed_composition();
            return;
        };
        let Some((current_kind, current_valid)) =
            next_surface.widget_compatibility_at_path(current_path.as_slice())
        else {
            self.block_managed_composition();
            return;
        };
        let previous_widget = self.surface.find_widget_at_path(widget_id, previous_path);
        let current_widget = next_surface.find_widget_at_path(widget_id, current_path);
        let exact_compatible = !retired_widget_ids.contains(&widget_id)
            && has_unique_widget_id(previous_widget_order, widget_id)
            && has_unique_widget_id(current_widget_order, widget_id)
            && previous_path == current_path
            && previous_valid
            && current_valid
            && previous_kind == current_kind
            && focused_widget_before_refresh == Some(widget_id)
            && self.interaction.focus.focused_widget() == Some(widget_id)
            && previous_widget.is_some_and(|widget| {
                Self::managed_refresh_composition_widget_is_live(
                    widget,
                    widget_id,
                    focused_widget_before_refresh,
                )
            })
            && current_widget.is_some_and(|widget| {
                Self::managed_refresh_composition_widget_is_live(
                    widget,
                    widget_id,
                    focused_widget_before_refresh,
                )
            });
        if !exact_compatible {
            self.block_managed_composition();
        }
    }

    fn managed_refresh_composition_widget_is_live(
        widget: &crate::runtime::SurfaceWidget<Message>,
        widget_id: WidgetId,
        focused_widget: Option<WidgetId>,
    ) -> bool {
        let common = widget.widget_object().common();
        widget.id() == widget_id
            && focused_widget == Some(widget_id)
            && widget.is_focusable()
            && !common.state.disabled
            && !common.state.read_only
            && widget.accepts_composition_input()
            && widget.retains_managed_composition()
    }

    pub(in crate::runtime::controller) fn block_managed_composition(&mut self) {
        self.interaction.composition.managed_composition = RuntimeManagedCompositionState::Blocked;
    }

    fn clear_managed_composition(&mut self) {
        self.interaction.composition.managed_composition = RuntimeManagedCompositionState::Idle;
    }
}

fn has_unique_widget_id(order: &[WidgetId], widget_id: WidgetId) -> bool {
    let mut found = false;
    for candidate in order {
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
