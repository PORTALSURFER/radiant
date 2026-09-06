use radiant::{
    application::custom_widget_mapped,
    gui::pointer_ingress::{GestureIngress, GestureKind, GesturePhase, GestureUnit, InputDeviceId},
    layout::{Rect, Vector2},
    runtime::{Command, GestureOutcome, GestureRequest, SurfaceRuntime},
    widgets::{
        GestureEvent, GesturePolicy, Widget, WidgetActionCapabilities, WidgetCapabilitiesV2,
        WidgetCommon, WidgetGestures, WidgetInput, WidgetOutput, WidgetSemanticsRevision,
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone)]
struct Probe {
    common: WidgetCommon,
    threshold: f32,
    raw: Rc<Cell<usize>>,
}
impl WidgetGestures for Probe {
    fn revision(&self) -> WidgetSemanticsRevision {
        WidgetSemanticsRevision::exact(self.policy())
    }
    fn policy(&self) -> GesturePolicy {
        GesturePolicy::none()
            .recognize(GestureKind::Pan, self.threshold)
            .unwrap()
            .recognize(GestureKind::Pinch, 0.2)
            .unwrap()
            .recognize(GestureKind::Rotate, 0.2)
            .unwrap()
    }
    fn dispatch(&mut self, event: GestureEvent) -> Option<WidgetOutput> {
        Some(WidgetOutput::typed(event))
    }
}
impl radiant::widgets::WidgetSemanticActions for Probe {
    fn revision(&self) -> WidgetSemanticsRevision {
        WidgetSemanticsRevision::exact(())
    }
    fn supports(&self, action: &radiant::widgets::SemanticAction) -> bool {
        *action == radiant::widgets::SemanticAction::Press
    }
    fn dispatch(
        &mut self,
        _: radiant::widgets::SemanticAction,
        _: radiant::widgets::SemanticActionSource,
    ) -> radiant::widgets::WidgetSemanticActionResult {
        radiant::widgets::WidgetSemanticActionResult::Accepted(None)
    }
}
impl radiant::widgets::WidgetSemantics for Probe {
    fn automation_role(&self) -> radiant::gui::automation::AutomationRole {
        radiant::gui::automation::AutomationRole::Button
    }
}
impl Widget for Probe {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
    fn capabilities(&self) -> radiant::widgets::WidgetCapabilities<'_> {
        radiant::widgets::WidgetCapabilities::new().semantics(self)
    }
    fn capabilities_v2(&self) -> WidgetCapabilitiesV2<'_> {
        WidgetCapabilitiesV2::new()
            .with_gestures(self)
            .with_semantic_actions(self)
    }
    fn action_capabilities(&mut self) -> WidgetActionCapabilities<'_> {
        WidgetActionCapabilities::none().with_handler(self)
    }
    fn preflight_pointer_press(
        &self,
        _: Rect,
        _: &WidgetInput,
    ) -> radiant::widgets::PointerPressAdmission {
        radiant::widgets::PointerPressAdmission::ManagedCapture
    }
    fn retains_managed_pointer_capture(&self) -> bool {
        true
    }
    fn handle_input(&mut self, _: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        if let WidgetInput::FocusChanged(focused) = input {
            self.common.state.focused = focused;
        } else {
            self.raw.set(self.raw.get() + 1);
        }
        None
    }
    fn append_paint(
        &self,
        _: &mut Vec<radiant::runtime::PaintPrimitive>,
        _: Rect,
        _: &radiant::layout::LayoutOutput,
        _: &radiant::theme::ThemeTokens,
    ) {
    }
}
fn sample(kind: GestureKind, phase: GesturePhase, value: Vector2) -> GestureIngress {
    GestureIngress::new(
        kind,
        phase,
        match kind {
            GestureKind::Pan => GestureUnit::LogicalPixels,
            GestureKind::Pinch => GestureUnit::Scale,
            GestureKind::Rotate => GestureUnit::Radians,
        },
        value,
        InputDeviceId::from_host(1).unwrap(),
        Some(radiant::gui::types::Point::new(20.0, 15.0)),
        Default::default(),
        None,
        None,
    )
    .unwrap()
}
fn bridge(
    events: Rc<RefCell<Vec<GestureEvent>>>,
    raw: Rc<Cell<usize>>,
    threshold: Rc<Cell<f32>>,
    shown: Rc<Cell<bool>>,
) -> impl radiant::runtime::RuntimeBridge<GestureEvent> {
    radiant::app(())
        .view(move |_: &()| {
            if !shown.get() {
                return radiant::application::text("Removed").id(2);
            }
            custom_widget_mapped(
                Probe {
                    common: WidgetCommon::fixed(1, 120.0, 40.0).with_keyboard_focus(),
                    threshold: threshold.get(),
                    raw: Rc::clone(&raw),
                },
                |event: GestureEvent| event,
            )
            .id(1)
        })
        .update(move |_, event| events.borrow_mut().push(event))
        .into_bridge()
}
#[test]
fn threshold_crossing_preserves_total_motion_and_one_capture_lifecycle() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let raw = Rc::new(Cell::new(0));
    let mut runtime = SurfaceRuntime::new(
        bridge(
            Rc::clone(&events),
            Rc::clone(&raw),
            Rc::new(Cell::new(5.0)),
            Rc::new(Cell::new(true)),
        ),
        Vector2::new(200.0, 80.0),
    );
    let start = runtime.dispatch_gesture_request(GestureRequest::new(sample(
        GestureKind::Pan,
        GesturePhase::Started,
        Vector2::new(0.0, 0.0),
    )));
    assert_eq!(start.outcome(), &GestureOutcome::Pending);
    let token = start.token().unwrap();
    assert!(events.borrow().is_empty());
    assert_eq!(runtime.focused_widget(), None);
    let moved = |value| {
        GestureRequest::new(sample(GestureKind::Pan, GesturePhase::Changed, value))
            .with_token(token)
    };
    assert_eq!(
        runtime
            .dispatch_gesture_request(moved(Vector2::new(3.0, 0.0)))
            .outcome(),
        &GestureOutcome::Pending
    );
    assert_eq!(
        runtime
            .dispatch_gesture_request(moved(Vector2::new(0.0, 4.0)))
            .outcome(),
        &GestureOutcome::Accepted(1)
    );
    assert_eq!(runtime.focused_widget(), Some(1));
    assert_eq!(events.borrow().len(), 1);
    assert_eq!(events.borrow()[0].phase(), GesturePhase::Started);
    assert_eq!(
        events.borrow()[0].anchor(),
        radiant::gui::types::Point::new(20.0, 15.0)
    );
    assert_eq!(events.borrow()[0].accumulated(), Vector2::new(3.0, 4.0));
    let repeated = runtime.dispatch_gesture_request(GestureRequest::new(sample(
        GestureKind::Pan,
        GesturePhase::Started,
        Vector2::new(0.0, 0.0),
    )));
    assert_eq!(repeated.outcome(), &GestureOutcome::Blocked);
    assert!(repeated.token().is_none());
    let point = radiant::gui::types::Point::new(20.0, 15.0);
    runtime.dispatch_input_at(point, WidgetInput::primary_press(point));
    runtime.dispatch_input_at(point, WidgetInput::primary_release(point));
    assert!(!runtime.dispatch_input(1, WidgetInput::primary_press(point)));
    assert!(!runtime.dispatch_input(1, WidgetInput::primary_release(point)));
    runtime.dispatch_pointer_move_with_outcome(point);
    assert!(!runtime.wheel_or_scroll_at(point, Vector2::new(0.0, 20.0)));
    assert_eq!(raw.get(), 0);
    let end = runtime.dispatch_gesture_request(
        GestureRequest::new(sample(
            GestureKind::Pan,
            GesturePhase::Ended,
            Vector2::new(0.0, 0.0),
        ))
        .with_token(token),
    );
    assert_eq!(end.outcome(), &GestureOutcome::Accepted(1));
    assert!(end.token().is_none());
    assert_eq!(
        events
            .borrow()
            .iter()
            .map(|e| e.phase())
            .collect::<Vec<_>>(),
        [GesturePhase::Started, GesturePhase::Ended]
    );
    assert_eq!(
        runtime
            .dispatch_gesture_request(moved(Vector2::new(1.0, 0.0)))
            .outcome(),
        &GestureOutcome::Stale
    );
}

