//! Headless recognition, capture and cancellation using the public gesture contract.
use radiant::prelude::*;
use radiant::{
    application::custom_widget_mapped,
    gui::{
        pointer_ingress::*,
        types::{Point, Rect},
    },
    runtime::{GestureOutcome, GestureRequest, PaintPrimitive, SurfaceRuntime},
    widgets::{
        GestureEvent, GesturePolicy, Widget, WidgetActionCapabilities, WidgetCapabilitiesV2,
        WidgetCommon, WidgetGestures, WidgetInput, WidgetOutput, WidgetSemanticsRevision,
    },
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
struct Pad {
    common: WidgetCommon,
}
impl WidgetGestures for Pad {
    fn revision(&self) -> WidgetSemanticsRevision {
        WidgetSemanticsRevision::exact(self.policy())
    }
    fn policy(&self) -> GesturePolicy {
        GesturePolicy::none()
            .recognize(GestureKind::Pan, 4.0)
            .unwrap()
    }
    fn dispatch(&mut self, event: GestureEvent) -> Option<WidgetOutput> {
        Some(WidgetOutput::typed(event))
    }
}
impl Widget for Pad {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
    fn capabilities_v2(&self) -> WidgetCapabilitiesV2<'_> {
        WidgetCapabilitiesV2::new().with_gestures(self)
    }
    fn action_capabilities(&mut self) -> WidgetActionCapabilities<'_> {
        WidgetActionCapabilities::none().with_gestures(self)
    }
    fn handle_input(&mut self, _: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        if let WidgetInput::FocusChanged(focused) = input {
            self.common.state.focused = focused;
        }
        None
    }
    fn append_paint(
        &self,
        _: &mut Vec<PaintPrimitive>,
        _: Rect,
        _: &radiant::layout::LayoutOutput,
        _: &radiant::theme::ThemeTokens,
    ) {
    }
}
fn exercise(delta: f32, expected: GestureOutcome) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&events);
    let mut runtime = SurfaceRuntime::new(
        app(())
            .view(|_: &()| {
                custom_widget_mapped(
                    Pad {
                        common: WidgetCommon::fixed(1, 120.0, 40.0).with_keyboard_focus(),
                    },
                    |event: GestureEvent| event,
                )
                .id(1)
                .on_gesture_with_revision(
                    GesturePolicy::none()
                        .recognize(GestureKind::Pan, 2.0)
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
    let sample = |phase, x| {
        GestureIngress::pan(
            phase,
            Vector2::new(x, 0.0),
            InputDeviceId::from_host(1).unwrap(),
            Some(Point::new(20.0, 15.0)),
            Default::default(),
        )
        .unwrap()
    };
    let start =
        runtime.dispatch_gesture_request(GestureRequest::new(sample(GesturePhase::Started, 0.0)));
    assert_eq!(start.outcome(), &GestureOutcome::Pending);
    let token = start.token().unwrap();
    assert_eq!(runtime.focused_widget(), None);
    let moved = GestureRequest::new(sample(GesturePhase::Changed, delta)).with_token(token);
    assert_eq!(runtime.dispatch_gesture_request(moved).outcome(), &expected);
    let end = runtime.dispatch_gesture_request(
        GestureRequest::new(sample(GesturePhase::Ended, 0.0)).with_token(token),
    );
    assert!(end.token().is_none());
    assert_eq!(
        events
            .borrow()
            .iter()
            .map(|event| event.phase())
            .collect::<Vec<_>>(),
        [GesturePhase::Started, GesturePhase::Ended]
    );
    assert_eq!(
        runtime.dispatch_gesture_request(moved).outcome(),
        &GestureOutcome::Stale
    );
}
#[derive(Clone, Debug)]
enum DragMessage {
    Source(radiant::runtime::DragSourcePhase),
    Target(radiant::runtime::DropPhase),
}
fn exercise_drag() {
    use radiant::{
        application::{DragSource, DropTarget},
        runtime::{DragSourcePhase, DropPhase},
    };
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let mut runtime = SurfaceRuntime::new(
        app(())
            .view(|_| {
                row([
                    button("Sample")
                        .filter_mapped(|_| None::<DragMessage>)
                        .width(100.0)
                        .height(40.0)
                        .id(1)
                        .drag_source(
                            DragSource::new(String::from("sample.wav"))
                                .on_event_with_revision((), |event| {
                                    Some(DragMessage::Source(event.phase()))
                                }),
                        )
                        .id(10),
                    button("Track")
                        .filter_mapped(|_| None::<DragMessage>)
                        .width(100.0)
                        .height(40.0)
                        .id(2)
                        .drop_target(
                            DropTarget::<String, DragMessage>::new().on_event_with_revision(
                                (),
                                |event| {
                                    assert_eq!(event.payload(), "sample.wav");
                                    Some(DragMessage::Target(event.phase()))
                                },
                            ),
                        )
                        .id(20),
                ])
                .spacing(0.0)
                .id(30)
            })
            .update(move |_, event| observed.borrow_mut().push(event))
            .into_bridge(),
        Vector2::new(240.0, 80.0),
    );
    let sample = |phase, x| {
        GestureIngress::pan(
            phase,
            Vector2::new(x, 0.0),
            InputDeviceId::from_host(1).unwrap(),
            Some(Point::new(20.0, 15.0)),
            Default::default(),
        )
        .unwrap()
    };
    let token = runtime
        .dispatch_gesture_request(GestureRequest::new(sample(GesturePhase::Started, 0.0)))
        .token()
        .unwrap();
    runtime.dispatch_gesture_request(
        GestureRequest::new(sample(GesturePhase::Changed, 110.0)).with_token(token),
    );
    assert!(runtime.drag_session_active());
    runtime.dispatch_gesture_request(
        GestureRequest::new(sample(GesturePhase::Ended, 0.0)).with_token(token),
    );
    assert!(!runtime.drag_session_active());
    assert!(
        events
            .borrow()
            .iter()
            .any(|event| matches!(event, DragMessage::Target(DropPhase::Dropped)))
    );
    assert!(matches!(
        events.borrow().last(),
        Some(DragMessage::Source(DragSourcePhase::Completed(_)))
    ));
}
fn main() {
    exercise(5.0, GestureOutcome::Accepted(1));
    exercise(2.0, GestureOutcome::AcceptedContainer(10));
    exercise_drag();
    println!(
        "Recognized child and ancestor gestures, dropped a typed payload and rejected stale continuations."
    );
}
#[cfg(test)]
mod tests {
    #[test]
    fn public_gesture_example_has_one_qualified_lifecycle() {
        super::exercise(5.0, super::GestureOutcome::Accepted(1));
        super::exercise(2.0, super::GestureOutcome::AcceptedContainer(10));
    }
    #[test]
    fn public_typed_drag_example_completes_through_update() {
        super::exercise_drag();
    }
}
