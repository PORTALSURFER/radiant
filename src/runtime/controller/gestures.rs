//! Bounded widget/ancestor recognition sharing controller capture admission and teardown.
use super::{SurfaceRuntime, interaction_state::RuntimeManagedCompositionState};
use crate::{
    gui::pointer_ingress::{
        GestureIngress, GestureIngressDisposition, GestureKind, GesturePhase, PointerSequenceToken,
    },
    gui::types::{Point, Vector2},
    runtime::{FocusTransferOutcome, RuntimeBridge, WidgetDispatchResult},
    widgets::{
        GestureCancellation, GestureEvent, GesturePolicy, WidgetId, WidgetInput,
        WidgetSemanticsRevision,
    },
};

/// Opaque identity of one runtime-admitted gesture sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GestureSequenceToken(PointerSequenceToken);
/// An explicit normalized gesture request and optional continuation authority.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GestureRequest {
    sample: GestureIngress,
    token: Option<GestureSequenceToken>,
}
impl GestureRequest {
    /// Create a tokenless Started request. Continuations require `with_token`.
    pub const fn new(sample: GestureIngress) -> Self {
        Self {
            sample,
            token: None,
        }
    }
    /// Attach the exact token returned by the admitting runtime.
    pub const fn with_token(mut self, token: GestureSequenceToken) -> Self {
        self.token = Some(token);
        self
    }
}
/// Exact result of gesture recognition or delivery.
#[derive(Clone, Debug, PartialEq)]
pub enum GestureOutcome {
    /// The recognizer has not crossed its declared threshold.
    Pending,
    /// One recognized event reached the current widget.
    Accepted(WidgetId),
    /// One recognized event reached a current container consumer.
    AcceptedContainer(crate::layout::NodeId),
    /// The sequence ended without crossing its threshold.
    Unrecognized,
    /// No eligible gesture consumer exists at the anchor.
    Unsupported,
    /// Another interaction owns capture or composition.
    Blocked,
    /// The token or retained owner is obsolete.
    Stale,
    /// Accumulation cannot be represented as finite coordinates.
    Invalid,
    /// The runtime is closed.
    Unavailable,
    /// Focus admission terminated before capture or gesture delivery.
    Focus(FocusTransferOutcome),
}
/// Admission result with authority for the next continuation, if still live.
#[derive(Clone, Debug, PartialEq)]
pub struct GestureAdmission {
    outcome: GestureOutcome,
    token: Option<GestureSequenceToken>,
}
impl GestureAdmission {
    /// Exact routing result.
    pub fn outcome(&self) -> &GestureOutcome {
        &self.outcome
    }
    /// Copy the current continuation token. Terminal outcomes never retain one.
    pub const fn token(&self) -> Option<GestureSequenceToken> {
        self.token
    }
}

#[derive(Clone)]
struct GestureTarget {
    id: WidgetId,
    path: crate::runtime::surface::WidgetPath,
    policy: GesturePolicy,
    owner: GestureOwner,
}
#[derive(Clone)]
enum GestureOwner {
    Widget {
        kind: &'static str,
        revision: WidgetSemanticsRevision,
    },
    Container {
        revision: crate::layout::LayoutInteractionRevision,
        interaction_revision: crate::layout::LayoutInteractionRevision,
        policy: std::rc::Rc<crate::layout::ContainerPolicy>,
        contract_version: u16,
    },
}
impl GestureTarget {
    fn outcome(&self) -> GestureOutcome {
        match self.owner {
            GestureOwner::Widget { .. } => GestureOutcome::Accepted(self.id),
            GestureOwner::Container { .. } => GestureOutcome::AcceptedContainer(self.id),
        }
    }
    fn is_widget(&self) -> bool {
        matches!(self.owner, GestureOwner::Widget { .. })
    }
}

