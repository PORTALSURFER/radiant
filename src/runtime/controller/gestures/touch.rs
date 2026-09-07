//! Two-contact recognition; ownership and delivery stay in the shared arena.

use super::*;
use crate::gui::pointer_ingress::{
    DeviceKind, GestureUnit, PointerIngress, PointerIngressDisposition, PointerPhase,
};
use crate::runtime::controller::pointer_ingress::touch_gesture::{
    TouchContactSample, TouchPairGeometry, TouchPairReset, TouchPairUpdate,
};

const FAMILIES: [GestureKind; 3] = [GestureKind::Pinch, GestureKind::Rotate, GestureKind::Pan];

pub(super) struct TouchGestureCapture {
    pub(super) tokens: [PointerSequenceToken; 2],
    last: TouchPairGeometry,
    // Parallel to pending capture.candidates. Each unique target counts once.
    families: Vec<u8>,
}

impl<Bridge: RuntimeBridge<Message>, Message> SurfaceRuntime<Bridge, Message> {
    pub(in crate::runtime::controller) fn route_admitted_touch_gesture(
        &mut self,
        ingress: PointerIngress,
        token: PointerSequenceToken,
        cross_window: Option<(
            crate::runtime::controller::gestures::drag_drop::CrossWindowInputHint,
            &mut Option<
                crate::runtime::controller::gestures::drag_drop::CrossWindowTerminalRequest<
                    Message,
                >,
            >,
            &mut Option<crate::runtime::controller::gestures::drag_drop::CrossWindowDragKey>,
        )>,
    ) -> PointerIngressDisposition {
        // Unsupported contacts keep their bounded transport records until up.
        // Never form a new pair around an already-held, untracked finger.
        if matches!(ingress.phase(), PointerPhase::Started { .. })
            && self.interaction.pointer.ingress.touch_pair.is_empty()
            && self
                .interaction
                .pointer
                .ingress
                .records
                .iter()
                .flatten()
                .filter(|record| record.kind == DeviceKind::Touch)
                .count()
                != 1
        {
            return PointerIngressDisposition::AdmittedUnsupportedConsumer;
        }
        let prior_tokens = self.interaction.pointer.ingress.touch_pair.tokens();
        let update = self
            .interaction
            .pointer
            .ingress
            .touch_pair
            .observe(TouchContactSample {
                device: ingress.device(),
                contact: ingress.contact(),
                token,
                phase: ingress.phase(),
                position: ingress.logical_position(),
            });
        match update {
            TouchPairUpdate::FirstContact => {
                self.route_single_touch_drag(ingress, token, cross_window)
            }
            TouchPairUpdate::Ignored => PointerIngressDisposition::AdmittedUnsupportedConsumer,
            TouchPairUpdate::PairEstablished(geometry) => {
                if let Some(capture) = self.interaction.gesture.as_ref()
                    && capture.single_touch.is_some()
                {
                    if capture.active {
                        self.cancel_gesture_capture(GestureCancellation::Source);
                        return PointerIngressDisposition::Blocked;
                    }
                    // No source event was emitted below threshold. Preserve the
                    // admitted pair while replacing its pending recognizer.
                    self.interaction.gesture = None;
                }
                let Some(tokens) = self.interaction.pointer.ingress.touch_pair.tokens() else {
                    return PointerIngressDisposition::Invalid;
                };
                match self.start_touch_pair(ingress, geometry, tokens) {
                    Ok(capture) => {
                        self.interaction.gesture = Some(capture);
                        PointerIngressDisposition::AdmittedUnsupportedConsumer
                    }
                    Err(outcome) => {
                        self.interaction.pointer.ingress.touch_pair.clear();
                        touch_disposition(&outcome)
                    }
                }
            }
            TouchPairUpdate::Updated(geometry) | TouchPairUpdate::PairEnded(geometry) => self
                .advance_touch_pair(
                    ingress,
                    token,
                    geometry,
                    matches!(update, TouchPairUpdate::PairEnded(_)),
                ),
            TouchPairUpdate::Reset(reason) => {
                let owns_pair = self.interaction.gesture.as_ref().is_some_and(|capture| {
                    capture
                        .touch
                        .as_ref()
                        .is_some_and(|touch| Some(touch.tokens) == prior_tokens)
                });
                let owns_single = self
                    .interaction
                    .gesture
                    .as_ref()
                    .is_some_and(|capture| capture.single_touch.is_some());
                if owns_single
                    && reason == TouchPairReset::Terminal
                    && ingress.phase() != PointerPhase::Cancelled
                {
                    return self.route_single_touch_drag(ingress, token, cross_window);
                }
                if owns_pair || owns_single {
                    self.cancel_gesture_capture(if reason == TouchPairReset::InvalidSample {
                        GestureCancellation::InvalidSample
                    } else {
                        GestureCancellation::Source
                    });
                }
                PointerIngressDisposition::AdmittedUnsupportedConsumer
            }
        }
    }

