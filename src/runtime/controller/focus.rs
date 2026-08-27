use std::collections::HashMap;

use super::interaction_state::{
    RuntimeFocusOwner, RuntimeFocusedKeyCapture, RuntimeSplitPaneSeparatorFocusOwner,
};
use super::split_pane_separator::SplitPaneSeparatorProjection;
use super::{FocusTraversal, SurfaceRuntime};
use crate::widgets::interaction::CompositionStartContext;
use crate::widgets::{FocusLossDecision, KeyboardModifiers};
use crate::{
    gui::input::InputTimestamp,
    gui::{focus::FocusSurface, input::KeyPress},
    runtime::RuntimeBridge,
    widgets::{WidgetId, WidgetInput, WidgetKey},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FocusTransition {
    InvalidTarget,
    Vetoed,
    Unchanged,
    Changed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SplitPaneSeparatorFocusAdmission {
    NotAcquirable,
    Vetoed,
    Invalidated,
    Admitted,
}

/// Result of one metadata-aware focused-key decision.
///
/// The type stays crate-private so the public API exposes only the existing
/// `Event`, `WidgetInput`, and `WidgetKey` vocabulary. `None` from the routing
/// helper means the focused widget did not opt in and the caller should retain
/// its legacy compatibility path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FocusedKeyDispatch {
    pub(crate) widget_id: Option<WidgetId>,
    pub(crate) routed: bool,
}

#[derive(Clone, Copy)]
enum FocusedKeySample {
    Press { repeat: bool },
    Release,
}

impl FocusedKeySample {
    fn is_initial_press(self) -> bool {
        matches!(self, Self::Press { repeat: false })
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Give keyboard focus to one focusable widget.
    ///
    /// Returns `false` when the widget is absent or does not participate in
    /// focus. A valid target may retain the current owner when that widget
    /// vetoes focus loss. Focus changes are routed into affected widgets so
    /// their retained interaction state can update before the next paint plan.
    pub fn focus_widget(&mut self, widget_id: WidgetId) -> bool {
        !matches!(
            self.request_focus(widget_id),
            FocusTransition::InvalidTarget
        )
    }

    /// Clear keyboard focus when a surface or backend loses focus ownership.
    pub fn clear_focus(&mut self) {
        let _ = self.clear_focus_with_transition();
    }

    pub(super) fn clear_focus_with_transition(&mut self) -> FocusTransition {
        let Some(previous_owner) = self.interaction.focus.owner else {
            return FocusTransition::Unchanged;
        };
        let previous_widget = previous_owner.widget_id();
        let previous_is_live =
            previous_widget.is_some_and(|widget_id| self.focus_owner_can_prepare_loss(widget_id));
        if let Some(previous_widget) = previous_widget
            && previous_is_live
            && self.prepare_focus_loss(previous_widget) == FocusLossDecision::Veto
        {
            self.repaint_requested = true;
            return FocusTransition::Vetoed;
        }
        if let Some(previous_widget) = previous_widget {
            self.clear_managed_wheel_sequence_for_widget(previous_widget);
            self.clear_managed_composition_for_widget(previous_widget);
            self.terminate_managed_pointer_capture_for_widget(previous_widget);
            self.mark_focused_key_capture_stale(previous_widget);
        }
        self.interaction.focus.owner = None;
        if let Some(previous_widget) = previous_widget
            && previous_is_live
        {
            self.route_focus_changed(previous_widget, false);
        }
        FocusTransition::Changed
    }

    pub(super) fn request_focus(&mut self, widget_id: WidgetId) -> FocusTransition {
        if !self.is_live_focus_target(widget_id) {
            return FocusTransition::InvalidTarget;
        }
        self.request_focus_owner(RuntimeFocusOwner::Widget(widget_id))
    }

    /// Admit one exact separator projection at the committed layout boundary.
    pub(super) fn request_split_pane_separator_focus(
        &mut self,
        projection: SplitPaneSeparatorProjection,
    ) -> FocusTransition {
        let Some(current) = self.current_split_pane_separator_projection(projection.target) else {
            return FocusTransition::InvalidTarget;
        };
        if current != projection {
            return FocusTransition::InvalidTarget;
        }

        let next = Self::split_pane_separator_focus_owner(current);
        if let Some(existing) = self.interaction.focus.owner {
            match existing {
                RuntimeFocusOwner::SplitPaneSeparator(existing)
                    if Self::separator_owner_identity_matches(existing, next) =>
                {
                    if existing.behavior.compatible_with(current.behavior) {
                        if existing.behavior != current.behavior {
                            self.interaction.focus.owner = Some(next);
                        }
                        return FocusTransition::Unchanged;
                    }
                    self.interaction.focus.owner = None;
                    return FocusTransition::InvalidTarget;
                }
                RuntimeFocusOwner::SplitPaneSeparator(_) => {
                    // A private separator owner may transfer directly to a
                    // different exact separator without routing widget loss.
                }
                RuntimeFocusOwner::Widget(_) => {}
            }
        }
        self.request_focus_owner(next)
    }

    pub(super) fn request_split_pane_separator_focus_at(
        &mut self,
        position: crate::gui::types::Point,
    ) -> SplitPaneSeparatorFocusAdmission {
        if !position.is_finite() {
            return SplitPaneSeparatorFocusAdmission::NotAcquirable;
        }
        let Some((target, target_bounds)) =
            self.layout_input_target_identity_and_bounds_at(position)
        else {
            return SplitPaneSeparatorFocusAdmission::NotAcquirable;
        };
        let Some(projection) = self.current_split_pane_separator_projection(target) else {
            return SplitPaneSeparatorFocusAdmission::NotAcquirable;
        };
        if target_bounds != projection.divider_bounds
            || !self.split_pane_separator_pointer_evidence_is_current(position, projection)
        {
            return SplitPaneSeparatorFocusAdmission::NotAcquirable;
        }

        let proposed_owner = Self::split_pane_separator_focus_owner(projection);
        match self.request_split_pane_separator_focus(projection) {
            FocusTransition::Vetoed => SplitPaneSeparatorFocusAdmission::Vetoed,
            FocusTransition::InvalidTarget => SplitPaneSeparatorFocusAdmission::Invalidated,
            FocusTransition::Unchanged | FocusTransition::Changed
                if self.split_pane_separator_pointer_evidence_is_current(position, projection) =>
            {
                SplitPaneSeparatorFocusAdmission::Admitted
            }
            FocusTransition::Unchanged | FocusTransition::Changed => {
                if let (
                    Some(RuntimeFocusOwner::SplitPaneSeparator(current_owner)),
                    RuntimeFocusOwner::SplitPaneSeparator(proposed_owner),
                ) = (self.interaction.focus.owner, proposed_owner)
                    && Self::separator_owner_is_behavior_refreshed_descendant(
                        current_owner,
                        proposed_owner,
                    )
                {
                    self.interaction.focus.owner = None;
                }
                SplitPaneSeparatorFocusAdmission::Invalidated
            }
        }
    }

    fn request_focus_owner(&mut self, next: RuntimeFocusOwner) -> FocusTransition {
        if self.interaction.focus.owner == Some(next) {
            return FocusTransition::Unchanged;
        }

        let previous_owner = self.interaction.focus.owner;
        let previous_widget = previous_owner.and_then(RuntimeFocusOwner::widget_id);
        let previous_is_live =
            previous_widget.is_some_and(|widget_id| self.focus_owner_can_prepare_loss(widget_id));
        if let Some(previous_widget) = previous_widget
            && previous_is_live
            && self.prepare_focus_loss(previous_widget) == FocusLossDecision::Veto
        {
            self.repaint_requested = true;
            return FocusTransition::Vetoed;
        }

        if let Some(previous_widget) = previous_widget {
            self.clear_managed_wheel_sequence_for_widget(previous_widget);
            self.clear_managed_composition_for_widget(previous_widget);
            self.terminate_managed_pointer_capture_for_widget(previous_widget);
            // Install the controller-owned target before FocusChanged(false)
            // can emit a message and synchronously reproject the surface.
            self.mark_focused_key_capture_stale(previous_widget);
        }
        self.interaction.focus.owner = Some(next);
        let application_projection_before = self.refresh_counters().application_projection;
        if let Some(previous_widget) = previous_widget
            && previous_is_live
        {
            self.route_focus_changed(previous_widget, false);
        }
        let reprojected =
            self.refresh_counters().application_projection != application_projection_before;
        if let RuntimeFocusOwner::Widget(widget_id) = next
            && !reprojected
            && self.is_authoritative_focus_target(widget_id)
        {
            self.route_focus_changed(widget_id, true);
        }

        match next {
            RuntimeFocusOwner::Widget(widget_id) => {
                if self.interaction.focus.owner == Some(next)
                    && !self.is_authoritative_focus_target(widget_id)
                {
                    // A focus-loss output may remove or supersede the proposed
                    // target while the old owner is being routed out.
                    self.interaction.focus.owner = None;
                    return FocusTransition::InvalidTarget;
                }
                if self.interaction.focus.owner == Some(next) {
                    FocusTransition::Changed
                } else {
                    FocusTransition::InvalidTarget
                }
            }
            RuntimeFocusOwner::SplitPaneSeparator(owner) => {
                if self.separator_focus_owner_is_current(owner) {
                    FocusTransition::Changed
                } else {
                    if self
                        .interaction
                        .focus
                        .owner
                        .is_some_and(|current| match current {
                            RuntimeFocusOwner::SplitPaneSeparator(current) => {
                                Self::separator_owner_identity_matches(current, next)
                            }
                            RuntimeFocusOwner::Widget(_) => false,
                        })
                    {
                        self.interaction.focus.owner = None;
                    }
                    FocusTransition::InvalidTarget
                }
            }
        }
    }

    pub(super) fn is_live_focus_target(&self, widget_id: WidgetId) -> bool {
        self.traversal
            .widgets
            .paths
            .current
            .contains_key(&widget_id)
            && self.traversal.widgets.focusable.contains(widget_id)
            && self.layout.rects.contains_key(&widget_id)
            && self
                .surface_widget(widget_id)
                .is_some_and(|widget| widget.is_focusable())
    }

    pub(super) fn is_authoritative_focus_target(&self, widget_id: WidgetId) -> bool {
        self.interaction.focus.focused_widget() == Some(widget_id)
            && self.is_live_focus_target(widget_id)
    }

    fn current_split_pane_separator_projection(
        &self,
        target: crate::layout::LayoutTargetIdentity,
    ) -> Option<SplitPaneSeparatorProjection> {
        let mut current = None;
        for projection in &self.traversal.containers.split_pane_separator_projections {
            if projection.target != target {
                continue;
            }
            if current.replace(*projection).is_some() {
                return None;
            }
        }
        current
    }

    fn split_pane_separator_pointer_evidence_is_current(
        &self,
        position: crate::gui::types::Point,
        projection: SplitPaneSeparatorProjection,
    ) -> bool {
        let Some((target, target_bounds)) =
            self.layout_input_target_identity_and_bounds_at(position)
        else {
            return false;
        };
        target == projection.target
            && target_bounds == projection.divider_bounds
            && projection.divider_bounds.has_finite_positive_area()
            && projection.divider_bounds.contains(position)
            && projection.live_ratio.is_finite()
            && (0.0..=1.0).contains(&projection.live_ratio)
            && self.current_split_pane_separator_projection(projection.target) == Some(projection)
    }

    fn split_pane_separator_focus_owner(
        projection: SplitPaneSeparatorProjection,
    ) -> RuntimeFocusOwner {
        RuntimeFocusOwner::SplitPaneSeparator(RuntimeSplitPaneSeparatorFocusOwner {
            target: projection.target,
            mounted_state_id: projection.mounted_state_id,
            axis: projection.axis,
            behavior: projection.behavior,
        })
    }

    fn separator_owner_identity_matches(
        owner: RuntimeSplitPaneSeparatorFocusOwner,
        candidate: RuntimeFocusOwner,
    ) -> bool {
        let RuntimeFocusOwner::SplitPaneSeparator(candidate) = candidate else {
            return false;
        };
        owner.target == candidate.target
            && owner.mounted_state_id == candidate.mounted_state_id
            && owner.axis == candidate.axis
    }

    fn separator_owner_is_behavior_refreshed_descendant(
        owner: RuntimeSplitPaneSeparatorFocusOwner,
        candidate: RuntimeSplitPaneSeparatorFocusOwner,
    ) -> bool {
        owner.target == candidate.target
            && owner.mounted_state_id == candidate.mounted_state_id
            && owner.axis == candidate.axis
            && owner.behavior.compatible_with(candidate.behavior)
    }

    fn separator_focus_owner_is_current(&self, owner: RuntimeSplitPaneSeparatorFocusOwner) -> bool {
        self.current_split_pane_separator_projection(owner.target)
            .is_some_and(|projection| {
                owner.mounted_state_id == projection.mounted_state_id
                    && owner.axis == projection.axis
                    && owner.behavior.compatible_with(projection.behavior)
            })
    }

    pub(super) fn revalidate_focus_owner(&mut self) {
        let Some(owner) = self.interaction.focus.owner else {
            return;
        };
        match owner {
            RuntimeFocusOwner::Widget(widget_id) => {
                if !self.traversal.widgets.focusable.contains(widget_id) {
                    self.interaction.focus.owner = None;
                }
            }
            RuntimeFocusOwner::SplitPaneSeparator(owner) => {
                let Some(projection) = self.current_split_pane_separator_projection(owner.target)
                else {
                    self.interaction.focus.owner = None;
                    return;
                };
                if owner.mounted_state_id != projection.mounted_state_id
                    || owner.axis != projection.axis
                    || !owner.behavior.compatible_with(projection.behavior)
                {
                    self.interaction.focus.owner = None;
                } else if owner.behavior != projection.behavior {
                    self.interaction.focus.owner = Some(RuntimeFocusOwner::SplitPaneSeparator(
                        RuntimeSplitPaneSeparatorFocusOwner {
                            behavior: projection.behavior,
                            ..owner
                        },
                    ));
                }
            }
        }
    }

    pub(super) fn clear_separator_focus_owner(&mut self) -> bool {
        if matches!(
            self.interaction.focus.owner,
            Some(RuntimeFocusOwner::SplitPaneSeparator(_))
        ) {
            self.interaction.focus.owner = None;
            true
        } else {
            false
        }
    }

    fn focus_owner_can_prepare_loss(&self, widget_id: WidgetId) -> bool {
        self.is_live_focus_target(widget_id)
    }

    fn prepare_focus_loss(&mut self, widget_id: WidgetId) -> FocusLossDecision {
        let Some(child_path) = self.traversal.widgets.paths.current.get(&widget_id) else {
            return FocusLossDecision::Allow;
        };
        self.surface
            .find_widget_mut_at_path(widget_id, child_path)
            .map(|widget| widget.widget_object_mut_runtime().prepare_focus_loss())
            .unwrap_or(FocusLossDecision::Allow)
    }

    /// Move keyboard focus through the current declarative tree.
    ///
    /// Traversal uses stable tree order and wraps at either end. Returns the new
    /// focus target, or `None` when no keyboard-focusable widgets are projected.
    pub fn traverse_focus(&mut self, direction: FocusTraversal) -> Option<WidgetId> {
        let next = next_focus_target(
            self.interaction.focus.focused_widget(),
            self.traversal.widgets.keyboard_focus.order(),
            self.traversal.widgets.keyboard_focus.rank(),
            direction,
        )?;
        match self.request_focus(next) {
            FocusTransition::Changed | FocusTransition::Unchanged => Some(next),
            FocusTransition::InvalidTarget | FocusTransition::Vetoed => None,
        }
    }

    /// Route a keyboard interaction to the current focus target.
    ///
    /// Pointer events should continue to use [`SurfaceRuntime::dispatch_input_at`]
    /// or [`SurfaceRuntime::dispatch_input`], because they carry their own hit
    /// target. Keyboard events are resolved through focused widget identity.
    pub fn dispatch_focused_input(&mut self, input: WidgetInput) -> Option<WidgetId> {
        let widget_id = self.interaction.focus.focused_widget()?;
        self.dispatch_input(widget_id, input).then_some(widget_id)
    }

    /// Route one metadata-aware focused key press through the generic runtime.
    ///
    /// `Some` means the focused widget opted into the metadata-aware contract,
    /// or a prior capture/stale record claimed the sample. `None` deliberately
    /// leaves the caller on the existing key-only compatibility path.
    pub(crate) fn dispatch_metadata_focused_key_press(
        &mut self,
        host_press: Option<KeyPress>,
        widget_key: Option<WidgetKey>,
        modifiers: KeyboardModifiers,
        timestamp: Option<InputTimestamp>,
        repeat: bool,
        focus: FocusSurface,
    ) -> Option<FocusedKeyDispatch> {
        self.dispatch_metadata_focused_key_sample(
            host_press,
            widget_key,
            modifiers,
            timestamp,
            FocusedKeySample::Press { repeat },
            focus,
        )
    }

    /// Route one metadata-aware focused key release through the generic runtime.
    pub(crate) fn dispatch_metadata_focused_key_release(
        &mut self,
        widget_key: Option<WidgetKey>,
        modifiers: KeyboardModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Option<FocusedKeyDispatch> {
        self.dispatch_metadata_focused_key_sample(
            None,
            widget_key,
            modifiers,
            timestamp,
            FocusedKeySample::Release,
            FocusSurface::None,
        )
    }

    fn dispatch_metadata_focused_key_sample(
        &mut self,
        host_press: Option<KeyPress>,
        widget_key: Option<WidgetKey>,
        modifiers: KeyboardModifiers,
        timestamp: Option<InputTimestamp>,
        sample: FocusedKeySample,
        focus: FocusSurface,
    ) -> Option<FocusedKeyDispatch> {
        if let Some(capture) = self.interaction.focus.focused_key_capture {
            if capture.stale {
                // The first sample after authority loss is itself ambiguous:
                // without a generation/token contract it may be a stale
                // continuation or a new press for a successor. Consume it as
                // stale evidence and let the following boundary establish a
                // new initial sequence.
                self.interaction.focus.focused_key_capture = None;
                return Some(FocusedKeyDispatch::default());
            } else {
                return Some(self.dispatch_captured_focused_key(
                    capture, widget_key, modifiers, timestamp, sample,
                ));
            }
        }

        let widget_id = self.interaction.focus.focused_widget()?;
        if !self.is_authoritative_focus_target(widget_id)
            || !self.focused_key_widget_has_authority(widget_id)
        {
            return self
                .surface_widget(widget_id)
                .filter(|widget| widget.widget_object().participates_in_focused_key_routing())
                .map(|_| FocusedKeyDispatch::default());
        }
        let key = widget_key?;
        if !self.focused_key_participates(widget_id) {
            return None;
        }
        if !sample.is_initial_press() {
            return Some(FocusedKeyDispatch::default());
        }

        let press = host_press.unwrap_or(KeyPress {
            key: key.to_key_code(),
            command: modifiers.command,
            control: modifiers.control,
            shift: modifiers.shift,
            alt: modifiers.alt,
        });
        let resolution =
            self.host_resolve_key_press(self.interaction.focus.pending_key_chord, press, focus);
        self.interaction.focus.pending_key_chord = resolution.pending_chord;
        if let Some(message) = resolution.action {
            let outcome = self.dispatch_message(message);
            self.pending_input_command_outcome.merge(outcome);
            return Some(FocusedKeyDispatch {
                widget_id: None,
                routed: true,
            });
        }
        if resolution.handled {
            return Some(FocusedKeyDispatch {
                widget_id: None,
                routed: true,
            });
        }

        // Host resolution is allowed to observe and mutate host-owned state,
        // so validate the same focus identity and authority before delivery.
        if self.interaction.focus.focused_widget() != Some(widget_id)
            || !self.is_authoritative_focus_target(widget_id)
            || !self.focused_key_participates(widget_id)
        {
            return Some(FocusedKeyDispatch::default());
        }
        let widget_id = self.dispatch_focused_input(WidgetInput::key_press_with_metadata(
            key, modifiers, false, timestamp,
        ));
        if let Some(widget_id) = widget_id {
            self.establish_focused_key_capture(widget_id, key);
            return Some(FocusedKeyDispatch {
                widget_id: Some(widget_id),
                routed: true,
            });
        }
        Some(FocusedKeyDispatch::default())
    }

    fn dispatch_captured_focused_key(
        &mut self,
        capture: RuntimeFocusedKeyCapture,
        widget_key: Option<WidgetKey>,
        modifiers: KeyboardModifiers,
        timestamp: Option<InputTimestamp>,
        sample: FocusedKeySample,
    ) -> FocusedKeyDispatch {
        if !self.focused_key_capture_is_current(capture) {
            self.mark_focused_key_capture_stale(capture.widget_id);
            return FocusedKeyDispatch::default();
        }
        let Some(key) = widget_key else {
            return FocusedKeyDispatch::default();
        };
        let matching_key = key == capture.key;
        let owner_cancellation = self.focused_widget_preempts_host_shortcut_key(key);
        let deliver = match sample {
            FocusedKeySample::Press { repeat } => (repeat && matching_key) || owner_cancellation,
            FocusedKeySample::Release => matching_key || owner_cancellation,
        };
        if !deliver {
            return FocusedKeyDispatch::default();
        }

        let input = match sample {
            FocusedKeySample::Press { repeat } => {
                WidgetInput::key_press_with_metadata(key, modifiers, repeat, timestamp)
            }
            FocusedKeySample::Release => {
                WidgetInput::key_release_with_metadata(key, modifiers, timestamp)
            }
        };
        let Some(widget_id) = self.dispatch_focused_input(input) else {
            self.mark_focused_key_capture_stale(capture.widget_id);
            return FocusedKeyDispatch::default();
        };
        self.reconcile_focused_key_capture_after_delivery(widget_id, capture.key);
        FocusedKeyDispatch {
            widget_id: Some(widget_id),
            routed: true,
        }
    }

    fn focused_key_participates(&self, widget_id: WidgetId) -> bool {
        self.surface_widget(widget_id).is_some_and(|widget| {
            widget.widget_object().participates_in_focused_key_routing()
                && self.focused_key_widget_has_authority(widget_id)
        })
    }

    fn focused_key_widget_has_authority(&self, widget_id: WidgetId) -> bool {
        self.surface_widget(widget_id).is_some_and(|widget| {
            widget.is_focusable() && !widget.widget_object().common().state.read_only
        })
    }

    fn focused_key_capture_is_current(&self, capture: RuntimeFocusedKeyCapture) -> bool {
        self.interaction.focus.focused_widget() == Some(capture.widget_id)
            && self.is_authoritative_focus_target(capture.widget_id)
            && self.focused_key_widget_has_authority(capture.widget_id)
            && self
                .surface_widget(capture.widget_id)
                .is_some_and(|widget| {
                    let object = widget.widget_object();
                    object.participates_in_focused_key_routing()
                        && object.captured_focused_key() == Some(capture.key)
                })
    }

    fn establish_focused_key_capture(&mut self, widget_id: WidgetId, key: WidgetKey) {
        if self.interaction.focus.focused_key_capture.is_some() {
            return;
        }
        if self.interaction.focus.focused_widget() == Some(widget_id)
            && self.is_authoritative_focus_target(widget_id)
            && self.focused_key_widget_has_authority(widget_id)
            && self.surface_widget(widget_id).is_some_and(|widget| {
                let object = widget.widget_object();
                object.participates_in_focused_key_routing()
                    && object.captured_focused_key() == Some(key)
            })
        {
            self.interaction.focus.focused_key_capture = Some(RuntimeFocusedKeyCapture {
                widget_id,
                key,
                stale: false,
            });
        }
    }

    fn reconcile_focused_key_capture_after_delivery(
        &mut self,
        widget_id: WidgetId,
        captured_key: WidgetKey,
    ) {
        let Some(capture) = self.interaction.focus.focused_key_capture else {
            return;
        };
        if capture.stale || capture.widget_id != widget_id {
            return;
        }
        let Some((participates, reported_key)) = self.surface_widget(widget_id).map(|widget| {
            let object = widget.widget_object();
            (
                object.participates_in_focused_key_routing(),
                object.captured_focused_key(),
            )
        }) else {
            self.mark_focused_key_capture_stale(widget_id);
            return;
        };
        if !self.is_authoritative_focus_target(widget_id) || !participates {
            self.mark_focused_key_capture_stale(widget_id);
            return;
        }
        match reported_key {
            Some(key) if key == captured_key => {}
            None => self.interaction.focus.focused_key_capture = None,
            Some(_) => self.mark_focused_key_capture_stale(widget_id),
        }
    }

    pub(super) fn mark_focused_key_capture_stale(&mut self, widget_id: WidgetId) {
        if let Some(capture) = &mut self.interaction.focus.focused_key_capture
            && capture.widget_id == widget_id
        {
            capture.stale = true;
        }
    }

    pub(super) fn validate_focused_key_capture_authority(&mut self) {
        let Some(capture) = self.interaction.focus.focused_key_capture else {
            return;
        };
        if capture.stale || !self.focused_key_capture_is_current(capture) {
            self.mark_focused_key_capture_stale(capture.widget_id);
        }
    }

    /// Return whether the current focus target is a text input.
    pub fn focused_text_input_id(&self) -> Option<WidgetId> {
        let widget_id = self.interaction.focus.focused_widget()?;
        self.surface_widget(widget_id).and_then(|widget| {
            widget
                .widget_object()
                .accepts_text_input()
                .then_some(widget_id)
        })
    }

    /// Return the exact current scalar replacement and selection context for
    /// a focused native composition start.
    pub fn focused_composition_start_context(&self) -> Option<CompositionStartContext> {
        let widget_id = self.interaction.focus.focused_widget()?;
        if !self.is_authoritative_focus_target(widget_id)
            || self.focused_text_input_id() != Some(widget_id)
        {
            return None;
        }
        self.surface_widget(widget_id)
            .and_then(|widget| widget.widget_object().composition_start_context())
    }

    /// Return whether the focused widget asks to receive `key` before host shortcuts.
    pub fn focused_widget_preempts_host_shortcut_key(&self, key: WidgetKey) -> bool {
        let Some(widget_id) = self.interaction.focus.focused_widget() else {
            return false;
        };
        self.surface_widget(widget_id)
            .is_some_and(|widget| widget.widget_object().preempts_host_shortcut_key(key))
    }

    /// Return selected text from the focused text input as a borrowed slice, if any.
    pub fn focused_text_selection_slice(&self) -> Option<&str> {
        let widget_id = self.focused_text_input_id()?;
        self.surface_widget(widget_id)
            .and_then(|widget| widget.widget_object().selected_text_slice())
    }

    /// Return selected text from the focused text input as an owned string, if any.
    pub fn focused_text_selection(&self) -> Option<String> {
        self.focused_text_selection_slice().map(str::to_owned)
    }

    /// Resolve one keypress through host-owned shortcuts before falling back to
    /// focused-widget key routing.
    ///
    /// Returns `true` when the shortcut catalog handled the press or a focused
    /// widget received it.
    pub fn dispatch_key_press(
        &mut self,
        press: KeyPress,
        widget_key: Option<WidgetKey>,
        focus: FocusSurface,
    ) -> bool {
        self.dispatch_key_press_with_timestamp(
            press,
            widget_key,
            focus,
            KeyboardModifiers::from(press),
            None,
            false,
        )
    }

    /// Resolve one keypress and preserve an optional native input timestamp on
    /// focused-widget fallback routing.
    pub(crate) fn dispatch_key_press_with_timestamp(
        &mut self,
        press: KeyPress,
        widget_key: Option<WidgetKey>,
        focus: FocusSurface,
        widget_modifiers: KeyboardModifiers,
        timestamp: Option<InputTimestamp>,
        repeat: bool,
    ) -> bool {
        if let Some(route) = self.dispatch_metadata_focused_key_press(
            Some(press),
            widget_key,
            widget_modifiers,
            timestamp,
            repeat,
            focus,
        ) {
            return route.routed;
        }
        let resolution =
            self.host_resolve_key_press(self.interaction.focus.pending_key_chord, press, focus);
        self.interaction.focus.pending_key_chord = resolution.pending_chord;
        if let Some(message) = resolution.action {
            let outcome = self.dispatch_message(message);
            self.pending_input_command_outcome.merge(outcome);
            return true;
        }
        if resolution.handled {
            return true;
        }
        widget_key
            .and_then(|key| {
                self.dispatch_focused_input(WidgetInput::key_press_with_metadata(
                    key,
                    widget_modifiers,
                    repeat,
                    timestamp,
                ))
            })
            .is_some()
    }

    pub(super) fn route_focus_changed(&mut self, widget_id: WidgetId, focused: bool) {
        let _ = self.dispatch_input_output(widget_id, WidgetInput::FocusChanged(focused));
    }

    pub(super) fn restore_focused_widget_state(&mut self, widget_id: WidgetId) {
        let Some(bounds) = self.layout.rects.get(&widget_id).copied() else {
            return;
        };
        let _ = self.dispatch_surface_input(widget_id, bounds, WidgetInput::FocusChanged(true));
    }
}

fn next_focus_target(
    current: Option<WidgetId>,
    order: &[WidgetId],
    rank: &HashMap<WidgetId, usize>,
    direction: FocusTraversal,
) -> Option<WidgetId> {
    if order.is_empty() {
        return None;
    }
    let current_index = current.and_then(|widget_id| rank.get(&widget_id).copied());
    let next_index = match (current_index, direction) {
        (Some(index), FocusTraversal::Forward) => (index + 1) % order.len(),
        (Some(0), FocusTraversal::Backward) => order.len() - 1,
        (Some(index), FocusTraversal::Backward) => index - 1,
        (None, FocusTraversal::Forward) => 0,
        (None, FocusTraversal::Backward) => order.len() - 1,
    };
    Some(order[next_index])
}
