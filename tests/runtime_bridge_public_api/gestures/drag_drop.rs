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

#[test]
fn modal_drag_cannot_negotiate_with_background_drop_target() {
    use radiant::application::{Layer, scene};
    use radiant::layout::OverlayAnchor;
    for modal in [false, true] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let received = events.clone();
        let bridge = radiant::app(())
            .view(move |_| {
                let target = button("Background")
                    .filter_mapped(|_| None::<Event>)
                    .id(2)
                    .drop_target(
                        DropTarget::<String, Event>::new().on_event_with_revision((), |event| {
                            Some(Event::Target(event.phase(), event.decision()))
                        }),
                    )
                    .id(20);
                let source = button("Modal source")
                    .filter_mapped(|_| None::<Event>)
                    .id(1)
                    .drag_source(
                        DragSource::new(String::from("payload"))
                            .on_event_with_revision((), |event| Some(Event::Source(event.phase()))),
                    )
                    .id(10);
                scene(target)
                    .layer(
                        Layer::modal(source)
                            .focus_policy(if modal {
                                radiant::runtime::OverlayFocusPolicy::Modal
                            } else {
                                radiant::runtime::OverlayFocusPolicy::None
                            })
                            .pass_through()
                            .anchored_to(OverlayAnchor::below(2, Vector2::new(100.0, 40.0))),
                    )
                    .into_view()
            })
            .update(move |_, event| received.borrow_mut().push(event))
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 80.0));
        let anchor = runtime.layout().rects[&1].center();
        let request = |phase, x| {
            GestureIngress::pan(
                phase,
                Vector2::new(x, 0.0),
                InputDeviceId::from_host(1).unwrap(),
                Some(anchor),
                Default::default(),
            )
            .unwrap()
        };
        let token = runtime
            .dispatch_gesture_request(GestureRequest::new(request(GesturePhase::Started, 0.0)))
            .token()
            .unwrap();
        let moved = runtime.dispatch_gesture_request(
            GestureRequest::new(request(GesturePhase::Changed, 130.0)).with_token(token),
        );
        assert_eq!(moved.outcome(), &GestureOutcome::AcceptedContainer(10));
        runtime.dispatch_gesture_request(
            GestureRequest::new(request(GesturePhase::Ended, 0.0)).with_token(token),
        );
        if modal {
            assert!(
                events
                    .borrow()
                    .iter()
                    .all(|event| !matches!(event, Event::Target(..))),
                "{:?}",
                events.borrow()
            );
            assert!(
                events
                    .borrow()
                    .contains(&Event::Source(DragSourcePhase::Cancelled(
                        DragCancelReason::NoTarget
                    )))
            );
        } else {
            assert!(events.borrow().contains(&Event::Target(
                DropPhase::Dropped,
                DropDecision::Accepted(DragOperation::Copy)
            )));
            assert!(
                events
                    .borrow()
                    .contains(&Event::Source(DragSourcePhase::Completed(
                        DragOperation::Copy
                    )))
            );
        }
    }
}

#[test]
fn single_touch_drag_preserves_typed_payload_and_terminal_threshold() {
    use radiant::gui::pointer_ingress::PointerPhase;
    use radiant::widgets::PointerButton;
    for mode in 0..3 {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = SurfaceRuntime::new(
            drag_bridge(
                events.clone(),
                Rc::new(Cell::new(0)),
                Rc::new(Cell::new(0)),
                Rc::new(Cell::new(DropDecision::Accepted(DragOperation::Move))),
                false,
                false,
            ),
            Vector2::new(240.0, 80.0),
        );
        let token = runtime
            .dispatch_pointer_ingress_with_admission(super::touch::touch(
                1,
                PointerPhase::Started {
                    button: PointerButton::Primary,
                },
                20.0,
                20.0,
                None,
            ))
            .sequence_token()
            .unwrap();
        assert!(events.borrow().is_empty());
        if mode == 0 {
            runtime.dispatch_pointer_ingress(super::touch::touch(
                1,
                PointerPhase::Moved,
                140.0,
                20.0,
                Some(token),
            ));
        }
        let end = super::touch::touch(
            1,
            PointerPhase::Ended {
                button: PointerButton::Primary,
            },
            if mode == 2 { 21.0 } else { 140.0 },
            20.0,
            Some(token),
        );
        runtime.dispatch_pointer_ingress(end);
        if mode == 2 {
            assert!(events.borrow().is_empty(), "{:?}", events.borrow());
        } else {
            assert_eq!(
                events
                    .borrow()
                    .iter()
                    .filter(|event| matches!(
                        event,
                        Event::Source(DragSourcePhase::Completed(DragOperation::Move))
                    ))
                    .count(),
                1,
                "{:?}",
                events.borrow()
            );
            assert_eq!(
                events
                    .borrow()
                    .iter()
                    .filter(|event| matches!(event, Event::Target(DropPhase::Dropped, _)))
                    .count(),
                1
            );
        }
        let count = events.borrow().len();
        runtime.dispatch_pointer_ingress(end);
        assert_eq!(events.borrow().len(), count);
    }
}