#[test]
fn policy_replacement_and_removal_cancel_old_owner_once_and_never_revive_tokens() {
    for remove in [false, true] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let threshold = Rc::new(Cell::new(0.0));
        let shown = Rc::new(Cell::new(true));
        let mut runtime = SurfaceRuntime::new(
            bridge(
                Rc::clone(&events),
                Rc::new(Cell::new(0)),
                Rc::clone(&threshold),
                Rc::clone(&shown),
            ),
            Vector2::new(200.0, 80.0),
        );
        let start = runtime.dispatch_gesture_request(GestureRequest::new(sample(
            GestureKind::Pan,
            GesturePhase::Started,
            Vector2::new(0.0, 0.0),
        )));
        assert_eq!(start.outcome(), &GestureOutcome::Accepted(1));
        let token = start.token().unwrap();
        if remove {
            shown.set(false);
        } else {
            threshold.set(3.0);
        }
        runtime.refresh();
        assert_eq!(
            events
                .borrow()
                .iter()
                .map(|e| e.phase())
                .collect::<Vec<_>>(),
            [GesturePhase::Started, GesturePhase::Cancelled]
        );
        assert_eq!(
            events.borrow()[1].cancellation(),
            Some(radiant::widgets::GestureCancellation::Retired)
        );
        shown.set(true);
        threshold.set(0.0);
        runtime.refresh();
        assert_eq!(
            runtime
                .dispatch_gesture_request(
                    GestureRequest::new(sample(
                        GestureKind::Pan,
                        GesturePhase::Changed,
                        Vector2::new(1.0, 0.0)
                    ))
                    .with_token(token)
                )
                .outcome(),
            &GestureOutcome::Stale
        );
        assert_eq!(events.borrow().len(), 2);
        runtime.execute_command(Command::exit());
    }
}