pub(super) struct GestureCapture {
    target: GestureTarget,
    // Pending candidates are ordered deepest first. They never own capture.
    candidates: Vec<GestureTarget>,
    hit_widget: WidgetId,
    hit_path: crate::runtime::surface::WidgetPath,
    generation: u64,
    token: GestureSequenceToken,
    sample: GestureIngress,
    anchor: Point,
    accumulated: Vector2,
    active: bool,
}
impl<Bridge: RuntimeBridge<Message>, Message> SurfaceRuntime<Bridge, Message> {
    /// Recognize one checked gesture through widget and ancestor consumers.
    /// Pending candidates do not focus or capture. The deepest candidate whose
    /// threshold is crossed claims the sequence through shared controller
    /// capture admission and teardown; all other pointer consumers are blocked.
    pub fn dispatch_gesture_request(&mut self, request: GestureRequest) -> GestureAdmission {
        let mut admitted_token = None;
        let outcome = self.route_gesture_request(request, &mut admitted_token);
        let token = if matches!(
            outcome,
            GestureOutcome::Pending
                | GestureOutcome::Accepted(_)
                | GestureOutcome::AcceptedContainer(_)
        ) && !matches!(
            request.sample.phase(),
            GesturePhase::Ended | GesturePhase::Cancelled
        ) {
            self.interaction
                .gesture
                .as_ref()
                .filter(|capture| {
                    Some(capture.token) == admitted_token
                        && capture.sample.device() == request.sample.device()
                        && capture.sample.kind() == request.sample.kind()
                })
                .map(|capture| capture.token)
        } else {
            None
        };
        GestureAdmission { outcome, token }
    }