    fn start_touch_pair(
        &mut self,
        ingress: PointerIngress,
        geometry: TouchPairGeometry,
        tokens: [PointerSequenceToken; 2],
    ) -> Result<GestureCapture, GestureOutcome> {
        if self.interaction.gesture.is_some() || self.gesture_has_incumbent() {
            return Err(GestureOutcome::Blocked);
        }
        let anchor = geometry.centroid;
        if self.layout_target_at(anchor).is_some() || self.scroll_affordance_at(anchor).is_some() {
            return Err(GestureOutcome::Unsupported);
        }
        let widget = self
            .widget_at_for_input(anchor, &WidgetInput::pointer_move(anchor))
            .ok_or(GestureOutcome::Unsupported)?;
        let current = self
            .surface_widget(widget)
            .ok_or(GestureOutcome::Unsupported)?;
        let common = current.widget_object().common();
        if common.state.disabled
            || common.state.read_only
            || self.accessibility_incumbent_owner(widget).is_some()
        {
            return Err(GestureOutcome::Blocked);
        }
        let hit_path = self
            .traversal
            .widgets
            .paths
            .current
            .get(&widget)
            .cloned()
            .ok_or(GestureOutcome::Unsupported)?;
        let mut candidates: Vec<GestureTarget> = Vec::new();
        let mut families: Vec<u8> = Vec::new();
        for (family, kind) in FAMILIES.into_iter().enumerate() {
            for target in self.gesture_candidates(widget, &hit_path, kind, anchor)? {
                // Pointer drag sources retain their primary-pointer contract.
                // Two contacts consume explicitly declared gesture handlers.
                if self.is_drag_source(&target) {
                    continue;
                }
                if let Some(index) = candidates
                    .iter()
                    .position(|candidate| candidate.id == target.id)
                {
                    if candidates[index].path != target.path {
                        return Err(GestureOutcome::Unsupported);
                    }
                    families[index] |= 1 << family;
                } else {
                    if candidates.len() == 64 {
                        return Err(GestureOutcome::Unsupported);
                    }
                    candidates.push(target);
                    families.push(1 << family);
                }
            }
        }
        let target = candidates
            .first()
            .cloned()
            .ok_or(GestureOutcome::Unsupported)?;
        let token = self
            .interaction
            .pointer
            .ingress
            .allocator
            .issue()
            .map_err(|_| GestureOutcome::Unavailable)?;
        Ok(GestureCapture {
            pointer_sequence: None,
            single_touch: None,
            target,
            candidates,
            hit_widget: widget,
            hit_path,
            generation: self.refresh_counters().runtime_projection,
            token: GestureSequenceToken(token),
            sample: touch_sample(
                ingress,
                geometry,
                GestureKind::Pan,
                GesturePhase::Started,
                Vector2::new(0.0, 0.0),
            )
            .ok_or(GestureOutcome::Invalid)?,
            anchor,
            accumulated: Vector2::new(0.0, 0.0),
            active: false,
            touch: Some(TouchGestureCapture {
                tokens,
                last: geometry,
                families,
            }),
        })
    }