#[test]
fn pinch_rotation_and_shutdown_preserve_phases_and_reject_foreign_authority() {
    for (kind, first, delta, neutral, expected) in [
        (GestureKind::Pinch, 1.0, 1.25, 1.0, 1.25),
        (GestureKind::Rotate, 0.0, -0.25, 0.0, -0.25),
    ] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = SurfaceRuntime::new(
            bridge(
                Rc::clone(&events),
                Rc::new(Cell::new(0)),
                Rc::new(Cell::new(1.0)),
                Rc::new(Cell::new(true)),
            ),
            Vector2::new(200.0, 80.0),
        );
        let mut foreign = SurfaceRuntime::new(
            bridge(
                Rc::new(RefCell::new(Vec::new())),
                Rc::new(Cell::new(0)),
                Rc::new(Cell::new(1.0)),
                Rc::new(Cell::new(true)),
            ),
            Vector2::new(200.0, 80.0),
        );
        let start = GestureRequest::new(sample(
            kind,
            GesturePhase::Started,
            Vector2::new(first, 0.0),
        ));
        let token = runtime.dispatch_gesture_request(start).token().unwrap();
        foreign.dispatch_gesture_request(start);
        let changed = GestureRequest::new(sample(
            kind,
            GesturePhase::Changed,
            Vector2::new(delta, 0.0),
        ))
        .with_token(token);
        assert_eq!(
            foreign.dispatch_gesture_request(changed).outcome(),
            &GestureOutcome::Stale
        );
        assert_eq!(foreign.focused_widget(), None);
        assert_eq!(
            runtime.dispatch_gesture_request(changed).outcome(),
            &GestureOutcome::Accepted(1)
        );
        assert_eq!(events.borrow()[0].accumulated().x, expected);
        let end = GestureRequest::new(sample(
            kind,
            GesturePhase::Ended,
            Vector2::new(neutral, 0.0),
        ))
        .with_token(token);
        runtime.dispatch_gesture_request(end);
        assert_eq!(
            events
                .borrow()
                .iter()
                .map(|e| e.phase())
                .collect::<Vec<_>>(),
            [GesturePhase::Started, GesturePhase::Ended]
        );
        let next = runtime.dispatch_gesture_request(start).token().unwrap();
        runtime.dispatch_gesture_request(
            GestureRequest::new(sample(
                kind,
                GesturePhase::Changed,
                Vector2::new(delta, 0.0),
            ))
            .with_token(next),
        );
        runtime.execute_command(Command::exit());
        runtime.execute_command(Command::exit());
        assert_eq!(
            events
                .borrow()
                .iter()
                .map(|e| e.phase())
                .collect::<Vec<_>>(),
            [
                GesturePhase::Started,
                GesturePhase::Ended,
                GesturePhase::Started,
                GesturePhase::Cancelled
            ]
        );
        assert_eq!(
            events.borrow()[3].cancellation(),
            Some(radiant::widgets::GestureCancellation::CaptureLost)
        );
        assert_eq!(
            runtime.dispatch_gesture_request(changed).outcome(),
            &GestureOutcome::Unavailable
        );
    }
}