    fn route_gesture_request(
        &mut self,
        request: GestureRequest,
        admitted_token: &mut Option<GestureSequenceToken>,
    ) -> GestureOutcome {
        if !self.lifecycle_accepts_work() {
            return GestureOutcome::Unavailable;
        }
        let sample = request.sample;
        let mut capture = if sample.phase() == GesturePhase::Started {
            if request.token.is_some() {
                return GestureOutcome::Stale;
            }
            if self.interaction.gesture.is_some() || self.gesture_has_incumbent() {
                return GestureOutcome::Blocked;
            }
            let Some(anchor) = sample
                .anchor()
                .or(self.interaction.pointer.current_position)
            else {
                return GestureOutcome::Unsupported;
            };
            if self.layout_target_at(anchor).is_some()
                || self.scroll_affordance_at(anchor).is_some()
            {
                return GestureOutcome::Unsupported;
            }
            let Some(widget) = self.widget_at_for_input(anchor, &WidgetInput::pointer_move(anchor))
            else {
                return GestureOutcome::Unsupported;
            };
            let Some(current) = self.surface_widget(widget) else {
                return GestureOutcome::Unsupported;
            };
            let common = current.widget_object().common();
            if common.state.disabled
                || common.state.read_only
                || self.accessibility_incumbent_owner(widget).is_some()
            {
                return GestureOutcome::Blocked;
            }
            let Some(hit_path) = self.traversal.widgets.paths.current.get(&widget).cloned() else {
                return GestureOutcome::Unsupported;
            };
            let candidates = match self.gesture_candidates(widget, &hit_path, sample.kind(), anchor)
            {
                Ok(candidates) => candidates,
                Err(outcome) => return outcome,
            };
            let Some(target) = candidates.first().cloned() else {
                return GestureOutcome::Unsupported;
            };
            let Ok(token) = self.interaction.pointer.ingress.allocator.issue() else {
                return GestureOutcome::Unavailable;
            };
            GestureCapture {
                target,
                candidates,
                hit_widget: widget,
                hit_path,
                generation: self.refresh_counters().runtime_projection,
                token: GestureSequenceToken(token),
                sample,
                anchor,
                accumulated: Vector2::new(
                    if sample.kind() == GestureKind::Pinch {
                        1.0
                    } else {
                        0.0
                    },
                    0.0,
                ),
                active: false,
            }
        } else {
            let Some(current) = self.interaction.gesture.as_ref() else {
                return GestureOutcome::Stale;
            };
            if request.token != Some(current.token)
                || sample.device() != current.sample.device()
                || sample.kind() != current.sample.kind()
            {
                return GestureOutcome::Stale;
            }
            let Some(capture) = self.interaction.gesture.take() else {
                return GestureOutcome::Stale;
            };
            capture
        };
        *admitted_token = Some(capture.token);
        if !self.gesture_capture_is_current(&capture) {
            self.finish_gesture_capture(capture, GestureCancellation::Retired);
            return GestureOutcome::Stale;
        }
        if sample.phase() == GesturePhase::Cancelled {
            capture.sample = sample;
            let outcome = capture.target.outcome();
            let active = capture.active;
            self.finish_gesture_capture(capture, GestureCancellation::Source);
            return if active {
                outcome
            } else {
                GestureOutcome::Unrecognized
            };
        }
        let accumulated = match sample.kind() {
            GestureKind::Pan => Vector2::new(
                capture.accumulated.x + sample.value().x,
                capture.accumulated.y + sample.value().y,
            ),
            GestureKind::Pinch => Vector2::new(capture.accumulated.x * sample.value().x, 0.0),
            GestureKind::Rotate => Vector2::new(capture.accumulated.x + sample.value().x, 0.0),
        };
        if !accumulated.x.is_finite()
            || !accumulated.y.is_finite()
            || (sample.kind() == GestureKind::Pinch && accumulated.x <= 0.0)
        {
            self.finish_gesture_capture(capture, GestureCancellation::InvalidSample);
            return GestureOutcome::Invalid;
        }
        capture.sample = sample;
        capture.accumulated = accumulated;
        let terminal = sample.phase() == GesturePhase::Ended;
        let was_active = capture.active;
        if !was_active {
            let magnitude = match sample.kind() {
                GestureKind::Pan => f64::from(accumulated.x).hypot(f64::from(accumulated.y)),
                GestureKind::Pinch => f64::from(accumulated.x - 1.0).abs(),
                GestureKind::Rotate => f64::from(accumulated.x).abs(),
            };
            let winner = capture
                .candidates
                .iter()
                .find(|target| {
                    target
                        .policy
                        .threshold(sample.kind())
                        .is_some_and(|threshold| magnitude >= f64::from(threshold))
                })
                .cloned();
            let Some(winner) = winner else {
                if !terminal {
                    self.interaction.gesture = Some(capture);
                }
                return if terminal {
                    GestureOutcome::Unrecognized
                } else {
                    GestureOutcome::Pending
                };
            };
            capture.target = winner;
            if !self.gesture_pending_target_is_current(&capture) {
                return GestureOutcome::Stale;
            }
            if self.gesture_has_incumbent()
                || self
                    .accessibility_incumbent_owner(capture.hit_widget)
                    .is_some()
            {
                return GestureOutcome::Blocked;
            }
            if capture.target.is_widget()
                && self
                    .surface_widget(capture.target.id)
                    .is_some_and(|widget| widget.is_focusable())
            {
                let Some(target) = self.focus_target(capture.target.id) else {
                    return GestureOutcome::Stale;
                };
                match self.transfer_focus(&target) {
                    FocusTransferOutcome::Admitted(_) => {}
                    outcome => return GestureOutcome::Focus(outcome),
                }
            }
            if !self.gesture_capture_is_current(&capture)
                || !self.gesture_pending_target_is_current(&capture)
                || self.gesture_has_incumbent()
                || self
                    .accessibility_incumbent_owner(capture.hit_widget)
                    .is_some()
            {
                return GestureOutcome::Stale;
            }
            if !self.gesture_handler_available(&capture.target) {
                return GestureOutcome::Unsupported;
            }
            capture.active = true;
            capture.candidates.clear();
            if capture.target.is_widget() {
                self.interaction.pointer.capture = Some(capture.target.id);
                self.interaction.pointer.capture_button = None;
            }
        }
        let target = capture.target.clone();
        let token = capture.token;
        let outcome = target.outcome();
        let event = GestureEvent {
            sample,
            anchor: capture.anchor,
            phase: if was_active {
                sample.phase()
            } else {
                GesturePhase::Started
            },
            accumulated,
            cancellation: None,
        };
        if terminal && was_active {
            self.clear_gesture_pointer_capture(&target);
            return if self.deliver_gesture(&target, event) {
                outcome
            } else {
                GestureOutcome::Unsupported
            };
        }
        self.interaction.gesture = Some(capture);
        if !self.deliver_gesture(&target, event) {
            self.cancel_gesture_capture(GestureCancellation::Retired);
            return GestureOutcome::Unsupported;
        }
        // A reducer can retire this sequence or admit another one. Never end
        // a replacement sequence solely because it selected the same node.
        if terminal
            && self
                .interaction
                .gesture
                .as_ref()
                .is_some_and(|capture| capture.token == token)
        {
            self.interaction.gesture = None;
            self.clear_gesture_pointer_capture(&target);
            if !self.deliver_gesture(
                &target,
                GestureEvent {
                    phase: GesturePhase::Ended,
                    ..event
                },
            ) {
                return GestureOutcome::Unsupported;
            }
        }
        outcome
    }