#[test]
fn second_contact_cancels_active_typed_drag_and_held_contacts_stay_inert() {
    use radiant::gui::pointer_ingress::PointerPhase;
    use radiant::widgets::PointerButton;
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = SurfaceRuntime::new(
        drag_bridge(
            events.clone(),
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(DropDecision::Accepted(DragOperation::Move))),
            false,
            false,
        ),
        Vector2::new(240.0, 80.0),
    );
    let start = PointerPhase::Started {
        button: PointerButton::Primary,
    };
    let end = PointerPhase::Ended {
        button: PointerButton::Primary,
    };
    let one = runtime
        .dispatch_pointer_ingress_with_admission(super::touch::touch(1, start, 20.0, 20.0, None))
        .sequence_token()
        .unwrap();
    runtime.dispatch_pointer_ingress(super::touch::touch(
        1,
        PointerPhase::Moved,
        140.0,
        20.0,
        Some(one),
    ));
    let two = runtime
        .dispatch_pointer_ingress_with_admission(super::touch::touch(2, start, 40.0, 20.0, None))
        .sequence_token()
        .unwrap();
    assert_eq!(
        events
            .borrow()
            .iter()
            .filter(|event| matches!(event, Event::Source(DragSourcePhase::Cancelled(_))))
            .count(),
        1,
        "{:?}",
        events.borrow()
    );
    let count = events.borrow().len();
    runtime.dispatch_pointer_ingress(super::touch::touch(1, end, 140.0, 20.0, Some(one)));
    let three = runtime
        .dispatch_pointer_ingress_with_admission(super::touch::touch(3, start, 20.0, 20.0, None))
        .sequence_token()
        .unwrap();
    runtime.dispatch_pointer_ingress(super::touch::touch(
        3,
        PointerPhase::Moved,
        140.0,
        20.0,
        Some(three),
    ));
    runtime.dispatch_pointer_ingress(super::touch::touch(2, end, 40.0, 20.0, Some(two)));
    runtime.dispatch_pointer_ingress(super::touch::touch(3, end, 140.0, 20.0, Some(three)));
    assert_eq!(events.borrow().len(), count);
    let fresh = runtime
        .dispatch_pointer_ingress_with_admission(super::touch::touch(4, start, 20.0, 20.0, None))
        .sequence_token()
        .unwrap();
    runtime.dispatch_pointer_ingress(super::touch::touch(4, end, 140.0, 20.0, Some(fresh)));
    assert_eq!(
        events
            .borrow()
            .iter()
            .filter(|event| matches!(event, Event::Source(DragSourcePhase::Completed(_))))
            .count(),
        1
    );
}

#[test]
fn pending_single_touch_yields_to_declared_two_contact_gesture() {
    use radiant::gui::pointer_ingress::PointerPhase;
    use radiant::widgets::PointerButton;
    #[derive(Debug, PartialEq)]
    enum InputEvent {
        Drag(DragSourcePhase),
        Gesture(GesturePhase),
    }
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let mut runtime = SurfaceRuntime::new(
        radiant::app(())
            .view(|_| {
                button("Source")
                    .filter_mapped(|_| None::<InputEvent>)
                    .width(180.0)
                    .height(40.0)
                    .id(1)
                    .drag_source(
                        DragSource::new(Rc::new(String::from("payload")))
                            .on_event_with_revision((), |event| {
                                Some(InputEvent::Drag(event.phase()))
                            }),
                    )
                    .id(10)
                    .on_gesture_with_revision(
                        GesturePolicy::none()
                            .recognize(GestureKind::Pinch, 0.1)
                            .unwrap(),
                        (),
                        |event| Some(InputEvent::Gesture(event.phase())),
                    )
                    .id(30)
            })
            .update(move |_, event| observed.borrow_mut().push(event))
            .into_bridge(),
        Vector2::new(240.0, 80.0),
    );
    let start = PointerPhase::Started {
        button: PointerButton::Primary,
    };
    let end = PointerPhase::Ended {
        button: PointerButton::Primary,
    };
    let one = runtime
        .dispatch_pointer_ingress_with_admission(super::touch::touch(1, start, 20.0, 20.0, None))
        .sequence_token()
        .unwrap();
    let two = runtime
        .dispatch_pointer_ingress_with_admission(super::touch::touch(2, start, 60.0, 20.0, None))
        .sequence_token()
        .unwrap();
    runtime.dispatch_pointer_ingress(super::touch::touch(
        2,
        PointerPhase::Moved,
        100.0,
        20.0,
        Some(two),
    ));
    runtime.dispatch_pointer_ingress(super::touch::touch(2, end, 100.0, 20.0, Some(two)));
    runtime.dispatch_pointer_ingress(super::touch::touch(1, end, 20.0, 20.0, Some(one)));
    assert_eq!(
        *events.borrow(),
        vec![
            InputEvent::Gesture(GesturePhase::Started),
            InputEvent::Gesture(GesturePhase::Ended)
        ]
    );
}