#[test]
fn terminal_threshold_crossing_and_overflow_have_exact_terminal_sequences() {
    for overflow in [false, true] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = SurfaceRuntime::new(
            bridge(
                Rc::clone(&events),
                Rc::new(Cell::new(0)),
                Rc::new(Cell::new(if overflow { 0.0 } else { 5.0 })),
                Rc::new(Cell::new(true)),
            ),
            Vector2::new(200.0, 80.0),
        );
        let first = if overflow { f32::MAX } else { 0.0 };
        let token = runtime
            .dispatch_gesture_request(GestureRequest::new(sample(
                GestureKind::Pan,
                GesturePhase::Started,
                Vector2::new(first, 0.0),
            )))
            .token()
            .unwrap();
        let phase = if overflow {
            GesturePhase::Changed
        } else {
            GesturePhase::Ended
        };
        let value = if overflow { f32::MAX } else { 6.0 };
        let terminal = runtime.dispatch_gesture_request(
            GestureRequest::new(sample(GestureKind::Pan, phase, Vector2::new(value, 0.0)))
                .with_token(token),
        );
        assert!(terminal.token().is_none());
        assert_eq!(
            terminal.outcome(),
            if overflow {
                &GestureOutcome::Invalid
            } else {
                &GestureOutcome::Accepted(1)
            }
        );
        assert_eq!(
            events
                .borrow()
                .iter()
                .map(|event| event.phase())
                .collect::<Vec<_>>(),
            [
                GesturePhase::Started,
                if overflow {
                    GesturePhase::Cancelled
                } else {
                    GesturePhase::Ended
                }
            ]
        );
        if overflow {
            assert_eq!(
                events.borrow()[1].cancellation(),
                Some(radiant::widgets::GestureCancellation::InvalidSample)
            );
        }
    }
    for threshold in [f32::NAN, f32::INFINITY, -1.0] {
        assert!(
            GesturePolicy::none()
                .recognize(GestureKind::Pan, threshold)
                .is_err()
        );
    }
}

#[test]
fn combined_handler_dispatches_both_facets_and_semantic_actions_respect_gesture_capture() {
    use radiant::runtime::{SemanticAction, SemanticActionOutcome, SemanticActionSource};
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = SurfaceRuntime::new(
        bridge(
            Rc::clone(&events),
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(0.0)),
            Rc::new(Cell::new(true)),
        ),
        Vector2::new(200.0, 80.0),
    );
    let id = radiant::gui::automation::AutomationNodeId::new("1");
    let target = runtime.semantic_action_target(&id).unwrap();
    assert_eq!(
        runtime.dispatch_semantic_action(
            &target,
            SemanticAction::Press,
            SemanticActionSource::Programmatic
        ),
        SemanticActionOutcome::Accepted
    );
    let started = runtime.dispatch_gesture_request(GestureRequest::new(sample(
        GestureKind::Pan,
        GesturePhase::Started,
        Vector2::new(0.0, 0.0),
    )));
    let token = started.token().unwrap();
    let target = runtime.semantic_action_target(&id).unwrap();
    assert_eq!(
        runtime.dispatch_semantic_action(
            &target,
            SemanticAction::Press,
            SemanticActionSource::Programmatic
        ),
        SemanticActionOutcome::Blocked
    );
    runtime.dispatch_gesture_request(
        GestureRequest::new(sample(
            GestureKind::Pan,
            GesturePhase::Cancelled,
            Vector2::new(0.0, 0.0),
        ))
        .with_token(token),
    );
    let target = runtime.semantic_action_target(&id).unwrap();
    assert_eq!(
        runtime.dispatch_semantic_action(
            &target,
            SemanticAction::Press,
            SemanticActionSource::Programmatic
        ),
        SemanticActionOutcome::Accepted
    );
    assert_eq!(
        events
            .borrow()
            .iter()
            .map(|event| event.phase())
            .collect::<Vec<_>>(),
        [GesturePhase::Started, GesturePhase::Cancelled]
    );
}

