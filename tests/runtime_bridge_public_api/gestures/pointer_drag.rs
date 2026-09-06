use super::*;
use radiant::{
    application::{DragSource, DropTarget, button, row},
    gui::pointer_ingress::{
        DeviceKind, PointerButtons, PointerContactId, PointerIngress, PointerIngressDisposition,
        PointerPhase, PointerSequenceToken,
    },
    runtime::{DragSourcePhase, DropPhase},
    widgets::PointerButton,
};
#[derive(Clone, Debug, PartialEq)]
enum Message {
    Click,
    Decision(DropPhase, radiant::runtime::DropDecision),
    Gesture(GesturePhase),
    Pointer(radiant::gui::pointer_ingress::PointerEvent),
    Source(DragSourcePhase),
    Drop(DropPhase),
}
fn bridge(events: Rc<RefCell<Vec<Message>>>) -> impl radiant::runtime::RuntimeBridge<Message> {
    radiant::app(())
        .view(|_| {
            row([
                button("Source")
                    .message(Message::Click)
                    .width(100.0)
                    .height(40.0)
                    .id(1)
                    .drag_source(
                        DragSource::new(String::from("payload"))
                            .on_event_with_revision((), |event| {
                                Some(Message::Source(event.phase()))
                            }),
                    )
                    .id(10),
                button("Target")
                    .filter_mapped(|_| None::<Message>)
                    .width(100.0)
                    .height(40.0)
                    .id(2)
                    .drop_target(
                        DropTarget::<String, Message>::new()
                            .on_event_with_revision((), |event| Some(Message::Drop(event.phase()))),
                    )
                    .id(20),
            ])
            .spacing(0.0)
            .id(30)
        })
        .update(move |_, event| events.borrow_mut().push(event))
        .into_bridge()
}
fn mouse(phase: PointerPhase, x: f32, token: Option<PointerSequenceToken>) -> PointerIngress {
    let device = InputDeviceId::from_host(1).unwrap();
    let contact = PointerContactId::from_host(1).unwrap();
    let position = radiant::layout::Point::new(x, 15.0);
    let buttons = if phase.is_terminal() {
        PointerButtons::empty()
    } else {
        PointerButtons::PRIMARY
    };
    if let Some(token) = token {
        PointerIngress::from_runtime(
            DeviceKind::Mouse,
            device,
            contact,
            phase,
            position,
            buttons,
            Default::default(),
            None,
            None,
            None,
            None,
            token,
        )
        .unwrap()
    } else {
        PointerIngress::new(
            DeviceKind::Mouse,
            device,
            contact,
            phase,
            position,
            buttons,
            Default::default(),
            None,
            None,
            None,
            None,
        )
        .unwrap()
    }
}
#[test]
fn pointer_drag_preserves_click_below_threshold_and_cancels_child_when_drag_wins() {
    for drag in [false, true] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = SurfaceRuntime::new(bridge(events.clone()), Vector2::new(240.0, 80.0));
        let start = runtime.dispatch_pointer_ingress_with_admission(mouse(
            PointerPhase::Started {
                button: PointerButton::Primary,
            },
            20.0,
            None,
        ));
        assert_eq!(
            start.disposition(),
            PointerIngressDisposition::RoutedWidget(1)
        );
        let token = start.sequence_token().unwrap();
        assert!(events.borrow().is_empty());
        let x = if drag { 130.0 } else { 22.0 };
        let moved = runtime.dispatch_pointer_ingress(mouse(PointerPhase::Moved, x, Some(token)));
        assert_eq!(
            moved,
            if drag {
                PointerIngressDisposition::RoutedGesture(10)
            } else {
                PointerIngressDisposition::RoutedWidget(1)
            }
        );
        runtime.dispatch_pointer_ingress(mouse(
            PointerPhase::Ended {
                button: PointerButton::Primary,
            },
            x,
            Some(token),
        ));
        assert!(!runtime.drag_session_active());
        if drag {
            assert!(!events.borrow().contains(&Message::Click));
            assert!(events.borrow().contains(&Message::Drop(DropPhase::Dropped)));
            assert_eq!(
                events
                    .borrow()
                    .iter()
                    .filter(|event| matches!(event, Message::Source(DragSourcePhase::Completed(_))))
                    .count(),
                1
            );
        } else {
            assert_eq!(*events.borrow(), [Message::Click]);
        }
        assert_eq!(
            runtime.dispatch_pointer_ingress(mouse(
                PointerPhase::Ended {
                    button: PointerButton::Primary
                },
                x,
                Some(token)
            )),
            PointerIngressDisposition::Stale
        );
        // A prior transfer tombstone must not consume the next independent click.
        let token = runtime
            .dispatch_pointer_ingress_with_admission(mouse(
                PointerPhase::Started {
                    button: PointerButton::Primary,
                },
                20.0,
                None,
            ))
            .sequence_token()
            .unwrap();
        runtime.dispatch_pointer_ingress(mouse(
            PointerPhase::Ended {
                button: PointerButton::Primary,
            },
            20.0,
            Some(token),
        ));
        assert_eq!(events.borrow().last(), Some(&Message::Click));
    }
}
#[test]
fn pointer_drag_can_cross_threshold_on_release_and_cancel_without_click() {
    for cancel in [false, true] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = SurfaceRuntime::new(bridge(events.clone()), Vector2::new(240.0, 80.0));
        let token = runtime
            .dispatch_pointer_ingress_with_admission(mouse(
                PointerPhase::Started {
                    button: PointerButton::Primary,
                },
                20.0,
                None,
            ))
            .sequence_token()
            .unwrap();
        if cancel {
            runtime.dispatch_pointer_ingress(mouse(PointerPhase::Moved, 130.0, Some(token)));
        }
        let phase = if cancel {
            PointerPhase::Cancelled
        } else {
            PointerPhase::Ended {
                button: PointerButton::Primary,
            }
        };
        assert_eq!(
            runtime.dispatch_pointer_ingress(mouse(phase, 130.0, Some(token))),
            PointerIngressDisposition::RoutedGesture(10)
        );
        assert!(!runtime.drag_session_active());
        assert!(!events.borrow().contains(&Message::Click));
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|event| matches!(event, Message::Source(DragSourcePhase::Started)))
                .count(),
            1
        );
        assert!(
            matches!(
                events.borrow().last(),
                Some(Message::Source(DragSourcePhase::Cancelled(_)))
            ) == cancel
        );
    }
}

