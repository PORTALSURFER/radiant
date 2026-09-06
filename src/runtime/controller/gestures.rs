//! Qualified gesture lifecycle sharing the existing pointer capture owner.
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

pub(super) struct GestureCapture {
    widget: WidgetId,
    kind: &'static str,
    path: crate::runtime::surface::WidgetPath,
    policy: GesturePolicy,
    revision: WidgetSemanticsRevision,
    generation: u64,
    token: GestureSequenceToken,
    sample: GestureIngress,
    anchor: Point,
    accumulated: Vector2,
    active: bool,
}
impl<Bridge: RuntimeBridge<Message>, Message> SurfaceRuntime<Bridge, Message> {
    /// Recognize one checked gesture and deliver it through the normal widget mapper.
    /// Pending gestures do not focus or capture; a threshold crossing claims the
    /// same pointer capture slot used by existing widget/layout interactions.
    pub fn dispatch_gesture_request(&mut self, request: GestureRequest) -> GestureAdmission {
        let outcome = self.route_gesture_request(request);
        let token = if matches!(
            outcome,
            GestureOutcome::Pending | GestureOutcome::Accepted(_)
        ) && !matches!(
            request.sample.phase(),
            GesturePhase::Ended | GesturePhase::Cancelled
        ) {
            self.interaction
                .gesture
                .as_ref()
                .filter(|capture| {
                    capture.sample.device() == request.sample.device()
                        && capture.sample.kind() == request.sample.kind()
                })
                .map(|capture| capture.token)
        } else {
            None
        };
        GestureAdmission { outcome, token }
    }

