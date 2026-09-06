use super::*;
use radiant::{
    application::{DragSource, DropTarget, button, row},
    runtime::{
        DragCancelReason, DragOperation, DragOperations, DragSourcePhase, DropDecision, DropPhase,
    },
};
#[derive(Clone, Debug, PartialEq)]
enum Event {
    Source(DragSourcePhase),
    Target(DropPhase, DropDecision),
}
fn drag_bridge(
    events: Rc<RefCell<Vec<Event>>>,
    source_revision: Rc<Cell<u32>>,
    target_revision: Rc<Cell<u32>>,
    decision: Rc<Cell<DropDecision>>,
    remove_on_enter: bool,
    remove_on_drop: bool,
) -> impl radiant::runtime::RuntimeBridge<Event> {
    radiant::app(false)
        .view(move |removed| {
            let revision = source_revision.get();
            let source = button("Source")
                .filter_mapped(|_| None::<Event>)
                .width(100.0)
                .height(40.0)
                .id(1);
            if *removed {
                return row([
                    source,
                    button("Removed").filter_mapped(|_| None::<Event>).id(2),
                ])
                .id(30);
            }
            let source = source
                .drag_source(
                    DragSource::new(Rc::new(String::from("payload")))
                        .operations(DragOperations::only(DragOperation::Move))
                        .on_event_with_revision(revision, |event| {
                            assert_eq!(event.payload().as_str(), "payload");
                            Some(Event::Source(event.phase()))
                        }),
                )
                .id(10);
            let current_decision = decision.get();
            let target = button("Target")
                .filter_mapped(|_| None::<Event>)
                .width(100.0)
                .height(40.0)
                .id(2)
                .drop_target(
                    DropTarget::<Rc<String>, Event>::new()
                        .negotiate_with_revision(current_decision, move |_, _| current_decision)
                        .on_event_with_revision(target_revision.get(), |event| {
                            assert_eq!(event.payload().as_str(), "payload");
                            assert_eq!(event.context().source(), 10);
                            assert_eq!(event.context().target(), Some(20));
                            Some(Event::Target(event.phase(), event.decision()))
                        }),
                )
                .id(20);
            row([source, target]).spacing(0.0).id(30)
        })
        .update(move |removed, event| {
            if (remove_on_enter && matches!(event, Event::Target(DropPhase::Entered, _)))
                || (remove_on_drop && matches!(event, Event::Target(DropPhase::Dropped, _)))
            {
                *removed = true;
            }
            events.borrow_mut().push(event);
        })
        .into_bridge()
}
fn send<B: radiant::runtime::RuntimeBridge<Event>>(
    runtime: &mut SurfaceRuntime<B, Event>,
    phase: GesturePhase,
    dx: f32,
    token: radiant::runtime::GestureSequenceToken,
) -> radiant::runtime::GestureAdmission {
    runtime.dispatch_gesture_request(
        GestureRequest::new(sample(GestureKind::Pan, phase, Vector2::new(dx, 0.0)))
            .with_token(token),
    )
}
#[test]
fn typed_drag_moves_enters_and_delivers_detached_terminal_messages() {
    for remove_on_drop in [false, true] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let bridge = drag_bridge(
            events.clone(),
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(DropDecision::Accepted(DragOperation::Move))),
            false,
            remove_on_drop,
        );
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 80.0));
        let start = runtime.dispatch_gesture_request(GestureRequest::new(sample(
            GestureKind::Pan,
            GesturePhase::Started,
            Vector2::default(),
        )));
        let token = start.token().unwrap();
        assert_eq!(
            send(&mut runtime, GesturePhase::Changed, 7.0, token).outcome(),
            &GestureOutcome::AcceptedContainer(10)
        );
        assert!(runtime.drag_session_active());
        assert_eq!(*events.borrow(), [Event::Source(DragSourcePhase::Started)]);
        send(&mut runtime, GesturePhase::Changed, 100.0, token);
        assert!(events.borrow().contains(&Event::Target(
            DropPhase::Entered,
            DropDecision::Accepted(DragOperation::Move)
        )));
        let end = send(&mut runtime, GesturePhase::Ended, 0.0, token);
        assert_eq!(end.token(), None);
        assert!(!runtime.drag_session_active());
        assert!(events.borrow().ends_with(&[
            Event::Target(
                DropPhase::Dropped,
                DropDecision::Accepted(DragOperation::Move)
            ),
            Event::Source(DragSourcePhase::Completed(DragOperation::Move))
        ]));
        let count = events.borrow().len();
        assert_eq!(
            send(&mut runtime, GesturePhase::Ended, 0.0, token).outcome(),
            &GestureOutcome::Stale
        );
        assert_eq!(events.borrow().len(), count);
    }
}
#[test]
fn typed_drag_rejection_pending_and_disallowed_operations_never_drop() {
    for decision in [
        DropDecision::Rejected,
        DropDecision::Pending,
        DropDecision::Accepted(DragOperation::Copy),
    ] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let bridge = drag_bridge(
            events.clone(),
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(decision)),
            false,
            false,
        );
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 80.0));
        let token = runtime
            .dispatch_gesture_request(GestureRequest::new(sample(
                GestureKind::Pan,
                GesturePhase::Started,
                Vector2::default(),
            )))
            .token()
            .unwrap();
        send(&mut runtime, GesturePhase::Changed, 110.0, token);
        send(&mut runtime, GesturePhase::Ended, 0.0, token);
        assert!(
            !events
                .borrow()
                .iter()
                .any(|event| matches!(event, Event::Target(DropPhase::Dropped, _)))
        );
        assert_eq!(
            events.borrow().last(),
            Some(&Event::Source(DragSourcePhase::Cancelled(
                DragCancelReason::NoTarget
            )))
        );
    }
}
#[test]
fn typed_drag_source_retirement_during_enter_cancels_once() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let bridge = drag_bridge(
        events.clone(),
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(DropDecision::Accepted(DragOperation::Move))),
        true,
        false,
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 80.0));
    let token = runtime
        .dispatch_gesture_request(GestureRequest::new(sample(
            GestureKind::Pan,
            GesturePhase::Started,
            Vector2::default(),
        )))
        .token()
        .unwrap();
    let moved = send(&mut runtime, GesturePhase::Changed, 110.0, token);
    assert_eq!(moved.token(), None);
    assert!(!runtime.drag_session_active());
    assert_eq!(
        events
            .borrow()
            .iter()
            .filter(|event| matches!(
                event,
                Event::Source(DragSourcePhase::Cancelled(DragCancelReason::SourceRetired))
            ))
            .count(),
        1
    );
    assert!(
        !events
            .borrow()
            .iter()
            .any(|event| matches!(event, Event::Source(DragSourcePhase::Moved)))
    );
    assert_eq!(
        send(&mut runtime, GesturePhase::Ended, 0.0, token).outcome(),
        &GestureOutcome::Stale
    );
}
#[test]
fn typed_drag_target_revision_retires_hover_then_reenters_current_target() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let revision = Rc::new(Cell::new(0));
    let bridge = drag_bridge(
        events.clone(),
        Rc::new(Cell::new(0)),
        revision.clone(),
        Rc::new(Cell::new(DropDecision::Accepted(DragOperation::Move))),
        false,
        false,
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 80.0));
    let token = runtime
        .dispatch_gesture_request(GestureRequest::new(sample(
            GestureKind::Pan,
            GesturePhase::Started,
            Vector2::default(),
        )))
        .token()
        .unwrap();
    send(&mut runtime, GesturePhase::Changed, 110.0, token);
    revision.set(1);
    runtime.refresh();
    assert!(runtime.drag_session_active());
    assert_eq!(
        events.borrow().last(),
        Some(&Event::Target(
            DropPhase::Left,
            DropDecision::Accepted(DragOperation::Move)
        ))
    );
    send(&mut runtime, GesturePhase::Changed, 0.0, token);
    assert_eq!(
        events
            .borrow()
            .iter()
            .filter(|event| matches!(event, Event::Target(DropPhase::Entered, _)))
            .count(),
        2
    );
    send(&mut runtime, GesturePhase::Cancelled, 0.0, token);
    assert!(!runtime.drag_session_active());
    assert!(events.borrow().ends_with(&[
        Event::Target(
            DropPhase::Cancelled,
            DropDecision::Accepted(DragOperation::Move)
        ),
        Event::Source(DragSourcePhase::Cancelled(DragCancelReason::CaptureLost))
    ]));
}

