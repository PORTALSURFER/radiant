//! Single-contact typed drag recognition in the existing gesture arena.
use super::*;
use crate::gui::pointer_ingress::{
    GestureUnit, PointerIngress, PointerIngressDisposition, PointerPhase,
};

impl<Bridge: RuntimeBridge<Message>, Message> SurfaceRuntime<Bridge, Message> {
    pub(super) fn route_single_touch_drag(
        &mut self,
        ingress: PointerIngress,
        pointer_token: PointerSequenceToken,
    ) -> PointerIngressDisposition {
        if matches!(ingress.phase(), PointerPhase::Started { .. }) {
            return match self.prepare_single_touch_drag(ingress, pointer_token) {
                Ok(capture) => {
                    self.interaction.gesture = Some(capture);
                    PointerIngressDisposition::AdmittedUnsupportedConsumer
                }
                Err(outcome) => touch::touch_disposition(&outcome),
            };
        }
        let Some(capture) = self
            .interaction
            .gesture
            .as_ref()
            .filter(|capture| capture.single_touch == Some(pointer_token))
        else {
            return PointerIngressDisposition::AdmittedUnsupportedConsumer;
        };
        let Some(record) = self
            .interaction
            .pointer
            .ingress
            .records
            .iter()
            .flatten()
            .find(|record| record.token == pointer_token)
        else {
            return PointerIngressDisposition::Stale;
        };
        let phase = match ingress.phase() {
            PointerPhase::Moved => GesturePhase::Changed,
            PointerPhase::Ended { .. } => GesturePhase::Ended,
            PointerPhase::Cancelled => GesturePhase::Cancelled,
            _ => return PointerIngressDisposition::AdmittedUnsupportedConsumer,
        };
        let delta = Vector2::new(
            ingress.logical_position().x - record.last_position.x,
            ingress.logical_position().y - record.last_position.y,
        );
        let token = capture.token;
        let Some(sample) = single_touch_sample(ingress, phase, delta, capture.anchor) else {
            self.cancel_gesture_capture(GestureCancellation::InvalidSample);
            return PointerIngressDisposition::Invalid;
        };
        let admission =
            self.dispatch_gesture_request(GestureRequest::new(sample).with_token(token));
        if admission.token().is_none()
            && self
                .interaction
                .pointer
                .ingress
                .touch_pair
                .contains_token(pointer_token)
        {
            self.interaction.pointer.ingress.touch_pair.clear();
        }
        touch::touch_disposition(admission.outcome())
    }

    fn prepare_single_touch_drag(
        &mut self,
        ingress: PointerIngress,
        pointer_token: PointerSequenceToken,
    ) -> Result<GestureCapture, GestureOutcome> {
        let anchor = ingress.logical_position();
        if self.layout_target_at(anchor).is_some() || self.scroll_affordance_at(anchor).is_some() {
            return Err(GestureOutcome::Unsupported);
        }
        let hit_widget = self
            .widget_at_for_input(anchor, &WidgetInput::pointer_move(anchor))
            .ok_or(GestureOutcome::Unsupported)?;
        let current = self
            .surface_widget(hit_widget)
            .ok_or(GestureOutcome::Unsupported)?;
        let common = current.widget_object().common();
        let blocked = common.state.disabled
            || common.state.read_only
            || self.accessibility_incumbent_owner(hit_widget).is_some();
        let hit_path = self
            .traversal
            .widgets
            .paths
            .current
            .get(&hit_widget)
            .cloned()
            .ok_or(GestureOutcome::Unsupported)?;
        let candidates = self.gesture_candidates_filtered(
            hit_widget,
            &hit_path,
            GestureKind::Pan,
            anchor,
            true,
        )?;
        let target = candidates
            .first()
            .cloned()
            .ok_or(GestureOutcome::Unsupported)?;
        if blocked || self.interaction.gesture.is_some() || self.gesture_has_incumbent() {
            return Err(GestureOutcome::Blocked);
        }
        let token = GestureSequenceToken(
            self.interaction
                .pointer
                .ingress
                .allocator
                .issue()
                .map_err(|_| GestureOutcome::Unavailable)?,
        );
        Ok(GestureCapture {
            pointer_sequence: None,
            single_touch: Some(pointer_token),
            target,
            candidates,
            hit_widget,
            hit_path,
            generation: self.refresh_counters().runtime_projection,
            token,
            sample: single_touch_sample(ingress, GesturePhase::Started, Vector2::default(), anchor)
                .ok_or(GestureOutcome::Invalid)?,
            anchor,
            accumulated: Vector2::default(),
            active: false,
            touch: None,
        })
    }
}