    fn route_gesture_request(&mut self, request: GestureRequest) -> GestureOutcome {
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
            let Some((policy, revision)) = current.gesture_policy() else {
                return GestureOutcome::Unsupported;
            };
            if policy.threshold(sample.kind()).is_none() {
                return GestureOutcome::Unsupported;
            }
            let kind = current.widget_object().compatibility_kind();
            let Some(path) = self.traversal.widgets.paths.current.get(&widget).cloned() else {
                return GestureOutcome::Unsupported;
            };
            if self
                .traversal
                .widgets
                .duplicate_widget_ids
                .contains(&widget)
                || !self
                    .surface_widget_mut(widget)
                    .is_some_and(|widget| widget.has_gesture_handler(policy))
            {
                return GestureOutcome::Unsupported;
            }
            let Ok(token) = self.interaction.pointer.ingress.allocator.issue() else {
                return GestureOutcome::Unavailable;
            };
            GestureCapture {
                widget,
                kind,
                path,
                policy,
                revision,
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
        if !self.gesture_capture_is_current(&capture) {
            self.finish_gesture_capture(capture, GestureCancellation::Retired);
            return GestureOutcome::Stale;
        }
        if sample.phase() == GesturePhase::Cancelled {
            capture.sample = sample;
            let widget = capture.widget;
            let active = capture.active;
            self.finish_gesture_capture(capture, GestureCancellation::Source);
            return if active {
                GestureOutcome::Accepted(widget)
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
            if magnitude
                < f64::from(
                    capture
                        .policy
                        .threshold(sample.kind())
                        .unwrap_or(f32::INFINITY),
                )
            {
                if !terminal {
                    self.interaction.gesture = Some(capture);
                }
                return if terminal {
                    GestureOutcome::Unrecognized
                } else {
                    GestureOutcome::Pending
                };
            }
            if !self.gesture_pending_target_is_current(&capture) {
                return GestureOutcome::Stale;
            }
            if self.gesture_has_incumbent() {
                return GestureOutcome::Blocked;
            }
            if self
                .surface_widget(capture.widget)
                .is_some_and(|widget| widget.is_focusable())
            {
                let Some(target) = self.focus_target(capture.widget) else {
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
            {
                return GestureOutcome::Stale;
            }
            if !self
                .surface_widget_mut(capture.widget)
                .is_some_and(|widget| widget.has_gesture_handler(capture.policy))
            {
                return GestureOutcome::Unsupported;
            }
            capture.active = true;
            self.interaction.pointer.capture = Some(capture.widget);
            self.interaction.pointer.capture_button = None;
        }
        let widget = capture.widget;
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
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_button = None;
            self.interaction.pointer.capture_state = None;
            return if self.deliver_gesture(widget, event) {
                GestureOutcome::Accepted(widget)
            } else {
                GestureOutcome::Unsupported
            };
        }
        self.interaction.gesture = Some(capture);
        if !self.deliver_gesture(widget, event) {
            self.cancel_gesture_capture(GestureCancellation::Retired);
            return GestureOutcome::Unsupported;
        }
        // A threshold crossed on Ended still has a complete Started/Ended lifecycle.
        if terminal
            && self
                .interaction
                .gesture
                .as_ref()
                .is_some_and(|capture| capture.widget == widget)
        {
            self.interaction.gesture = None;
            if self.interaction.pointer.capture == Some(widget) {
                self.interaction.pointer.capture = None;
                self.interaction.pointer.capture_state = None;
            }
            if !was_active {
                self.deliver_gesture(
                    widget,
                    GestureEvent {
                        phase: GesturePhase::Ended,
                        ..event
                    },
                );
            }
        }
        GestureOutcome::Accepted(widget)
    }

    fn gesture_pending_target_is_current(&self, capture: &GestureCapture) -> bool {
        self.layout_target_at(capture.anchor).is_none()
            && self.scroll_affordance_at(capture.anchor).is_none()
            && self.widget_at_for_input(capture.anchor, &WidgetInput::pointer_move(capture.anchor))
                == Some(capture.widget)
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
    fn gesture_capture_is_current(&self, capture: &GestureCapture) -> bool {
        if !self.lifecycle_accepts_work()
            || !self.layout.rects.contains_key(&capture.widget)
            || self
                .traversal
                .widgets
                .duplicate_widget_ids
                .contains(&capture.widget)
            || self.traversal.widgets.paths.current.get(&capture.widget) != Some(&capture.path)
        {
            return false;
        }
        let Some(widget) = self.surface_widget(capture.widget) else {
            return false;
        };
        let common = widget.widget_object().common();
        !common.state.disabled
            && !common.state.read_only
            && widget.widget_object().compatibility_kind() == capture.kind
            && (!capture.active || self.interaction.pointer.capture == Some(capture.widget))
            && widget.gesture_policy().is_some_and(|(policy, revision)| {
                policy == capture.policy
                    && ((capture.revision.is_exact() && revision == capture.revision)
                        || (!capture.revision.is_exact()
                            && capture.generation == self.refresh_counters().runtime_projection))
            })
    }
    fn deliver_gesture(&mut self, widget: WidgetId, event: GestureEvent) -> bool {
        let Some(dispatch) = self
            .surface_widget_mut(widget)
            .and_then(|widget| widget.dispatch_gesture(event))
        else {
            return false;
        };
        self.finish_gesture_dispatch(dispatch);
        true
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
        if capture.active && self.interaction.pointer.capture == Some(capture.widget) {
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_button = None;
            self.interaction.pointer.capture_state = None;
        }
        if capture.active
            && self.surface_widget(capture.widget).is_some_and(|widget| {
                widget.widget_object().compatibility_kind() == capture.kind
                    && self.traversal.widgets.paths.current.get(&capture.widget)
                        == Some(&capture.path)
                    && widget.gesture_policy().is_some_and(|(policy, revision)| {
                        policy == capture.policy
                            && ((capture.revision.is_exact() && revision == capture.revision)
                                || (!capture.revision.is_exact()
                                    && capture.generation
                                        == self.refresh_counters().runtime_projection))
                    })
            })
        {
            return self.surface_widget_mut(capture.widget).and_then(|widget| {
                widget.dispatch_gesture(GestureEvent {
                    sample: capture.sample,
                    anchor: capture.anchor,
                    phase: GesturePhase::Cancelled,
                    accumulated: capture.accumulated,
                    cancellation: Some(reason),
                })
            });
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
        let compatible = !retired.contains(&capture.widget)
            && paths.get(&capture.widget) == Some(&capture.path)
            && next
                .find_widget_at_path(capture.widget, &capture.path)
                .is_some_and(|widget| {
                    let common = widget.widget_object().common();
                    common.id == capture.widget
                        && !common.state.disabled
                        && !common.state.read_only
                        && widget.widget_object().compatibility_kind() == capture.kind
                        && widget.gesture_policy().is_some_and(|(policy, revision)| {
                            policy == capture.policy
                                && (capture.revision.is_exact() && revision == capture.revision)
                        })
                });
        if compatible {
            return;
        }
        let Some(capture) = self.interaction.gesture.take() else {
            return;
        };
        if !capture.active {
            return;
        }
        if self.interaction.pointer.capture == Some(capture.widget) {
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_button = None;
            self.interaction.pointer.capture_state = None;
        }
        let event = GestureEvent {
            sample: capture.sample,
            anchor: capture.anchor,
            phase: GesturePhase::Cancelled,
            accumulated: capture.accumulated,
            cancellation: Some(GestureCancellation::Retired),
        };
        if let Some(WidgetDispatchResult::Message(message)) = self
            .surface_widget_mut(capture.widget)
            .and_then(|widget| widget.dispatch_gesture(event))
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
    pub(super) fn gesture_owns_pointer_capture(&self) -> bool {
        self.interaction
            .gesture
            .as_ref()
            .is_some_and(|capture| capture.active)
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
            GestureOutcome::Pending => GestureIngressDisposition::Pending,
            GestureOutcome::Unrecognized => GestureIngressDisposition::Unrecognized,
            GestureOutcome::Unsupported => GestureIngressDisposition::AdmittedUnsupportedConsumer,
            GestureOutcome::Invalid => GestureIngressDisposition::Invalid,
            GestureOutcome::Stale => GestureIngressDisposition::Stale,
            _ => GestureIngressDisposition::Blocked,
        }
    }
}
