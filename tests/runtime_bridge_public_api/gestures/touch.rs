use super::*;
use radiant::{
    gui::pointer_ingress::{
        DeviceKind, PointerButtons, PointerContactId, PointerIngress, PointerIngressDisposition,
        PointerPhase, PointerSequenceToken,
    },
    runtime::RuntimeBridge,
    widgets::PointerButton,
};

struct Fixture<B: RuntimeBridge<GestureEvent>> {
    runtime: SurfaceRuntime<B, GestureEvent>,
    events: Rc<RefCell<Vec<GestureEvent>>>,
    shown: Rc<Cell<bool>>,
    threshold: Rc<Cell<f32>>,
}
fn fixture() -> Fixture<impl RuntimeBridge<GestureEvent>> {
    let events = Rc::new(RefCell::new(Vec::new()));
    let shown = Rc::new(Cell::new(true));
    let threshold = Rc::new(Cell::new(5.0));
    let runtime = SurfaceRuntime::new(
        super::bridge(
            events.clone(),
            Rc::new(Cell::new(0)),
            threshold.clone(),
            shown.clone(),
        ),
        Vector2::new(200.0, 80.0),
    );
    Fixture {
        runtime,
        events,
        shown,
        threshold,
    }
}
fn touch(
    contact: u64,
    phase: PointerPhase,
    x: f32,
    y: f32,
    token: Option<PointerSequenceToken>,
) -> PointerIngress {
    let device = InputDeviceId::from_host(2).unwrap();
    let contact = PointerContactId::from_host(contact).unwrap();
    let point = radiant::layout::Point::new(x, y);
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
}
fn start<B: RuntimeBridge<M>, M>(
    runtime: &mut SurfaceRuntime<B, M>,
    contact: u64,
    x: f32,
) -> PointerSequenceToken {
    runtime
        .dispatch_pointer_ingress_with_admission(touch(
            contact,
            PointerPhase::Started {
                button: PointerButton::Primary,
            },
            x,
            20.0,
            None,
        ))
        .sequence_token()
        .unwrap()
}
fn move_touch<B: RuntimeBridge<M>, M>(
    runtime: &mut SurfaceRuntime<B, M>,
    contact: u64,
    token: PointerSequenceToken,
    x: f32,
    y: f32,
) -> PointerIngressDisposition {
    runtime.dispatch_pointer_ingress(touch(contact, PointerPhase::Moved, x, y, Some(token)))
}

#[test]
fn touch_first_contact_moves_remain_inert_and_pair_rebases_before_pan() {
    let mut f = fixture();
    let a = start(&mut f.runtime, 1, 10.0);
    move_touch(&mut f.runtime, 1, a, 20.0, 20.0);
    assert!(f.events.borrow().is_empty());
    let b = start(&mut f.runtime, 2, 100.0);
    assert!(f.events.borrow().is_empty());
    move_touch(&mut f.runtime, 1, a, 26.0, 20.0);
    assert!(f.events.borrow().is_empty());
    assert_eq!(
        move_touch(&mut f.runtime, 2, b, 106.0, 20.0),
        PointerIngressDisposition::RoutedGesture(1)
    );
    assert_eq!(f.events.borrow().len(), 1);
    assert_eq!(f.events.borrow()[0].sample().kind(), GestureKind::Pan);
    assert_eq!(f.events.borrow()[0].accumulated(), Vector2::new(6.0, 0.0));
    f.runtime.dispatch_pointer_ingress(touch(
        2,
        PointerPhase::Ended {
            button: PointerButton::Primary,
        },
        110.0,
        20.0,
        Some(b),
    ));
    assert_eq!(
        f.events.borrow().last().unwrap().phase(),
        GesturePhase::Ended
    );
    assert_eq!(
        f.events.borrow().last().unwrap().accumulated(),
        Vector2::new(8.0, 0.0)
    );
    assert_eq!(
        move_touch(&mut f.runtime, 1, a, 30.0, 20.0),
        PointerIngressDisposition::Stale
    );
}

#[test]
fn touch_pinch_wins_normalized_competition_and_keeps_its_family() {
    let mut f = fixture();
    let a = start(&mut f.runtime, 1, 20.0);
    let b = start(&mut f.runtime, 2, 60.0);
    assert_eq!(
        move_touch(&mut f.runtime, 2, b, 80.0, 20.0),
        PointerIngressDisposition::RoutedGesture(1)
    );
    assert_eq!(f.events.borrow()[0].sample().kind(), GestureKind::Pinch);
    assert_eq!(f.events.borrow()[0].accumulated().x, 1.5);
    move_touch(&mut f.runtime, 1, a, 40.0, 20.0);
    assert!(
        f.events
            .borrow()
            .iter()
            .all(|event| event.sample().kind() == GestureKind::Pinch)
    );
    assert_eq!(f.events.borrow().last().unwrap().accumulated().x, 1.0);
}

