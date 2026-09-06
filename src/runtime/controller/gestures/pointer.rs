//! Pointer-derived pan recognition observes one admitted child press and transfers its capture.
use super::*;
use crate::gui::pointer_ingress::{
    DeviceKind, GestureUnit, PointerEvent, PointerIngress, PointerIngressDisposition, PointerPhase,
};
use crate::runtime::controller::pointer_ingress::{PointerOwnerWitness, PointerSequenceRecord};
use crate::widgets::PointerButton;

impl<Bridge: RuntimeBridge<Message>, Message> SurfaceRuntime<Bridge, Message> {
    pub(in crate::runtime::controller) fn prepare_pointer_gesture(
        &mut self,
        ingress: PointerIngress,
    ) -> Option<GestureCapture> {
        if ingress.kind() != DeviceKind::Mouse
            || ingress.phase()
                != (PointerPhase::Started {
                    button: PointerButton::Primary,
                })
            || self.interaction.gesture.is_some()
            || self.gesture_has_incumbent()
        {
            return None;
        }
        let anchor = ingress.logical_position();
        if self.layout_target_at(anchor).is_some() || self.scroll_affordance_at(anchor).is_some() {
            return None;
        }
        let hit_widget = self.widget_at_for_input(anchor, &WidgetInput::pointer_move(anchor))?;
        let hit_path = self
            .traversal
            .widgets
            .paths
            .current
            .get(&hit_widget)?
            .clone();
        let current = self.surface_widget(hit_widget)?;
        let common = current.widget_object().common();
        if common.state.disabled
            || common.state.read_only
            || self.accessibility_incumbent_owner(hit_widget).is_some()
        {
            return None;
        }
        let candidates = self
            .gesture_candidates(hit_widget, &hit_path, GestureKind::Pan, anchor)
            .ok()?;
        let target = candidates.first()?.clone();
        let sample =
            pointer_pan_sample(ingress, GesturePhase::Started, Vector2::default(), anchor)?;
        let token = GestureSequenceToken(self.interaction.pointer.ingress.allocator.issue().ok()?);
        Some(GestureCapture {
            target,
            candidates,
            hit_widget,
            hit_path,
            generation: self.refresh_counters().runtime_projection,
            token,
            sample,
            anchor,
            accumulated: Vector2::default(),
            active: false,
            pointer_sequence: None,
        })
    }
    pub(in crate::runtime::controller) fn install_pointer_gesture(
        &mut self,
        candidate: Option<GestureCapture>,
        index: usize,
        token: Option<PointerSequenceToken>,
    ) {
        let Some(mut capture) = candidate else {
            return;
        };
        let Some(record) = self.interaction.pointer.ingress.records[index] else {
            return;
        };
        if token != Some(record.token)
            || self.interaction.gesture.is_some()
            || !matches!(record.owner, Some(PointerOwnerWitness::Widget { id, .. }) if id == capture.hit_widget)
            || !self.pointer_widget_witness_is_current(record.owner)
            || self
                .accessibility_incumbent_owner(capture.hit_widget)
                .is_some()
            || !self.gesture_capture_is_current(&capture)
            || !self.gesture_pending_target_is_current(&capture)
        {
            return;
        }
        capture.pointer_sequence = Some(record.token);
        if let Some(record) = self.interaction.pointer.ingress.records[index].as_mut() {
            record.gesture_token = Some(capture.token);
        }
        self.interaction.gesture = Some(capture);
    }
    fn retire_pointer_gesture_record(&mut self, index: usize, token: PointerSequenceToken) {
        if let Some(slot) = self.interaction.pointer.ingress.records.get_mut(index)
            && slot.is_some_and(|record| record.token == token)
        {
            *slot = None;
        }
    }
    pub(in crate::runtime::controller) fn pointer_gesture_is_current(
        &self,
        record: PointerSequenceRecord,
    ) -> bool {
        self.interaction.gesture.as_ref().is_some_and(|capture| {
            Some(capture.token) == record.gesture_token
                && capture.pointer_sequence == Some(record.token)
        })
    }
    pub(in crate::runtime::controller) fn route_pointer_gesture(
        &mut self,
        ingress: PointerIngress,
        index: usize,
        record: PointerSequenceRecord,
    ) -> Option<PointerIngressDisposition> {
        let token = record.gesture_token?;
        if !self.pointer_gesture_is_current(record) {
            return if matches!(
                record.owner,
                Some(PointerOwnerWitness::Gesture | PointerOwnerWitness::GestureTransfer)
            ) {
                self.retire_pointer_gesture_record(index, record.token);
                Some(PointerIngressDisposition::Stale)
            } else {
                None
            };
        }
        let capture = self.interaction.gesture.as_ref()?;
        let active = capture.active;
        if !active && ingress.phase() == PointerPhase::Cancelled {
            self.cancel_gesture_capture(GestureCancellation::Source);
            return None;
        }
        let delta = Vector2::new(
            ingress.logical_position().x - record.last_position.x,
            ingress.logical_position().y - record.last_position.y,
        );
        let accumulated = Vector2::new(
            capture.accumulated.x + delta.x,
            capture.accumulated.y + delta.y,
        );
        let phase = match ingress.phase() {
            PointerPhase::Moved => GesturePhase::Changed,
            PointerPhase::Ended { .. } => GesturePhase::Ended,
            PointerPhase::Cancelled => GesturePhase::Cancelled,
            _ => return None,
        };
        let sample = pointer_pan_sample(ingress, phase, delta, capture.anchor);
        let Some(sample) = sample else {
            self.cancel_gesture_capture(GestureCancellation::InvalidSample);
            if active {
                self.retire_pointer_gesture_record(index, record.token);
                return Some(PointerIngressDisposition::Invalid);
            }
            return None;
        };
        let crossing = !active
            && capture
                .recognition_target(accumulated, GestureKind::Pan)
                .is_some();
        if crossing {
            if !self.pointer_widget_witness_is_current(record.owner)
                || self.gesture_has_non_pointer_incumbent()
                || self
                    .accessibility_incumbent_owner(capture.hit_widget)
                    .is_some()
            {
                self.cancel_gesture_capture(GestureCancellation::CaptureLost);
                return None;
            }
            let capture = self.interaction.gesture.take()?;
            // The original child loses authority before its cancellation mapper.
            // Keep the pointer token as an inert tombstone until transfer completes.
            if let Some(record) = self.interaction.pointer.ingress.records[index].as_mut() {
                record.owner = Some(PointerOwnerWitness::GestureTransfer);
            }
            let event = self
                .surface
                .widget_has_pointer_mapper(capture.hit_widget)
                .then(|| {
                    PointerIngress::from_runtime(
                        record.kind,
                        record.device,
                        record.contact,
                        PointerPhase::Cancelled,
                        ingress.logical_position(),
                        ingress.buttons(),
                        ingress.modifiers(),
                        ingress.pressure(),
                        ingress.tilt(),
                        ingress.timestamp(),
                        ingress.sequence_range(),
                        record.token,
                    )
                    .ok()
                    .map(|cancel| PointerEvent::from_ingress(cancel, Some(record.token)))
                })
                .flatten();
            self.cancel_pointer_capture_with_delivery(event);
            if !self.interaction.pointer.ingress.records[index].is_some_and(|current| {
                current.token == record.token
                    && current.owner == Some(PointerOwnerWitness::GestureTransfer)
            }) || self.interaction.gesture.is_some()
                || !self.gesture_capture_is_current(&capture)
                || !self.gesture_pending_target_is_current(&capture)
                || self.gesture_has_incumbent()
            {
                self.retire_pointer_gesture_record(index, record.token);
                return Some(PointerIngressDisposition::Stale);
            }
            if let Some(record) = self.interaction.pointer.ingress.records[index].as_mut() {
                record.owner = Some(PointerOwnerWitness::Gesture);
            }
            self.interaction.gesture = Some(capture);
        }
        let admission =
            self.dispatch_gesture_request(GestureRequest::new(sample).with_token(token));
        if !active && !crossing {
            return None;
        }
        let disposition = match admission.outcome() {
            GestureOutcome::Accepted(id) | GestureOutcome::AcceptedContainer(id) => {
                PointerIngressDisposition::RoutedGesture(*id)
            }
            GestureOutcome::Invalid => PointerIngressDisposition::Invalid,
            GestureOutcome::Stale => PointerIngressDisposition::Stale,
            _ => PointerIngressDisposition::Blocked,
        };
        if ingress.phase().is_terminal() || !self.pointer_gesture_is_current(record) {
            self.retire_pointer_gesture_record(index, record.token);
        } else if let Some(current) = self.interaction.pointer.ingress.records[index]
            .as_mut()
            .filter(|current| current.token == record.token)
        {
            current.last_position = ingress.logical_position();
            current.last_buttons = ingress.buttons();
        }
        Some(disposition)
    }
}
fn pointer_pan_sample(
    ingress: PointerIngress,
    phase: GesturePhase,
    value: Vector2,
    anchor: Point,
) -> Option<GestureIngress> {
    GestureIngress::new(
        GestureKind::Pan,
        phase,
        GestureUnit::LogicalPixels,
        value,
        ingress.device(),
        Some(anchor),
        ingress.modifiers(),
        ingress.timestamp(),
        ingress.sequence_range(),
    )
    .ok()
}