#[test]
fn typed_drag_preview_motion_does_not_project_or_layout_and_end_command_retires_capture() {
    let bridge = radiant::app(())
        .view(|_| {
            button("Source")
                .filter_mapped(|_| None::<Event>)
                .width(100.0)
                .height(40.0)
                .id(1)
                .drag_source(DragSource::new(42u32))
                .id(10)
        })
        .update(|_, _| {})
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 80.0));
    let token = runtime
        .dispatch_gesture_request(GestureRequest::new(sample(
            GestureKind::Pan,
            GesturePhase::Started,
            Vector2::default(),
        )))
        .token()
        .unwrap();
    let before = runtime.refresh_counters();
    send(&mut runtime, GesturePhase::Changed, 7.0, token);
    send(&mut runtime, GesturePhase::Changed, 10.0, token);
    let after = runtime.refresh_counters();
    assert!(runtime.drag_session_active());
    assert_eq!(after.application_projection, before.application_projection);
    assert_eq!(after.runtime_projection, before.runtime_projection);
    assert_eq!(after.layout, before.layout);
    runtime.execute_command(Command::end_drag());
    assert!(!runtime.drag_session_active());
    assert_eq!(
        send(&mut runtime, GesturePhase::Ended, 0.0, token).outcome(),
        &GestureOutcome::Stale
    );
}
#[test]
fn typed_drag_conservative_source_cancels_when_started_reprojects() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let output = events.clone();
    let bridge = radiant::app(())
        .view(|_| {
            button("Source")
                .filter_mapped(|_| None::<Event>)
                .width(100.0)
                .height(40.0)
                .id(1)
                .drag_source(
                    DragSource::new(42u32).on_event(|event| Some(Event::Source(event.phase()))),
                )
                .id(10)
        })
        .update(move |_, event| output.borrow_mut().push(event))
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 80.0));
    let token = runtime
        .dispatch_gesture_request(GestureRequest::new(sample(
            GestureKind::Pan,
            GesturePhase::Started,
            Vector2::default(),
        )))
        .token()
        .unwrap();
    assert_eq!(
        send(&mut runtime, GesturePhase::Changed, 7.0, token).token(),
        None
    );
    assert!(!runtime.drag_session_active());
    assert_eq!(
        *events.borrow(),
        [
            Event::Source(DragSourcePhase::Started),
            Event::Source(DragSourcePhase::Cancelled(DragCancelReason::SourceRetired))
        ]
    );
}
#[test]
fn typed_drag_crossing_threshold_on_terminal_delivers_one_complete_transaction() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let bridge = drag_bridge(
        events.clone(),
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(0)),
        Rc::new(Cell::new(DropDecision::Accepted(DragOperation::Move))),
        false,
        false,
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 80.0));
    let token = runtime
        .dispatch_gesture_request(GestureRequest::new(sample(
            GestureKind::Pan,
            GesturePhase::Started,
            Vector2::default(),
        )))
        .token()
        .unwrap();
    assert_eq!(
        send(&mut runtime, GesturePhase::Ended, 110.0, token).token(),
        None
    );
    assert!(!runtime.drag_session_active());
    assert_eq!(
        events
            .borrow()
            .iter()
            .filter(|event| matches!(event, Event::Source(DragSourcePhase::Started)))
            .count(),
        1
    );
    assert!(events.borrow().ends_with(&[
        Event::Target(
            DropPhase::Dropped,
            DropDecision::Accepted(DragOperation::Move)
        ),
        Event::Source(DragSourcePhase::Completed(DragOperation::Move))
    ]));
}