#[test]
fn pointer_drag_cancels_original_typed_child_once_and_rechecks_source_after_its_mapper() {
    for remove_on_cancel in [false, true] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let output = events.clone();
        let bridge = radiant::app(false).view(|removed| {
            if *removed { return button("Removed").message(Message::Click).id(3); }
            radiant::application::render_canvas_pointer(1, 0, radiant::runtime::RenderCanvasContent::SignalBands {
                frames: 1, band_count: 1, frame_range: [0.0, 1.0], samples: std::sync::Arc::from([0.0]),
            }, Message::Pointer).size(100.0,40.0).id(1)
                .drag_source(DragSource::new(42u32).on_event_with_revision((), |event| Some(Message::Source(event.phase())))).id(10)
        }).update(move |removed, message| {
            if remove_on_cancel && matches!(&message, Message::Pointer(event) if event.phase() == PointerPhase::Cancelled) { *removed = true; }
            output.borrow_mut().push(message);
        }).into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 80.0));
        let token = runtime
            .dispatch_pointer_ingress_with_admission(mouse(
                PointerPhase::Started {
                    button: PointerButton::Primary,
                },
                20.0,
                None,
            ))
            .sequence_token()
            .unwrap();
        let moved = runtime.dispatch_pointer_ingress(mouse(PointerPhase::Moved, 30.0, Some(token)));
        assert_eq!(
            moved,
            if remove_on_cancel {
                PointerIngressDisposition::Stale
            } else {
                PointerIngressDisposition::RoutedGesture(10)
            }
        );
        let pointers = events
            .borrow()
            .iter()
            .filter_map(|message| match message {
                Message::Pointer(event) => Some(*event),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            pointers
                .iter()
                .map(|event| event.phase())
                .collect::<Vec<_>>(),
            [
                PointerPhase::Started {
                    button: PointerButton::Primary
                },
                PointerPhase::Cancelled
            ]
        );
        assert!(
            pointers
                .iter()
                .all(|event| event.sequence_token() == Some(token))
        );
        assert_eq!(
            events
                .borrow()
                .iter()
                .any(|event| matches!(event, Message::Source(DragSourcePhase::Started))),
            !remove_on_cancel
        );
        runtime.dispatch_pointer_ingress(mouse(PointerPhase::Cancelled, 30.0, Some(token)));
        assert!(!runtime.drag_session_active());
        assert_eq!(events.borrow().iter().filter(|event| matches!(event,Message::Pointer(event) if event.phase() == PointerPhase::Cancelled)).count(),1);
    }
}