    fn advance_touch_pair(
        &mut self,
        ingress: PointerIngress,
        pointer_token: PointerSequenceToken,
        geometry: TouchPairGeometry,
        terminal: bool,
    ) -> PointerIngressDisposition {
        if !self.interaction.gesture.as_ref().is_some_and(|capture| {
            capture
                .touch
                .as_ref()
                .is_some_and(|touch| touch.tokens.contains(&pointer_token))
        }) {
            return PointerIngressDisposition::Stale;
        }
        let Some(mut capture) = self.interaction.gesture.take() else {
            return PointerIngressDisposition::Stale;
        };
        if !self.gesture_capture_is_current(&capture) {
            self.finish_gesture_capture(capture, GestureCancellation::Retired);
            return PointerIngressDisposition::Stale;
        }
        let Some(touch) = capture.touch.as_ref() else {
            return PointerIngressDisposition::Stale;
        };
        let tokens = touch.tokens;
        let previous = touch.last;
        let was_active = capture.active;
        let kind = if was_active {
            capture.sample.kind()
        } else {
            let Some((index, kind)) = touch_choice(&capture, geometry) else {
                if terminal {
                    self.retire_touch_pointer_sequences(tokens);
                } else {
                    if let Some(touch) = capture.touch.as_mut() {
                        touch.last = geometry;
                    }
                    self.interaction.gesture = Some(capture);
                }
                return PointerIngressDisposition::AdmittedUnsupportedConsumer;
            };
            capture.target = capture.candidates[index].clone();
            capture.candidates = vec![capture.target.clone()];
            capture.accumulated = neutral(kind);
            let Some(sample) = touch_sample(
                ingress,
                geometry,
                kind,
                GesturePhase::Started,
                neutral(kind),
            ) else {
                self.finish_gesture_capture(capture, GestureCancellation::InvalidSample);
                return PointerIngressDisposition::Invalid;
            };
            capture.sample = sample;
            kind
        };
        let value = if was_active {
            match kind {
                GestureKind::Pan => Vector2::new(
                    geometry.pan.x - previous.pan.x,
                    geometry.pan.y - previous.pan.y,
                ),
                GestureKind::Pinch => Vector2::new(geometry.scale / previous.scale, 0.0),
                GestureKind::Rotate => {
                    let difference = geometry.rotation - previous.rotation;
                    Vector2::new(difference.sin().atan2(difference.cos()), 0.0)
                }
            }
        } else {
            geometry_value(geometry, kind)
        };
        let phase = if terminal {
            GesturePhase::Ended
        } else {
            GesturePhase::Changed
        };
        let Some(sample) = touch_sample(ingress, geometry, kind, phase, value) else {
            self.finish_gesture_capture(capture, GestureCancellation::InvalidSample);
            return PointerIngressDisposition::Invalid;
        };
        if let Some(touch) = capture.touch.as_mut() {
            touch.last = geometry;
        }
        let token = capture.token;
        self.interaction.gesture = Some(capture);
        let admission =
            self.dispatch_gesture_request(GestureRequest::new(sample).with_token(token));
        if admission.token().is_none() {
            if self
                .interaction
                .gesture
                .as_ref()
                .is_some_and(|capture| capture.token == token)
            {
                self.cancel_gesture_capture(GestureCancellation::Retired);
            }
            self.retire_touch_pointer_sequences(tokens);
        }
        touch_disposition(admission.outcome())
    }

    pub(super) fn retire_gesture_touch(&mut self, capture: &GestureCapture) {
        if capture.single_touch.is_some_and(|token| {
            self.interaction
                .pointer
                .ingress
                .touch_pair
                .contains_token(token)
        }) {
            // Retain transport tombstones until up; held contacts cannot form a
            // replacement gesture after cancellation or source retirement.
            self.interaction.pointer.ingress.touch_pair.clear();
        }
        if let Some(touch) = &capture.touch {
            self.retire_touch_pointer_sequences(touch.tokens);
        }
    }
}