#[test]
fn typed_drag_targets_obey_clipping_and_topmost_widgets_but_accept_empty_regions() {
    for mode in 0..3 {
        let events = Rc::new(RefCell::new(Vec::new()));
        let output = events.clone();
        let bridge = radiant::app(())
            .view(move |_| {
                let source = button("Source")
                    .filter_mapped(|_| None::<Event>)
                    .width(100.0)
                    .height(40.0)
                    .id(1)
                    .drag_source(
                        DragSource::new(42u32)
                            .on_event_with_revision((), |event| Some(Event::Source(event.phase()))),
                    )
                    .id(10);
                let target = radiant::application::empty()
                    .width(100.0)
                    .height(100.0)
                    .id(2)
                    .drop_target(
                        DropTarget::<u32, Event>::new().on_event_with_revision((), |event| {
                            Some(Event::Target(event.phase(), event.decision()))
                        }),
                    )
                    .width(100.0)
                    .height(100.0)
                    .id(20);
                let target = match mode {
                    1 => radiant::application::stack([
                        target,
                        button("Cover")
                            .filter_mapped(|_| None::<Event>)
                            .width(100.0)
                            .height(100.0)
                            .id(3),
                    ])
                    .id(21),
                    2 => radiant::application::scroll(target)
                        .width(100.0)
                        .height(20.0)
                        .id(21),
                    _ => target,
                };
                row([source, target])
                    .spacing(0.0)
                    .align_cross(radiant::layout::CrossAlign::Start)
                    .id(30)
            })
            .update(move |_, event| output.borrow_mut().push(event))
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 120.0));
        let token = runtime
            .dispatch_gesture_request(GestureRequest::new(sample(
                GestureKind::Pan,
                GesturePhase::Started,
                Vector2::default(),
            )))
            .token()
            .unwrap();
        runtime.dispatch_gesture_request(
            GestureRequest::new(sample(
                GestureKind::Pan,
                GesturePhase::Changed,
                Vector2::new(110.0, 20.0),
            ))
            .with_token(token),
        );
        send(&mut runtime, GesturePhase::Ended, 0.0, token);
        assert_eq!(
            events
                .borrow()
                .iter()
                .any(|event| matches!(event, Event::Target(DropPhase::Dropped, _))),
            mode == 0,
            "mode {mode}: {:?}",
            events.borrow()
        );
    }
}