#[test]
fn pointer_drag_uses_deepest_crossed_child_or_ancestor_recognizer_and_keeps_one_winner() {
    for (delta, winner) in [(3.0, 10), (5.0, 1)] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let output = events.clone();
        let raw = Rc::new(Cell::new(0));
        let bridge = radiant::app(())
            .view(move |_| {
                custom_widget_mapped(
                    Probe {
                        common: WidgetCommon::fixed(1, 100.0, 40.0).with_keyboard_focus(),
                        threshold: 5.0,
                        conservative: false,
                        raw: raw.clone(),
                    },
                    |event: GestureEvent| Message::Gesture(event.phase()),
                )
                .id(1)
                .drag_source(
                    DragSource::new(42u32)
                        .recognize_after(3.0)
                        .unwrap()
                        .on_event_with_revision((), |event| Some(Message::Source(event.phase()))),
                )
                .id(10)
            })
            .update(move |_, event| output.borrow_mut().push(event))
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 80.0));
        let token = runtime
            .dispatch_pointer_ingress_with_admission(mouse(
                PointerPhase::Started {
                    button: PointerButton::Primary,
                },
                20.0,
                None,
            ))
            .sequence_token()
            .unwrap();
        assert_eq!(
            runtime.dispatch_pointer_ingress(mouse(PointerPhase::Moved, 20.0 + delta, Some(token))),
            PointerIngressDisposition::RoutedGesture(winner)
        );
        assert_eq!(runtime.drag_session_active(), winner == 10);
        assert_eq!(
            runtime.dispatch_pointer_ingress(mouse(PointerPhase::Moved, 30.0, Some(token))),
            PointerIngressDisposition::RoutedGesture(winner)
        );
        runtime.dispatch_pointer_ingress(mouse(PointerPhase::Cancelled, 30.0, Some(token)));
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|message| matches!(
                    message,
                    Message::Gesture(GesturePhase::Started)
                        | Message::Source(DragSourcePhase::Started)
                ))
                .count(),
            1
        );
        assert!(!runtime.drag_session_active());
    }
}

