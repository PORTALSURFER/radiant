use super::*;
use crate::{
    application::{DragSource, DropTarget, button},
    gui::drag_drop::DragAutoscrollPolicy,
    gui::pointer_ingress::{GestureIngress, GestureKind, GesturePhase, GestureUnit, InputDeviceId},
    layout::Vector2,
    runtime::{GestureRequest, RuntimeBridge, SurfaceRuntime},
};
use std::{cell::RefCell, rc::Rc};

fn pan(phase: GesturePhase, amount: f32) -> GestureIngress {
    GestureIngress::new(
        GestureKind::Pan,
        phase,
        GestureUnit::LogicalPixels,
        Vector2::new(amount, 0.0),
        InputDeviceId::from_host(91).expect("valid test device"),
        Some(Point::new(20.0, 20.0)),
        Default::default(),
        None,
        None,
    )
    .expect("finite test sample")
}

fn activate_source<Bridge>(runtime: &mut SurfaceRuntime<Bridge, DropPhase>) -> CrossWindowDragExport
where
    Bridge: RuntimeBridge<DropPhase>,
{
    let token = runtime
        .dispatch_gesture_request(GestureRequest::new(pan(GesturePhase::Started, 0.0)))
        .token()
        .expect("source admits a pan");
    let admission = runtime.dispatch_gesture_request_with_cross_window(
        GestureRequest::new(pan(GesturePhase::Changed, 10.0)).with_token(token),
        CrossWindowInputHint::foreign_or_none(),
        None,
        None,
    );
    assert!(matches!(
        admission.outcome(),
        crate::runtime::GestureOutcome::Accepted(_)
            | crate::runtime::GestureOutcome::AcceptedContainer(_)
    ));
    runtime
        .cross_window_drag_export()
        .expect("recognized typed source exports its offer")
}

fn foreign_input(export: &CrossWindowDragExport) -> CrossWindowForeignInput {
    foreign_input_at(export, Point::new(20.0, 20.0))
}

fn foreign_input_at(export: &CrossWindowDragExport, position: Point) -> CrossWindowForeignInput {
    CrossWindowForeignInput::new(
        export.key(),
        export.offer(),
        export.source(),
        export.lease(),
        position,
        Default::default(),
        export.autoscroll_policy(),
        export.metadata(),
    )
}

#[test]
fn foreign_autoscroll_deadline_marks_due_without_target_callbacks_and_stops_at_boundary() {
    let source_bridge = crate::app(())
        .view(|_| {
            button("source")
                .filter_mapped(|_| None::<DropPhase>)
                .width(100.0)
                .height(100.0)
                .id(1)
                .drag_source(DragSource::new(7_u8).autoscroll(DragAutoscrollPolicy::default()))
                .id(10)
        })
        .update(|_, _: DropPhase| {})
        .into_bridge();
    let mut source = SurfaceRuntime::new(source_bridge, Vector2::new(100.0, 100.0));
    let export = activate_source(&mut source);

    let callbacks = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&callbacks);
    let receiver_bridge = crate::app(())
        .view(move |_| {
            let observed = Rc::clone(&observed);
            crate::application::scroll(
                button("target")
                    .filter_mapped(|_| None::<DropPhase>)
                    .width(100.0)
                    .height(240.0)
                    .id(2)
                    .drop_target(DropTarget::<u8, DropPhase>::new().on_event_with_revision(
                        (),
                        move |event| {
                            observed.borrow_mut().push(event.phase());
                            Some(event.phase())
                        },
                    ))
                    .id(20),
            )
            .width(100.0)
            .height(100.0)
            .id(30)
        })
        .update(|_, _: DropPhase| {})
        .into_bridge();
    let mut receiver = SurfaceRuntime::new(receiver_bridge, Vector2::new(100.0, 100.0));
    let origin = std::time::Instant::now();
    receiver.set_timed_repaint_clock(Some(origin));
    assert_eq!(
        receiver
            .route_cross_window_foreign(foreign_input_at(&export, Point::new(50.0, 99.0)))
            .into_messages(),
        vec![DropPhase::Entered]
    );
    let deadline = receiver
        .cross_window_foreign_autoscroll_deadline()
        .expect("edge receipt arms a receiver-local deadline");
    assert!(receiver.advance_timed_repaints(deadline));
    assert!(receiver.has_pending_cross_window_autoscroll());
    assert_eq!(
        callbacks.borrow().as_slice(),
        &[DropPhase::Entered],
        "timer advancement only marks work for its admitted native collector"
    );

    let attempt = receiver.take_pending_cross_window_autoscroll(export.key(), deadline);
    assert!(attempt.accepted && attempt.moved);
    assert!(receiver.rearm_cross_window_autoscroll(export.key(), deadline));
    for _ in 0..32 {
        let Some(deadline) = receiver.cross_window_foreign_autoscroll_deadline() else {
            break;
        };
        receiver.advance_timed_repaints(deadline);
        let attempt = receiver.take_pending_cross_window_autoscroll(export.key(), deadline);
        if !attempt.moved {
            assert!(!receiver.rearm_cross_window_autoscroll(export.key(), deadline));
            break;
        }
        receiver.rearm_cross_window_autoscroll(export.key(), deadline);
    }
    assert_eq!(receiver.layout_state.scroll_offset(30).y, 140.0);
    assert_eq!(receiver.cross_window_foreign_autoscroll_deadline(), None);
    assert_eq!(callbacks.borrow().as_slice(), &[DropPhase::Entered]);
}