#[test]
fn pending_gesture_rechecks_original_anchor_before_claiming_moved_target() {
    let shifted = Rc::new(Cell::new(false));
    let shift = Rc::clone(&shifted);
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&events);
    let mut runtime = SurfaceRuntime::new(
        radiant::app(())
            .view(move |_: &()| {
                radiant::application::column([custom_widget_mapped(
                    Probe {
                        common: WidgetCommon::fixed(1, 120.0, 40.0).with_keyboard_focus(),
                        threshold: 5.0,
                        raw: Rc::new(Cell::new(0)),
                    },
                    |event: GestureEvent| event,
                )
                .id(1)])
                .id(100)
                .padding(if shift.get() { 30.0 } else { 0.0 })
            })
            .update(move |_, event| observed.borrow_mut().push(event))
            .into_bridge(),
        Vector2::new(240.0, 140.0),
    );
    let token = runtime
        .dispatch_gesture_request(GestureRequest::new(sample(
            GestureKind::Pan,
            GesturePhase::Started,
            Vector2::new(0.0, 0.0),
        )))
        .token()
        .unwrap();
    shifted.set(true);
    runtime.refresh();
    assert!(runtime.layout().rects.contains_key(&1));
    assert!(!runtime.layout().rects[&1].contains(radiant::gui::types::Point::new(20.0, 15.0)));
    let moved = runtime.dispatch_gesture_request(
        GestureRequest::new(sample(
            GestureKind::Pan,
            GesturePhase::Changed,
            Vector2::new(6.0, 0.0),
        ))
        .with_token(token),
    );
    assert_eq!(moved.outcome(), &GestureOutcome::Stale);
    assert!(moved.token().is_none());
    assert_eq!(runtime.focused_widget(), None);
    assert!(events.borrow().is_empty());
}

#[test]
fn native_style_missing_anchor_uses_only_the_latched_initial_pointer_position() {
    use radiant::gui::types::Point;
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = SurfaceRuntime::new(
        bridge(
            Rc::clone(&events),
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(0.0)),
            Rc::new(Cell::new(true)),
        ),
        Vector2::new(200.0, 80.0),
    );
    let sample = |phase, scale| {
        GestureIngress::pinch(
            phase,
            scale,
            InputDeviceId::from_host(1).unwrap(),
            None,
            Default::default(),
        )
        .unwrap()
    };
    assert_eq!(
        runtime
            .dispatch_gesture_request(GestureRequest::new(sample(GesturePhase::Started, 1.0)))
            .outcome(),
        &GestureOutcome::Unsupported
    );
    let moved = |position| radiant::runtime::Event::PointerMove {
        position,
        modifiers: Default::default(),
        timestamp: None,
        sequence_range: None,
    };
    runtime.dispatch_event(moved(Point::new(20.0, 15.0)));
    let token = runtime
        .dispatch_gesture_request(GestureRequest::new(sample(GesturePhase::Started, 1.0)))
        .token()
        .unwrap();
    runtime.dispatch_event(moved(Point::new(80.0, 30.0)));
    assert_eq!(
        runtime
            .dispatch_gesture_request(
                GestureRequest::new(sample(GesturePhase::Changed, 1.25)).with_token(token)
            )
            .outcome(),
        &GestureOutcome::Accepted(1)
    );
    assert_eq!(events.borrow()[0].sample().anchor(), None);
    assert_eq!(events.borrow()[0].anchor(), Point::new(20.0, 15.0));
}