    fn gesture_candidates(
        &mut self,
        widget: WidgetId,
        path: &crate::runtime::surface::WidgetPath,
        kind: GestureKind,
        anchor: Point,
    ) -> Result<Vec<GestureTarget>, GestureOutcome> {
        if self
            .traversal
            .widgets
            .duplicate_widget_ids
            .contains(&widget)
        {
            return Err(GestureOutcome::Unsupported);
        }
        let mut candidates = Vec::new();
        if let Some(current) = self.surface_widget(widget)
            && let Some((policy, revision)) = current.gesture_policy()
            && policy.threshold(kind).is_some()
        {
            let target = GestureTarget {
                id: widget,
                path: path.clone(),
                policy,
                owner: GestureOwner::Widget {
                    kind: current.widget_object().compatibility_kind(),
                    revision,
                },
            };
            if self.gesture_handler_available(&target) {
                candidates.push(target);
            }
        }
        for record in &self.traversal.containers.layout_interactions {
            if !path.as_slice().starts_with(record.path.as_slice())
                || !self
                    .layout
                    .rects
                    .get(&record.id)
                    .is_some_and(|bounds| bounds.contains(anchor))
            {
                continue;
            }
            let facets = record.interaction.capabilities_v2();
            let Some(gestures) = facets.gestures() else {
                continue;
            };
            let policy = gestures.policy();
            if policy.threshold(kind).is_none() {
                continue;
            }
            if !record.gesture_qualified {
                return Err(GestureOutcome::Unsupported);
            }
            if candidates.len() == 64
                || self
                    .traversal
                    .containers
                    .layout_interactions
                    .iter()
                    .filter(|other| other.id == record.id)
                    .count()
                    != 1
            {
                return Err(GestureOutcome::Unsupported);
            }
            let Some(container) = self.surface.find_container_at_path(&record.path) else {
                continue;
            };
            // Custom measure/place policies expose no equality revision. They
            // cannot authorize retained container gesture ownership yet.
            if container.revision().layout_policy.is_some() {
                continue;
            }
            candidates.push(GestureTarget {
                id: record.id,
                path: record.path.clone(),
                policy,
                owner: GestureOwner::Container {
                    revision: gestures.revision(),
                    interaction_revision: record.revision.clone(),
                    policy: std::rc::Rc::new(container.revision().policy.clone()),
                    contract_version: record.contract_version,
                },
            });
        }
        candidates.sort_by_key(|target| std::cmp::Reverse(target.path.as_slice().len()));
        Ok(candidates)
    }
    fn gesture_pending_target_is_current(&self, capture: &GestureCapture) -> bool {
        self.layout_target_at(capture.anchor).is_none()
            && (capture.target.is_widget()
                || self
                    .layout
                    .rects
                    .get(&capture.target.id)
                    .is_some_and(|bounds| bounds.contains(capture.anchor)))
            && self
                .surface_widget(capture.hit_widget)
                .is_some_and(|widget| {
                    let common = widget.widget_object().common();
                    !common.state.disabled && !common.state.read_only
                })
            && self.scroll_affordance_at(capture.anchor).is_none()
            && self
                .traversal
                .widgets
                .paths
                .current
                .get(&capture.hit_widget)
                == Some(&capture.hit_path)
            && self.widget_at_for_input(capture.anchor, &WidgetInput::pointer_move(capture.anchor))
                == Some(capture.hit_widget)
    }
    fn gesture_has_incumbent(&self) -> bool {
        matches!(
            self.interaction.wheel.managed_sequence,
            super::interaction_state::RuntimeManagedWheelSequenceState::Active { .. }
        ) || self.interaction.pointer.capture.is_some()
            || self.interaction.pointer.managed_capture.is_some()
            || self.interaction.pointer.scroll_drag_capture.is_some()
            || self.interaction.layout_capture.is_some()
            || self.interaction.composition.managed_composition
                != RuntimeManagedCompositionState::Idle
    }
    fn gesture_target_matches_surface(
        target: &GestureTarget,
        surface: &crate::runtime::UiSurface<Message>,
        paths: &std::collections::HashMap<WidgetId, crate::runtime::WidgetPath>,
        same_projection: bool,
    ) -> bool {
        match &target.owner {
            GestureOwner::Widget { kind, revision } => {
                paths.get(&target.id) == Some(&target.path)
                    && surface
                        .find_widget_at_path(target.id, &target.path)
                        .is_some_and(|widget| {
                            let common = widget.widget_object().common();
                            !common.state.disabled
                                && !common.state.read_only
                                && widget.widget_object().compatibility_kind() == *kind
                                && widget.gesture_policy().is_some_and(|(policy, current)| {
                                    policy == target.policy
                                        && ((revision.is_exact() && current == *revision)
                                            || (!revision.is_exact() && same_projection))
                                })
                        })
            }
            GestureOwner::Container {
                revision,
                interaction_revision,
                policy,
                contract_version,
            } => surface
                .find_container_at_path(&target.path)
                .is_some_and(|container| {
                    container.node_id() == target.id
                        && container.revision().layout_policy.is_none()
                        && container.revision().policy == policy.as_ref()
                        && container
                            .revision()
                            .layout_capabilities
                            .is_some_and(|capabilities| {
                                capabilities.contract_version == *contract_version
                                    && capabilities.interaction.as_ref().is_some_and(
                                        |interaction| {
                                            let current = interaction.revision();
                                            ((interaction_revision.is_exact()
                                                && current == *interaction_revision)
                                                || (!interaction_revision.is_exact()
                                                    && same_projection))
                                                && interaction
                                                    .capabilities_v2()
                                                    .gestures()
                                                    .is_some_and(|gestures| {
                                                        gestures.policy() == target.policy
                                                            && ((revision.is_exact()
                                                                && gestures.revision()
                                                                    == *revision)
                                                                || (!revision.is_exact()
                                                                    && same_projection))
                                                    })
                                        },
                                    )
                            })
                }),
        }
    }
    fn gesture_capture_is_current(&self, capture: &GestureCapture) -> bool {
        let current = |target: &GestureTarget| {
            self.layout.rects.get(&target.id).is_some_and(|bounds| {
                bounds.min.is_finite()
                    && bounds.max.is_finite()
                    && bounds.width() > 0.0
                    && bounds.height() > 0.0
            }) && !self
                .traversal
                .widgets
                .duplicate_widget_ids
                .contains(&target.id)
                && (!matches!(target.owner, GestureOwner::Container { .. })
                    || self
                        .traversal
                        .containers
                        .layout_interactions
                        .iter()
                        .filter(|record| record.id == target.id && record.gesture_qualified)
                        .count()
                        == 1)
                && Self::gesture_target_matches_surface(
                    target,
                    &self.surface,
                    &self.traversal.widgets.paths.current,
                    capture.generation == self.refresh_counters().runtime_projection,
                )
        };
        self.lifecycle_accepts_work()
            && current(&capture.target)
            && (capture.active || capture.candidates.iter().all(current))
            && (!capture.active
                || !capture.target.is_widget()
                || self.interaction.pointer.capture == Some(capture.target.id))
    }
    fn gesture_handler_available(&mut self, target: &GestureTarget) -> bool {
        if target.is_widget() {
            self.surface_widget_mut(target.id)
                .is_some_and(|widget| widget.has_gesture_handler(target.policy))
        } else {
            self.surface
                .find_container_at_path(&target.path)
                .and_then(|container| container.revision().layout_capabilities)
                .and_then(|capabilities| capabilities.interaction.as_ref())
                .and_then(|interaction| interaction.capabilities_v2().gestures())
                .is_some_and(|gestures| gestures.policy() == target.policy)
        }
    }
    fn gesture_dispatch(
        &mut self,
        target: &GestureTarget,
        event: GestureEvent,
    ) -> Option<WidgetDispatchResult<Message>> {
        if target.is_widget() {
            self.surface_widget_mut(target.id)
                .and_then(|widget| widget.dispatch_gesture(event))
        } else {
            let interaction = self
                .surface
                .find_container_at_path(&target.path)?
                .revision()
                .layout_capabilities?
                .interaction
                .clone()?;
            let gestures = interaction.capabilities_v2().gestures()?;
            if gestures.policy() != target.policy {
                return None;
            }
            Some(gestures.dispatch(event).map_or(
                WidgetDispatchResult::NoOutput,
                WidgetDispatchResult::Message,
            ))
        }
    }
    fn deliver_gesture(&mut self, target: &GestureTarget, event: GestureEvent) -> bool {
        let Some(dispatch) = self.gesture_dispatch(target, event) else {
            return false;
        };
        self.finish_gesture_dispatch(dispatch);
        true
    }
    fn clear_gesture_pointer_capture(&mut self, target: &GestureTarget) {
        if target.is_widget() && self.interaction.pointer.capture == Some(target.id) {
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_button = None;
            self.interaction.pointer.capture_state = None;
        }
    }
    pub(super) fn finish_gesture_dispatch(&mut self, dispatch: WidgetDispatchResult<Message>) {
        match dispatch {
            WidgetDispatchResult::Message(message) => {
                let outcome = self.dispatch_message(message);
                self.pending_input_command_outcome.merge(outcome);
            }
            WidgetDispatchResult::Command(activation) => {
                let dispatch = self.resolve_command_request(
                    activation.request(),
                    crate::gui::focus::FocusSurface::None,
                );
                if let Some(message) = dispatch.message {
                    let outcome = self.dispatch_message(message);
                    self.pending_input_command_outcome.merge(outcome);
                }
            }
            WidgetDispatchResult::NoOutput | WidgetDispatchResult::UnmappedOutput => {
                self.relayout()
            }
        }
    }
    fn finish_gesture_capture(&mut self, capture: GestureCapture, reason: GestureCancellation) {
        if let Some(dispatch) = self.gesture_cancellation_dispatch(capture, reason) {
            self.finish_gesture_dispatch(dispatch);
        }
    }
    fn gesture_cancellation_dispatch(
        &mut self,
        capture: GestureCapture,
        reason: GestureCancellation,
    ) -> Option<WidgetDispatchResult<Message>> {
        self.clear_gesture_pointer_capture(&capture.target);
        if capture.active
            && Self::gesture_target_matches_surface(
                &capture.target,
                &self.surface,
                &self.traversal.widgets.paths.current,
                capture.generation == self.refresh_counters().runtime_projection,
            )
        {
            return self.gesture_dispatch(
                &capture.target,
                GestureEvent {
                    sample: capture.sample,
                    anchor: capture.anchor,
                    phase: GesturePhase::Cancelled,
                    accumulated: capture.accumulated,
                    cancellation: Some(reason),
                },
            );
        }
        None
    }
    pub(super) fn take_gesture_cancellation(
        &mut self,
        reason: GestureCancellation,
    ) -> Option<WidgetDispatchResult<Message>> {
        let capture = self.interaction.gesture.take()?;
        self.gesture_cancellation_dispatch(capture, reason)
    }
    pub(super) fn cancel_gesture_capture(&mut self, reason: GestureCancellation) {
        if let Some(capture) = self.interaction.gesture.take() {
            self.finish_gesture_capture(capture, reason);
        }
    }
    pub(super) fn reconcile_gesture_before_surface_replace(
        &mut self,
        next: &crate::runtime::UiSurface<Message>,
        paths: &std::collections::HashMap<WidgetId, crate::runtime::WidgetPath>,
        retired: &[WidgetId],
        terminal_messages: &mut Vec<Message>,
    ) {
        let Some(capture) = self.interaction.gesture.as_ref() else {
            return;
        };
        let has_container = !capture.target.is_widget()
            || capture.candidates.iter().any(|target| !target.is_widget());
        let unambiguous = !has_container || next.gesture_source_is_unambiguous();
        let compatible = |target: &GestureTarget| {
            unambiguous
                && (!target.is_widget() || !retired.contains(&target.id))
                && Self::gesture_target_matches_surface(target, next, paths, false)
        };
        if compatible(&capture.target)
            && (capture.active || capture.candidates.iter().all(compatible))
        {
            return;
        }
        let Some(capture) = self.interaction.gesture.take() else {
            return;
        };
        self.clear_gesture_pointer_capture(&capture.target);
        if !capture.active {
            return;
        }
        let event = GestureEvent {
            sample: capture.sample,
            anchor: capture.anchor,
            phase: GesturePhase::Cancelled,
            accumulated: capture.accumulated,
            cancellation: Some(GestureCancellation::Retired),
        };
        if let Some(WidgetDispatchResult::Message(message)) =
            self.gesture_dispatch(&capture.target, event)
        {
            terminal_messages.push(message);
        }
    }