#[test]
fn foreign_autoscroll_scroll_callback_retires_source_before_ancestor_chaining() {
    let source_bridge = crate::app(())
        .view(|_| {
            button("source")
                .filter_mapped(|_| None::<DropPhase>)
                .width(100.0)
                .height(100.0)
                .id(1)
                .drag_source(DragSource::new(7_u8).autoscroll(DragAutoscrollPolicy::default()))
                .id(10)
        })
        .update(|_, _: DropPhase| {})
        .into_bridge();
    let source = Rc::new(RefCell::new(Some(SurfaceRuntime::new(
        source_bridge,
        Vector2::new(100.0, 100.0),
    ))));
    let export = {
        let mut source = source.borrow_mut();
        activate_source(source.as_mut().unwrap())
    };

    let retire_source = Rc::clone(&source);
    let receiver_bridge = crate::app(())
        .view(|_| {
            crate::application::scroll(
                button("target")
                    .filter_mapped(|_| None::<crate::runtime::ScrollUpdate>)
                    .width(100.0)
                    .height(240.0)
                    .id(2)
                    .drop_target(DropTarget::<u8, crate::runtime::ScrollUpdate>::new())
                    .id(20),
            )
            .width(100.0)
            .height(100.0)
            .id(30)
            .on_scroll_update(|update| update)
        })
        .update(move |_, _: crate::runtime::ScrollUpdate| {
            retire_source.borrow_mut().take();
        })
        .into_bridge();
    let mut receiver = SurfaceRuntime::new(receiver_bridge, Vector2::new(100.0, 100.0));
    let origin = std::time::Instant::now();
    receiver.set_timed_repaint_clock(Some(origin));
    let _ = receiver.route_cross_window_foreign(foreign_input_at(&export, Point::new(50.0, 99.0)));
    let deadline = receiver.cross_window_foreign_autoscroll_deadline().unwrap();
    assert!(receiver.advance_timed_repaints(deadline));

    let attempt = receiver.take_pending_cross_window_autoscroll(export.key(), deadline);
    assert!(attempt.moved);
    assert!(
        !export.is_live(),
        "the scroll callback retired the source while the guarded route was live"
    );
    assert!(!receiver.has_pending_cross_window_autoscroll());
    assert!(!receiver.rearm_cross_window_autoscroll(export.key(), deadline));
    assert_eq!(receiver.cross_window_foreign_autoscroll_deadline(), None);
}