#[test]
fn touch_rotation_preserves_wrapped_direction_and_terminal_position() {
    let mut f = fixture();
    let _a = start(&mut f.runtime, 1, 20.0);
    let b = start(&mut f.runtime, 2, 60.0);
    move_touch(&mut f.runtime, 2, b, 20.0, 60.0);
    assert_eq!(f.events.borrow()[0].sample().kind(), GestureKind::Rotate);
    assert!((f.events.borrow()[0].accumulated().x - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
    f.runtime.dispatch_pointer_ingress(touch(
        2,
        PointerPhase::Ended {
            button: PointerButton::Primary,
        },
        25.0,
        70.0,
        Some(b),
    ));
    let events = f.events.borrow();
    assert_eq!(events.last().unwrap().phase(), GesturePhase::Ended);
    assert!((events.last().unwrap().accumulated().x - 50.0_f32.atan2(5.0)).abs() < 0.0001);
}

#[test]
fn touch_third_contact_cancels_once_and_old_tokens_cannot_revive() {
    let mut f = fixture();
    let a = start(&mut f.runtime, 1, 20.0);
    let b = start(&mut f.runtime, 2, 60.0);
    move_touch(&mut f.runtime, 2, b, 80.0, 20.0);
    let _c = start(&mut f.runtime, 3, 100.0);
    assert_eq!(
        f.events
            .borrow()
            .iter()
            .filter(|event| event.phase() == GesturePhase::Cancelled)
            .count(),
        1
    );
    let count = f.events.borrow().len();
    assert_eq!(
        move_touch(&mut f.runtime, 1, a, 25.0, 20.0),
        PointerIngressDisposition::Stale
    );
    assert_eq!(
        move_touch(&mut f.runtime, 2, b, 90.0, 20.0),
        PointerIngressDisposition::Stale
    );
    assert_eq!(f.events.borrow().len(), count);
}

#[test]
fn touch_cancellation_and_source_replacement_have_one_terminal() {
    for replace in [false, true] {
        let mut f = fixture();
        let a = start(&mut f.runtime, 1, 20.0);
        let b = start(&mut f.runtime, 2, 60.0);
        move_touch(&mut f.runtime, 2, b, 80.0, 20.0);
        if replace {
            f.shown.set(false);
            f.runtime.refresh();
        } else {
            f.runtime.dispatch_pointer_ingress(touch(
                1,
                PointerPhase::Cancelled,
                20.0,
                20.0,
                Some(a),
            ));
        }
        assert_eq!(
            f.events
                .borrow()
                .iter()
                .filter(|event| event.phase() == GesturePhase::Cancelled)
                .count(),
            1
        );
        assert_eq!(
            move_touch(&mut f.runtime, 2, b, 90.0, 20.0),
            PointerIngressDisposition::Stale
        );
    }
}

#[test]
fn touch_policy_change_retires_the_pair() {
    let mut f = fixture();
    let _a = start(&mut f.runtime, 1, 20.0);
    let b = start(&mut f.runtime, 2, 60.0);
    move_touch(&mut f.runtime, 2, b, 80.0, 20.0);
    f.threshold.set(9.0);
    f.runtime.refresh();
    assert_eq!(
        f.events.borrow().last().unwrap().phase(),
        GesturePhase::Cancelled
    );
    assert_eq!(
        move_touch(&mut f.runtime, 2, b, 90.0, 20.0),
        PointerIngressDisposition::Stale
    );
}

#[test]
fn touch_pair_does_not_steal_an_active_native_gesture() {
    let mut f = fixture();
    let native = f
        .runtime
        .dispatch_gesture_request(GestureRequest::new(sample(
            GestureKind::Pan,
            GesturePhase::Started,
            Vector2::new(0.0, 0.0),
        )))
        .token()
        .unwrap();
    f.runtime.dispatch_gesture_request(
        GestureRequest::new(sample(
            GestureKind::Pan,
            GesturePhase::Changed,
            Vector2::new(6.0, 0.0),
        ))
        .with_token(native),
    );
    let _a = start(&mut f.runtime, 1, 20.0);
    let result = f.runtime.dispatch_pointer_ingress_with_admission(touch(
        2,
        PointerPhase::Started {
            button: PointerButton::Primary,
        },
        60.0,
        20.0,
        None,
    ));
    assert_eq!(result.disposition(), PointerIngressDisposition::Blocked);
    assert_eq!(f.events.borrow().len(), 1);
    f.runtime.dispatch_gesture_request(
        GestureRequest::new(sample(
            GestureKind::Pan,
            GesturePhase::Ended,
            Vector2::new(1.0, 0.0),
        ))
        .with_token(native),
    );
    assert_eq!(
        f.events.borrow().last().unwrap().phase(),
        GesturePhase::Ended
    );
    assert_eq!(f.events.borrow().last().unwrap().accumulated().x, 7.0);
}

#[test]
fn touch_extra_held_contact_blocks_new_pairs_until_all_contacts_end() {
    let mut f = fixture();
    let _a = start(&mut f.runtime, 1, 20.0);
    let b = start(&mut f.runtime, 2, 60.0);
    move_touch(&mut f.runtime, 2, b, 80.0, 20.0);
    let c = start(&mut f.runtime, 3, 100.0);
    let count = f.events.borrow().len();
    let d = start(&mut f.runtime, 4, 20.0);
    let e = start(&mut f.runtime, 5, 60.0);
    move_touch(&mut f.runtime, 5, e, 100.0, 20.0);
    assert_eq!(f.events.borrow().len(), count);
    for (contact, token) in [(3, c), (4, d), (5, e)] {
        f.runtime.dispatch_pointer_ingress(touch(
            contact,
            PointerPhase::Ended {
                button: PointerButton::Primary,
            },
            60.0,
            20.0,
            Some(token),
        ));
    }
    let _a = start(&mut f.runtime, 6, 20.0);
    let b = start(&mut f.runtime, 7, 60.0);
    move_touch(&mut f.runtime, 7, b, 80.0, 20.0);
    assert_eq!(
        f.events.borrow().last().unwrap().phase(),
        GesturePhase::Started
    );
    assert_eq!(f.events.borrow().len(), count + 1);
}

#[test]
fn touch_deepest_crossed_target_wins_before_normalized_score() {
    for (threshold, expected) in [(2.0, 1_u8), (10.0, 10_u8)] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let observed = events.clone();
        let mut runtime = SurfaceRuntime::new(
            radiant::app(())
                .view(move |_| {
                    super::arena_leaf(threshold, Rc::new(Cell::new(0)))
                        .on_gesture_with_revision(super::pan_policy(0.5), (), |event| {
                            Some((10, event))
                        })
                        .id(10)
                })
                .update(move |_, event| observed.borrow_mut().push(event))
                .into_bridge(),
            Vector2::new(200.0, 80.0),
        );
        let a = start(&mut runtime, 1, 20.0);
        let _b = start(&mut runtime, 2, 100.0);
        assert_eq!(
            move_touch(&mut runtime, 1, a, 26.0, 20.0),
            PointerIngressDisposition::RoutedGesture(u64::from(expected))
        );
        assert_eq!(events.borrow().len(), 1);
        assert_eq!(events.borrow()[0].0, expected);
    }
}

#[test]
fn touch_equal_normalized_family_scores_choose_pinch_before_pan() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let mut runtime = SurfaceRuntime::new(
        radiant::app(())
            .view(|_| {
                radiant::application::button("Touch")
                    .filter_mapped(|_| None::<GestureEvent>)
                    .width(120.0)
                    .height(40.0)
                    .id(1)
                    .on_gesture_with_revision(
                        GesturePolicy::none()
                            .recognize(GestureKind::Pan, 4.0)
                            .unwrap()
                            .recognize(GestureKind::Pinch, 0.25)
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
    let _a = start(&mut runtime, 1, 20.0);
    let b = start(&mut runtime, 2, 52.0);
    assert_eq!(
        move_touch(&mut runtime, 2, b, 68.0, 20.0),
        PointerIngressDisposition::RoutedGesture(10)
    );
    assert_eq!(events.borrow()[0].sample().kind(), GestureKind::Pinch);
}

#[test]
fn touch_terminal_can_cross_threshold_and_emits_one_start_and_end() {
    let mut f = fixture();
    let a = start(&mut f.runtime, 1, 20.0);
    let b = start(&mut f.runtime, 2, 100.0);
    f.runtime.dispatch_pointer_ingress(touch(
        2,
        PointerPhase::Ended {
            button: PointerButton::Primary,
        },
        112.0,
        20.0,
        Some(b),
    ));
    assert_eq!(
        f.events
            .borrow()
            .iter()
            .map(|event| event.phase())
            .collect::<Vec<_>>(),
        [GesturePhase::Started, GesturePhase::Ended]
    );
    assert_eq!(f.events.borrow()[1].accumulated(), Vector2::new(6.0, 0.0));
    assert_eq!(
        move_touch(&mut f.runtime, 1, a, 25.0, 20.0),
        PointerIngressDisposition::Stale
    );
}

#[test]
fn touch_focus_loss_cancels_before_late_contact_samples() {
    let mut f = fixture();
    let _a = start(&mut f.runtime, 1, 20.0);
    let b = start(&mut f.runtime, 2, 60.0);
    move_touch(&mut f.runtime, 2, b, 80.0, 20.0);
    assert_eq!(f.runtime.focused_widget(), Some(1));
    f.runtime.clear_focus();
    assert_eq!(f.runtime.focused_widget(), None);
    assert_eq!(
        f.events.borrow().last().unwrap().phase(),
        GesturePhase::Cancelled
    );
    assert_eq!(
        move_touch(&mut f.runtime, 2, b, 90.0, 20.0),
        PointerIngressDisposition::Stale
    );
}