    pub(super) fn validate_gesture_capture(&mut self) {
        if self
            .interaction
            .gesture
            .as_ref()
            .is_some_and(|capture| !self.gesture_capture_is_current(capture))
        {
            self.cancel_gesture_capture(GestureCancellation::Retired);
        }
    }
    pub(super) fn gesture_blocks_widget_input(&self, input: &WidgetInput) -> bool {
        self.gesture_owns_pointer_capture()
            && matches!(
                input,
                WidgetInput::PointerMove { .. }
                    | WidgetInput::PointerPress { .. }
                    | WidgetInput::PointerDoubleClick { .. }
                    | WidgetInput::PointerRelease { .. }
                    | WidgetInput::PointerModifiersChanged { .. }
                    | WidgetInput::Wheel { .. }
            )
    }
    pub(crate) fn gesture_owns_pointer_capture(&self) -> bool {
        self.interaction
            .gesture
            .as_ref()
            .is_some_and(|capture| capture.active)
    }
    pub(crate) fn retained_gesture_device(
        &self,
    ) -> Option<crate::gui::pointer_ingress::InputDeviceId> {
        self.interaction
            .gesture
            .as_ref()
            .map(|capture| capture.sample.device())
    }
    pub(crate) fn reject_native_gesture_continuation(
        &mut self,
        device: crate::gui::pointer_ingress::InputDeviceId,
        kind: GestureKind,
        phase: GesturePhase,
    ) {
        if phase != GesturePhase::Started
            && self.interaction.gesture.as_ref().is_some_and(|capture| {
                capture.sample.device() == device && capture.sample.kind() == kind
            })
        {
            self.cancel_gesture_capture(GestureCancellation::InvalidSample);
        }
    }