#[test]
fn retired_single_touch_cannot_restart_before_physical_release() {
    use radiant::gui::pointer_ingress::PointerPhase;
    use radiant::widgets::PointerButton;
    for active in [false, true] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let revision = Rc::new(Cell::new(0));
        let mut runtime = SurfaceRuntime::new(
            drag_bridge(
                events.clone(),
                revision.clone(),
                Rc::new(Cell::new(0)),
                Rc::new(Cell::new(DropDecision::Accepted(DragOperation::Move))),
                false,
                false,
            ),
            Vector2::new(240.0, 80.0),
        );
        let start = PointerPhase::Started {
            button: PointerButton::Primary,
        };
        let end = PointerPhase::Ended {
            button: PointerButton::Primary,
        };
        let token = runtime
            .dispatch_pointer_ingress_with_admission(super::touch::touch(
                1, start, 20.0, 20.0, None,
            ))
            .sequence_token()
            .unwrap();
        if active {
            runtime.dispatch_pointer_ingress(super::touch::touch(
                1,
                PointerPhase::Moved,
                140.0,
                20.0,
                Some(token),
            ));
        }
        revision.set(1);
        runtime.refresh();
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| matches!(
                    event,
                    Event::Source(DragSourcePhase::Cancelled(DragCancelReason::SourceRetired))
                ))
                .count(),
            usize::from(active)
        );
        let count = events.borrow().len();
        runtime.dispatch_pointer_ingress(super::touch::touch(
            1,
            PointerPhase::Moved,
            160.0,
            20.0,
            Some(token),
        ));
        runtime.dispatch_pointer_ingress(super::touch::touch(1, end, 160.0, 20.0, Some(token)));
        assert_eq!(events.borrow().len(), count);
        let fresh = runtime
            .dispatch_pointer_ingress_with_admission(super::touch::touch(
                1, start, 20.0, 20.0, None,
            ))
            .sequence_token()
            .unwrap();
        runtime.dispatch_pointer_ingress(super::touch::touch(1, end, 140.0, 20.0, Some(fresh)));
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| matches!(event, Event::Source(DragSourcePhase::Completed(_))))
                .count(),
            1
        );
    }
}

#[test]
fn touch_drag_does_not_steal_mouse_capture_or_revive_after_callback_retirement() {
    use radiant::gui::pointer_ingress::PointerPhase;
    use radiant::widgets::PointerButton;
    for mouse_incumbent in [false, true] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = SurfaceRuntime::new(
            drag_bridge(
                events.clone(),
                Rc::new(Cell::new(0)),
                Rc::new(Cell::new(0)),
                Rc::new(Cell::new(DropDecision::Accepted(DragOperation::Move))),
                !mouse_incumbent,
                false,
            ),
            Vector2::new(240.0, 80.0),
        );
        let start = PointerPhase::Started {
            button: PointerButton::Primary,
        };
        let end = PointerPhase::Ended {
            button: PointerButton::Primary,
        };
        let mouse = mouse_incumbent.then(|| {
            runtime
                .dispatch_pointer_ingress_with_admission(super::pointer_drag::mouse(
                    start, 20.0, None,
                ))
                .sequence_token()
                .unwrap()
        });
        let finger = runtime
            .dispatch_pointer_ingress_with_admission(super::touch::touch(
                1, start, 20.0, 20.0, None,
            ))
            .sequence_token()
            .unwrap();
        runtime.dispatch_pointer_ingress(super::touch::touch(
            1,
            PointerPhase::Moved,
            140.0,
            20.0,
            Some(finger),
        ));
        runtime.dispatch_pointer_ingress(super::touch::touch(1, end, 140.0, 20.0, Some(finger)));
        if let Some(mouse) = mouse {
            assert!(events.borrow().is_empty());
            runtime.dispatch_pointer_ingress(super::pointer_drag::mouse(end, 140.0, Some(mouse)));
            assert_eq!(
                events
                    .borrow()
                    .iter()
                    .filter(|event| matches!(event, Event::Source(DragSourcePhase::Completed(_))))
                    .count(),
                1
            );
        } else {
            assert_eq!(
                events
                    .borrow()
                    .iter()
                    .filter(|event| matches!(event, Event::Source(DragSourcePhase::Cancelled(_))))
                    .count(),
                1,
                "{:?}",
                events.borrow()
            );
            assert!(!events.borrow().iter().any(|event| matches!(
                event,
                Event::Target(DropPhase::Dropped, _) | Event::Source(DragSourcePhase::Completed(_))
            )));
        }
    }
}