fn single_touch_sample(
    ingress: PointerIngress,
    phase: GesturePhase,
    delta: Vector2,
    anchor: Point,
) -> Option<GestureIngress> {
    GestureIngress::new(
        GestureKind::Pan,
        phase,
        GestureUnit::LogicalPixels,
        delta,
        ingress.device(),
        Some(anchor),
        ingress.modifiers(),
        ingress.timestamp(),
        ingress.sequence_range(),
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{DragSource, render_canvas_pointer},
        gui::pointer_ingress::{
            DeviceKind, InputDeviceId, PointerButtons, PointerContactId, PointerEvent,
        },
        runtime::{DragSourcePhase, RenderCanvasContent},
        widgets::PointerButton,
    };
    use std::{cell::RefCell, rc::Rc, sync::Arc};
    #[derive(Debug)]
    enum Message {
        Pointer(PointerEvent),
        Drag(DragSourcePhase),
    }

    #[test]
    fn single_touch_neither_adopts_native_gestures_nor_synthesizes_child_capture() {
        for active in [false, true] {
            let events = Rc::new(RefCell::new(Vec::new()));
            let observed = events.clone();
            let mut runtime = SurfaceRuntime::new(
                crate::app(())
                    .view(|_| {
                        render_canvas_pointer(
                            1,
                            0,
                            RenderCanvasContent::SignalBands {
                                frames: 1,
                                band_count: 1,
                                frame_range: [0.0, 1.0],
                                samples: Arc::from([0.0]),
                            },
                            Message::Pointer,
                        )
                        .size(120.0, 40.0)
                        .id(1)
                        .drag_source(
                            DragSource::new(Rc::new(42u32)).on_event_with_revision((), |event| {
                                Some(Message::Drag(event.phase()))
                            }),
                        )
                        .id(10)
                    })
                    .update(move |_, event| observed.borrow_mut().push(event))
                    .into_bridge(),
                Vector2::new(200.0, 80.0),
            );
            let device = InputDeviceId::from_host(2).unwrap();
            let contact = PointerContactId::from_host(1).unwrap();
            let start = PointerIngress::new(
                DeviceKind::Touch,
                device,
                contact,
                PointerPhase::Started {
                    button: PointerButton::Primary,
                },
                Point::new(20.0, 20.0),
                PointerButtons::PRIMARY,
                Default::default(),
                None,
                None,
                None,
                None,
            )
            .unwrap();
            let token = runtime
                .dispatch_pointer_ingress_with_admission(start)
                .sequence_token()
                .unwrap();
            let input = |phase, x| {
                PointerIngress::from_runtime(
                    DeviceKind::Touch,
                    device,
                    contact,
                    phase,
                    Point::new(x, 20.0),
                    if phase.is_terminal() {
                        PointerButtons::empty()
                    } else {
                        PointerButtons::PRIMARY
                    },
                    Default::default(),
                    None,
                    None,
                    None,
                    None,
                    token,
                )
                .unwrap()
            };
            if active {
                runtime.dispatch_pointer_ingress(input(PointerPhase::Moved, 50.0));
            }
            assert_eq!(runtime.interaction.pointer.capture, None);
            assert_eq!(runtime.retained_gesture_device(), None);
            let count = events.borrow().len();
            for kind in [GestureKind::Pan, GestureKind::Pinch] {
                for phase in [GesturePhase::Changed, GesturePhase::Ended] {
                    let sample = if kind == GestureKind::Pan {
                        GestureIngress::pan(
                            phase,
                            Vector2::new(10.0, 0.0),
                            device,
                            Some(Point::new(20.0, 20.0)),
                            Default::default(),
                        )
                    } else {
                        GestureIngress::pinch(
                            phase,
                            1.1,
                            device,
                            Some(Point::new(20.0, 20.0)),
                            Default::default(),
                        )
                    }
                    .unwrap();
                    assert_eq!(
                        runtime.dispatch_native_gesture_ingress(sample),
                        GestureIngressDisposition::Stale
                    );
                    runtime.reject_native_gesture_continuation(device, kind, phase);
                }
            }
            assert_eq!(events.borrow().len(), count);
            runtime.dispatch_pointer_ingress(input(
                PointerPhase::Ended {
                    button: PointerButton::Primary,
                },
                60.0,
            ));
            assert_eq!(runtime.interaction.pointer.capture, None);
            assert!(events.borrow().iter().all(|event| match event {
                Message::Pointer(event) => panic!("synthetic child pointer: {event:?}"),
                Message::Drag(_) => true,
            }));
            assert_eq!(
                events
                    .borrow()
                    .iter()
                    .filter(|event| matches!(event, Message::Drag(DragSourcePhase::Started)))
                    .count(),
                1
            );
            assert_eq!(
                events
                    .borrow()
                    .iter()
                    .filter(|event| matches!(event, Message::Drag(DragSourcePhase::Cancelled(_))))
                    .count(),
                1
            );
        }
    }
}