#[test]
fn pointer_drag_operation_negotiation_uses_current_sample_modifiers() {
    use radiant::runtime::{DragOperation, DragOperations, DropDecision};
    let events = Rc::new(RefCell::new(Vec::new()));
    let output = events.clone();
    let bridge = radiant::app(())
        .view(|_| {
            row([
                button("Source")
                    .message(Message::Click)
                    .width(100.0)
                    .height(40.0)
                    .id(1)
                    .drag_source(
                        DragSource::new(42u32)
                            .operations(DragOperations::all())
                            .on_event_with_revision((), |event| {
                                Some(Message::Source(event.phase()))
                            }),
                    )
                    .id(10),
                button("Target")
                    .filter_mapped(|_| None::<Message>)
                    .width(100.0)
                    .height(40.0)
                    .id(2)
                    .drop_target(
                        DropTarget::<u32, Message>::new()
                            .negotiate_with_revision((), |_, context| {
                                DropDecision::Accepted(if context.modifiers().alt {
                                    DragOperation::Copy
                                } else {
                                    DragOperation::Move
                                })
                            })
                            .on_event_with_revision((), |event| {
                                Some(Message::Decision(event.phase(), event.decision()))
                            }),
                    )
                    .id(20),
            ])
            .spacing(0.0)
            .id(30)
        })
        .update(move |_, event| output.borrow_mut().push(event))
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 80.0));
    let token = runtime
        .dispatch_pointer_ingress_with_admission(mouse(
            PointerPhase::Started {
                button: PointerButton::Primary,
            },
            20.0,
            None,
        ))
        .sequence_token()
        .unwrap();
    runtime.dispatch_pointer_ingress(mouse(PointerPhase::Moved, 130.0, Some(token)));
    assert!(events.borrow().contains(&Message::Decision(
        DropPhase::Entered,
        DropDecision::Accepted(DragOperation::Move)
    )));
    for phase in [
        PointerPhase::Moved,
        PointerPhase::Ended {
            button: PointerButton::Primary,
        },
    ] {
        let ingress = PointerIngress::from_runtime(
            DeviceKind::Mouse,
            InputDeviceId::from_host(1).unwrap(),
            PointerContactId::from_host(1).unwrap(),
            phase,
            radiant::layout::Point::new(130.0, 15.0),
            PointerButtons::empty(),
            radiant::widgets::PointerModifiers {
                alt: true,
                ..Default::default()
            },
            None,
            None,
            None,
            None,
            token,
        )
        .unwrap();
        assert_eq!(
            runtime.dispatch_pointer_ingress(ingress),
            PointerIngressDisposition::RoutedGesture(10)
        );
    }
    assert!(events.borrow().contains(&Message::Decision(
        DropPhase::Over,
        DropDecision::Accepted(DragOperation::Copy)
    )));
    assert!(events.borrow().ends_with(&[
        Message::Decision(
            DropPhase::Dropped,
            DropDecision::Accepted(DragOperation::Copy)
        ),
        Message::Source(DragSourcePhase::Completed(DragOperation::Copy))
    ]));
}

#[test]
fn pointer_drag_preview_motion_after_transfer_keeps_projection_and_layout_unchanged() {
    let bridge = radiant::app(())
        .view(|_| {
            button("Source")
                .filter_mapped(|_| None::<()>)
                .size(100.0, 40.0)
                .id(1)
                .drag_source(DragSource::new(42u32))
                .id(10)
        })
        .update(|_, _| {})
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 80.0));
    let token = runtime
        .dispatch_pointer_ingress_with_admission(mouse(
            PointerPhase::Started {
                button: PointerButton::Primary,
            },
            20.0,
            None,
        ))
        .sequence_token()
        .unwrap();
    runtime.dispatch_pointer_ingress(mouse(PointerPhase::Moved, 30.0, Some(token)));
    assert!(runtime.drag_session_active());
    let before = runtime.refresh_counters();
    for x in [40.0, 50.0, 60.0] {
        assert_eq!(
            runtime.dispatch_pointer_ingress(mouse(PointerPhase::Moved, x, Some(token))),
            PointerIngressDisposition::RoutedGesture(10)
        );
    }
    let after = runtime.refresh_counters();
    assert_eq!(after.application_projection, before.application_projection);
    assert_eq!(after.runtime_projection, before.runtime_projection);
    assert_eq!(after.layout, before.layout);
    runtime.dispatch_pointer_ingress(mouse(PointerPhase::Cancelled, 60.0, Some(token)));
    assert!(!runtime.drag_session_active());
}