    pub(crate) fn dispatch_native_gesture_ingress(
        &mut self,
        sample: GestureIngress,
    ) -> GestureIngressDisposition {
        let token = self
            .interaction
            .gesture
            .as_ref()
            .filter(|capture| {
                capture.sample.kind() == sample.kind() && capture.sample.device() == sample.device()
            })
            .map(|capture| capture.token);
        let request = GestureRequest {
            sample,
            token: if sample.phase() == GesturePhase::Started {
                None
            } else {
                token
            },
        };
        match self.dispatch_gesture_request(request).outcome {
            GestureOutcome::Accepted(widget) => GestureIngressDisposition::RoutedWidget(widget),
            GestureOutcome::AcceptedContainer(node) => {
                GestureIngressDisposition::RoutedContainer(node)
            }
            GestureOutcome::Pending => GestureIngressDisposition::Pending,
            GestureOutcome::Unrecognized => GestureIngressDisposition::Unrecognized,
            GestureOutcome::Unsupported => GestureIngressDisposition::AdmittedUnsupportedConsumer,
            GestureOutcome::Invalid => GestureIngressDisposition::Invalid,
            GestureOutcome::Stale => GestureIngressDisposition::Stale,
            _ => GestureIngressDisposition::Blocked,
        }
    }
}
