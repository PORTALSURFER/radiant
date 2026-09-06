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
fn exercise() {
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
    let moved = GestureRequest::new(sample(GesturePhase::Changed, 5.0)).with_token(token);
    assert_eq!(
        runtime.dispatch_gesture_request(moved).outcome(),
        &GestureOutcome::Accepted(1)
    );
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
fn main() {
    exercise();
    println!("Recognized one pan gesture and rejected a replayed continuation.");
}
#[cfg(test)]
mod tests {
    #[test]
    fn public_gesture_example_has_one_qualified_lifecycle() {
        super::exercise();
    }
}