fn touch_choice(
    capture: &GestureCapture,
    geometry: TouchPairGeometry,
) -> Option<(usize, GestureKind)> {
    let touch = capture.touch.as_ref()?;
    let mut best: Option<(usize, usize, usize, f64)> = None;
    for (index, target) in capture.candidates.iter().enumerate() {
        for (family, kind) in FAMILIES.into_iter().enumerate() {
            if touch
                .families
                .get(index)
                .is_none_or(|mask| mask & (1 << family) == 0)
            {
                continue;
            }
            let Some(threshold) = target.policy.threshold(kind) else {
                continue;
            };
            let magnitude = match kind {
                GestureKind::Pan => f64::from(geometry.pan.x).hypot(f64::from(geometry.pan.y)),
                GestureKind::Pinch => f64::from(geometry.scale_delta).abs(),
                GestureKind::Rotate => f64::from(geometry.rotation).abs(),
            };
            if magnitude < f64::from(threshold) {
                continue;
            }
            let score = if threshold > 0.0 {
                magnitude / f64::from(threshold)
            } else {
                f64::INFINITY
            };
            let depth = target.path.as_slice().len();
            if best.is_none_or(|(_, old_family, old_depth, old_score)| {
                depth > old_depth
                    || (depth == old_depth
                        && (score > old_score || (score == old_score && family < old_family)))
            }) {
                best = Some((index, family, depth, score));
            }
        }
    }
    best.map(|(index, family, _, _)| (index, FAMILIES[family]))
}
fn neutral(kind: GestureKind) -> Vector2 {
    Vector2::new(if kind == GestureKind::Pinch { 1.0 } else { 0.0 }, 0.0)
}
fn geometry_value(geometry: TouchPairGeometry, kind: GestureKind) -> Vector2 {
    match kind {
        GestureKind::Pan => geometry.pan,
        GestureKind::Pinch => Vector2::new(geometry.scale, 0.0),
        GestureKind::Rotate => Vector2::new(geometry.rotation, 0.0),
    }
}
fn touch_sample(
    ingress: PointerIngress,
    geometry: TouchPairGeometry,
    kind: GestureKind,
    phase: GesturePhase,
    value: Vector2,
) -> Option<GestureIngress> {
    let unit = match kind {
        GestureKind::Pan => GestureUnit::LogicalPixels,
        GestureKind::Pinch => GestureUnit::Scale,
        GestureKind::Rotate => GestureUnit::Radians,
    };
    GestureIngress::new(
        kind,
        phase,
        unit,
        value,
        ingress.device(),
        Some(geometry.centroid),
        ingress.modifiers(),
        ingress.timestamp(),
        ingress.sequence_range(),
    )
    .ok()
}
pub(super) fn touch_disposition(outcome: &GestureOutcome) -> PointerIngressDisposition {
    match outcome {
        GestureOutcome::Accepted(id) | GestureOutcome::AcceptedContainer(id) => {
            PointerIngressDisposition::RoutedGesture(*id)
        }
        GestureOutcome::Pending | GestureOutcome::Unrecognized | GestureOutcome::Unsupported => {
            PointerIngressDisposition::AdmittedUnsupportedConsumer
        }
        GestureOutcome::Stale => PointerIngressDisposition::Stale,
        GestureOutcome::Invalid => PointerIngressDisposition::Invalid,
        _ => PointerIngressDisposition::Blocked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::pointer_ingress::{PointerButtons, PointerContactId};
    use crate::widgets::PointerButton;
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn native_gesture_continuations_cannot_adopt_a_physical_touch_pair() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let observed = events.clone();
        let mut runtime = SurfaceRuntime::new(
            crate::app(())
                .view(|_| {
                    crate::application::button("Touch")
                        .filter_mapped(|_| None::<GestureEvent>)
                        .width(120.0)
                        .height(40.0)
                        .id(1)
                        .on_gesture_with_revision(
                            GesturePolicy::none()
                                .recognize(GestureKind::Pinch, 0.2)
                                .unwrap(),
                            (),
                            Some,
                        )
                        .id(10)
                })
                .update(move |_, event| observed.borrow_mut().push(event))
                .into_bridge(),
            Vector2::new(200.0, 80.0),
        );
        let device = crate::gui::pointer_ingress::InputDeviceId::from_host(2).unwrap();
        let input = |contact, phase: PointerPhase, x, token: Option<PointerSequenceToken>| {
            let contact = PointerContactId::from_host(contact).unwrap();
            let point = Point::new(x, 20.0);
            let buttons = if phase.is_terminal() {
                PointerButtons::empty()
            } else {
                PointerButtons::PRIMARY
            };
            match token {
                Some(token) => PointerIngress::from_runtime(
                    DeviceKind::Touch,
                    device,
                    contact,
                    phase,
                    point,
                    buttons,
                    Default::default(),
                    None,
                    None,
                    None,
                    None,
                    token,
                )
                .unwrap(),
                None => PointerIngress::new(
                    DeviceKind::Touch,
                    device,
                    contact,
                    phase,
                    point,
                    buttons,
                    Default::default(),
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap(),
            }
        };
        runtime.dispatch_pointer_ingress(input(
            1,
            PointerPhase::Started {
                button: PointerButton::Primary,
            },
            20.0,
            None,
        ));
        let token = runtime
            .dispatch_pointer_ingress_with_admission(input(
                2,
                PointerPhase::Started {
                    button: PointerButton::Primary,
                },
                60.0,
                None,
            ))
            .sequence_token()
            .unwrap();
        assert_eq!(
            runtime.dispatch_pointer_ingress(input(2, PointerPhase::Moved, 80.0, Some(token))),
            PointerIngressDisposition::RoutedGesture(10)
        );
        assert_eq!(runtime.retained_gesture_device(), None);
        for phase in [GesturePhase::Changed, GesturePhase::Ended] {
            let sample = GestureIngress::pinch(
                phase,
                1.1,
                device,
                Some(Point::new(40.0, 20.0)),
                Default::default(),
            )
            .unwrap();
            assert_eq!(
                runtime.dispatch_native_gesture_ingress(sample),
                GestureIngressDisposition::Stale
            );
            runtime.reject_native_gesture_continuation(device, GestureKind::Pinch, phase);
        }
        assert_eq!(events.borrow().len(), 1);
        runtime.dispatch_pointer_ingress(input(
            2,
            PointerPhase::Ended {
                button: PointerButton::Primary,
            },
            90.0,
            Some(token),
        ));
        assert_eq!(events.borrow().len(), 2);
        assert_eq!(events.borrow()[1].phase(), GesturePhase::Ended);
    }
}