#[test]
fn foreign_offer_keeps_payload_identity_and_wrong_keys_cannot_remove_receipt() {
    let payload = Rc::new(7_u8);
    let source_payload = Rc::clone(&payload);
    let source_bridge = crate::app(())
        .view(move |_| {
            button("source")
                .filter_mapped(|_| None::<DropPhase>)
                .width(100.0)
                .height(100.0)
                .id(1)
                .drag_source(DragSource::new(Rc::clone(&source_payload)))
                .id(10)
        })
        .update(|_, _: DropPhase| {})
        .into_bridge();
    let mut source = SurfaceRuntime::new(source_bridge, Vector2::new(100.0, 100.0));
    let export = activate_source(&mut source);

    let observed_phases = Rc::new(RefCell::new(Vec::new()));
    let observed_identity = Rc::new(RefCell::new(Vec::new()));
    let expected = Rc::clone(&payload);
    let phases = Rc::clone(&observed_phases);
    let identities = Rc::clone(&observed_identity);
    let receiver_bridge = crate::app(())
        .view(move |_| {
            let expected = Rc::clone(&expected);
            let phases = Rc::clone(&phases);
            let identities = Rc::clone(&identities);
            button("target")
                .filter_mapped(|_| None::<DropPhase>)
                .width(100.0)
                .height(100.0)
                .id(2)
                .drop_target(
                    DropTarget::<Rc<u8>, DropPhase>::new().on_event_with_revision(
                        (),
                        move |event| {
                            identities
                                .borrow_mut()
                                .push(Rc::ptr_eq(event.payload(), &expected));
                            phases.borrow_mut().push(event.phase());
                            Some(event.phase())
                        },
                    ),
                )
                .id(20)
        })
        .update(|_, _: DropPhase| {})
        .into_bridge();
    let mut receiver = SurfaceRuntime::new(receiver_bridge, Vector2::new(100.0, 100.0));

    let route = receiver.route_cross_window_foreign(foreign_input(&export));
    assert_eq!(route.into_messages(), vec![DropPhase::Entered]);
    assert_eq!(observed_phases.borrow().as_slice(), &[DropPhase::Entered]);
    assert_eq!(observed_identity.borrow().as_slice(), &[true]);
    assert!(receiver.cross_window_foreign_preview().is_some());
    assert!(receiver.finish_cross_window_foreign_feedback(export.key()));
    assert_eq!(
        observed_phases.borrow().as_slice(),
        &[DropPhase::Entered],
        "a paint-only requalification must not emit a duplicate Over"
    );

    let other_payload = Rc::new(8_u8);
    let other_bridge = crate::app(())
        .view(move |_| {
            button("other")
                .filter_mapped(|_| None::<DropPhase>)
                .width(100.0)
                .height(100.0)
                .id(3)
                .drag_source(DragSource::new(Rc::clone(&other_payload)))
                .id(30)
        })
        .update(|_, _: DropPhase| {})
        .into_bridge();
    let mut other = SurfaceRuntime::new(other_bridge, Vector2::new(100.0, 100.0));
    let other_export = activate_source(&mut other);

    assert!(
        receiver
            .clear_cross_window_foreign(other_export.key())
            .into_messages()
            .is_empty()
    );
    assert!(
        receiver
            .take_cross_window_foreign_terminal(other_export.key())
            .is_none()
    );
    assert!(receiver.cross_window_foreign_preview().is_some());
    assert_eq!(observed_phases.borrow().as_slice(), &[DropPhase::Entered]);

    assert!(
        receiver
            .route_cross_window_foreign(foreign_input(&other_export))
            .into_messages()
            .is_empty(),
        "a competing export cannot replace a live receipt before its Left is reduced"
    );
    assert!(receiver.cross_window_foreign_preview().is_some());
    assert_eq!(observed_phases.borrow().as_slice(), &[DropPhase::Entered]);
}

#[test]
fn closing_receiver_rejects_foreign_work_without_target_callbacks() {
    let source_bridge = crate::app(())
        .view(|_| {
            button("source")
                .filter_mapped(|_| None::<DropPhase>)
                .width(100.0)
                .height(100.0)
                .id(1)
                .drag_source(DragSource::new(7_u8))
                .id(10)
        })
        .update(|_, _: DropPhase| {})
        .into_bridge();
    let mut source = SurfaceRuntime::new(source_bridge, Vector2::new(100.0, 100.0));
    let export = activate_source(&mut source);

    let callbacks = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&callbacks);
    let receiver_bridge = crate::app(())
        .view(move |_| {
            let observed = Rc::clone(&observed);
            button("target")
                .filter_mapped(|_| None::<DropPhase>)
                .width(100.0)
                .height(100.0)
                .id(2)
                .drop_target(DropTarget::<u8, DropPhase>::new().on_event_with_revision(
                    (),
                    move |event| {
                        observed.borrow_mut().push(event.phase());
                        Some(event.phase())
                    },
                ))
                .id(20)
        })
        .update(|_, _: DropPhase| {})
        .into_bridge();
    let mut receiver = SurfaceRuntime::new(receiver_bridge, Vector2::new(100.0, 100.0));

    assert!(receiver.begin_closing());
    assert!(
        receiver
            .route_cross_window_foreign(foreign_input(&export))
            .into_messages()
            .is_empty()
    );
    assert!(receiver.cross_window_foreign_preview().is_none());
    assert!(callbacks.borrow().is_empty());
}