#[test]
fn focus_commits_before_cancellation_message_can_request_another_target() {
    use radiant::runtime::{
        RuntimeBridge, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper,
    };
    use radiant::widgets::{ButtonWidget, WidgetSizing};
    struct Bridge {
        events: Rc<RefCell<Vec<GestureEvent>>>,
    }
    impl RuntimeBridge<GestureEvent> for Bridge {
        fn project_surface(&mut self) -> std::sync::Arc<UiSurface<GestureEvent>> {
            super::arc_surface(UiSurface::new(SurfaceNode::column(
                100,
                0.0,
                vec![
                    SurfaceChild::fill(SurfaceNode::widget(
                        Probe {
                            common: WidgetCommon::fixed(1, 100.0, 40.0).with_keyboard_focus(),
                            threshold: 0.0,
                            raw: Rc::new(Cell::new(0)),
                        },
                        WidgetMessageMapper::typed(|event: GestureEvent| event),
                    )),
                    SurfaceChild::fill(SurfaceNode::static_widget(ButtonWidget::new(
                        2,
                        "Requested",
                        WidgetSizing::fixed(Vector2::new(100.0, 40.0)),
                    ))),
                    SurfaceChild::fill(SurfaceNode::static_widget(ButtonWidget::new(
                        3,
                        "From cancellation",
                        WidgetSizing::fixed(Vector2::new(100.0, 40.0)),
                    ))),
                ],
            )))
        }
        fn reduce_message(&mut self, event: GestureEvent) {
            self.events.borrow_mut().push(event);
        }
        fn update(&mut self, event: GestureEvent) -> Command<GestureEvent> {
            self.reduce_message(event);
            if event.phase() == GesturePhase::Cancelled {
                Command::Focus(3)
            } else {
                Command::none()
            }
        }
    }
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = SurfaceRuntime::new(
        Bridge {
            events: Rc::clone(&events),
        },
        Vector2::new(200.0, 180.0),
    );
    assert_eq!(
        runtime
            .dispatch_gesture_request(GestureRequest::new(sample(
                GestureKind::Pan,
                GesturePhase::Started,
                Vector2::new(0.0, 0.0)
            )))
            .outcome(),
        &GestureOutcome::Accepted(1)
    );
    assert!(!runtime.focus_widget(2));
    assert_eq!(runtime.focused_widget(), Some(3));
    assert_eq!(
        events
            .borrow()
            .iter()
            .map(|event| event.phase())
            .collect::<Vec<_>>(),
        [GesturePhase::Started, GesturePhase::Cancelled]
    );
}

#[test]
fn handler_withdrawn_during_focus_is_rejected_before_gesture_capture() {
    #[derive(Clone)]
    struct WithdrawOnFocus(Probe);
    impl Widget for WithdrawOnFocus {
        fn common(&self) -> &WidgetCommon {
            self.0.common()
        }
        fn common_mut(&mut self) -> &mut WidgetCommon {
            self.0.common_mut()
        }
        fn capabilities_v2(&self) -> WidgetCapabilitiesV2<'_> {
            self.0.capabilities_v2()
        }
        fn action_capabilities(&mut self) -> WidgetActionCapabilities<'_> {
            if self.0.common.state.focused {
                WidgetActionCapabilities::none()
            } else {
                self.0.action_capabilities()
            }
        }
        fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
            self.0.handle_input(bounds, input)
        }
        fn append_paint(
            &self,
            _: &mut Vec<radiant::runtime::PaintPrimitive>,
            _: Rect,
            _: &radiant::layout::LayoutOutput,
            _: &radiant::theme::ThemeTokens,
        ) {
        }
    }
    let events = Rc::new(Cell::new(0));
    let observed = Rc::clone(&events);
    let mut runtime = SurfaceRuntime::new(
        radiant::app(())
            .view(|_: &()| {
                custom_widget_mapped(
                    WithdrawOnFocus(Probe {
                        common: WidgetCommon::fixed(1, 120.0, 40.0).with_keyboard_focus(),
                        threshold: 0.0,
                        raw: Rc::new(Cell::new(0)),
                    }),
                    |event: GestureEvent| event,
                )
                .id(1)
            })
            .update(move |_, _| observed.set(observed.get() + 1))
            .into_bridge(),
        Vector2::new(200.0, 80.0),
    );
    let result = runtime.dispatch_gesture_request(GestureRequest::new(sample(
        GestureKind::Pan,
        GesturePhase::Started,
        Vector2::new(1.0, 0.0),
    )));
    assert_eq!(result.outcome(), &GestureOutcome::Unsupported);
    assert!(result.token().is_none());
    assert_eq!(events.get(), 0);
    // No gesture capture was installed; ordinary widget input remains available.
    assert!(runtime.dispatch_input(
        1,
        WidgetInput::pointer_move(radiant::gui::types::Point::new(20.0, 15.0))
    ));
}