#[test]
fn expired_source_lease_cancels_current_receiver_exactly_once() {
    let source_bridge = crate::app(())
        .view(|_| {
            button("source")
                .filter_mapped(|_| None::<DropPhase>)
                .width(100.0)
                .height(100.0)
                .id(1)
                .drag_source(DragSource::new(7_u8))
                .id(10)
        })
        .update(|_, _: DropPhase| {})
        .into_bridge();
    let mut source = SurfaceRuntime::new(source_bridge, Vector2::new(100.0, 100.0));
    let export = activate_source(&mut source);

    let callbacks = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&callbacks);
    let receiver_bridge = crate::app(())
        .view(move |_| {
            let observed = Rc::clone(&observed);
            button("target")
                .filter_mapped(|_| None::<DropPhase>)
                .width(100.0)
                .height(100.0)
                .id(2)
                .drop_target(DropTarget::<u8, DropPhase>::new().on_event_with_revision(
                    (),
                    move |event| {
                        observed.borrow_mut().push(event.phase());
                        Some(event.phase())
                    },
                ))
                .id(20)
        })
        .update(|_, _: DropPhase| {})
        .into_bridge();
    let mut receiver = SurfaceRuntime::new(receiver_bridge, Vector2::new(100.0, 100.0));

    assert_eq!(
        receiver
            .route_cross_window_foreign(foreign_input(&export))
            .into_messages(),
        vec![DropPhase::Entered]
    );
    drop(source);
    assert!(!export.is_live());
    assert_eq!(
        receiver
            .cancel_cross_window_foreign(export.key())
            .into_messages(),
        vec![DropPhase::Cancelled]
    );
    assert!(receiver.cross_window_foreign_preview().is_none());
    assert!(
        receiver
            .cancel_cross_window_foreign(export.key())
            .into_messages()
            .is_empty(),
        "a removed foreign receipt cannot emit a second cleanup"
    );
    assert_eq!(
        callbacks.borrow().as_slice(),
        &[DropPhase::Entered, DropPhase::Cancelled]
    );
}

#[test]
fn expired_source_lease_never_maps_cleanup_into_a_closing_receiver() {
    let source_bridge = crate::app(())
        .view(|_| {
            button("source")
                .filter_mapped(|_| None::<DropPhase>)
                .width(100.0)
                .height(100.0)
                .id(1)
                .drag_source(DragSource::new(7_u8))
                .id(10)
        })
        .update(|_, _: DropPhase| {})
        .into_bridge();
    let mut source = SurfaceRuntime::new(source_bridge, Vector2::new(100.0, 100.0));
    let export = activate_source(&mut source);

    let callbacks = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&callbacks);
    let receiver_bridge = crate::app(())
        .view(move |_| {
            let observed = Rc::clone(&observed);
            button("target")
                .filter_mapped(|_| None::<DropPhase>)
                .width(100.0)
                .height(100.0)
                .id(2)
                .drop_target(DropTarget::<u8, DropPhase>::new().on_event_with_revision(
                    (),
                    move |event| {
                        observed.borrow_mut().push(event.phase());
                        Some(event.phase())
                    },
                ))
                .id(20)
        })
        .update(|_, _: DropPhase| {})
        .into_bridge();
    let mut receiver = SurfaceRuntime::new(receiver_bridge, Vector2::new(100.0, 100.0));
    assert_eq!(
        receiver
            .route_cross_window_foreign(foreign_input(&export))
            .into_messages(),
        vec![DropPhase::Entered]
    );
    drop(source);
    assert!(receiver.begin_closing());
    assert!(
        receiver
            .cancel_cross_window_foreign(export.key())
            .into_messages()
            .is_empty()
    );
    assert_eq!(callbacks.borrow().as_slice(), &[DropPhase::Entered]);
}

#[test]
fn foreign_receiver_binding_does_not_replace_an_incumbent_local_gesture_capture() {
    let source_bridge = crate::app(())
        .view(|_| {
            button("source")
                .filter_mapped(|_| None::<DropPhase>)
                .width(100.0)
                .height(100.0)
                .id(1)
                .drag_source(DragSource::new(7_u8))
                .id(10)
        })
        .update(|_, _: DropPhase| {})
        .into_bridge();
    let mut source = SurfaceRuntime::new(source_bridge, Vector2::new(100.0, 100.0));
    let export = activate_source(&mut source);

    let receiver_bridge = crate::app(())
        .view(|_| {
            button("target")
                .filter_mapped(|_| None::<DropPhase>)
                .width(100.0)
                .height(100.0)
                .id(2)
                .drop_target(DropTarget::<u8, DropPhase>::new())
                .id(20)
                .drag_source(DragSource::new(9_u8))
                .id(30)
        })
        .update(|_, _: DropPhase| {})
        .into_bridge();
    let mut receiver = SurfaceRuntime::new(receiver_bridge, Vector2::new(100.0, 100.0));
    let incumbent = receiver
        .dispatch_gesture_request(GestureRequest::new(pan(GesturePhase::Started, 0.0)))
        .token()
        .expect("receiver admits its own pending gesture");

    let _ = receiver.route_cross_window_foreign(foreign_input(&export));
    assert_eq!(
        receiver
            .interaction
            .gesture
            .as_ref()
            .map(|capture| capture.token),
        Some(incumbent),
        "foreign receipt state owns no GestureCapture"
    );
}
