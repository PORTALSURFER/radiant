use super::{RetainedKnobDomainWidget, initial_normalized};
use crate::{
    application::knob_domain,
    gui::types::{Point, Rect, Vector2},
    widgets::{
        EditPhase, KnobDomainCancellationReason, KnobDomainError, KnobDomainKeyboardGesture,
        KnobDomainMappingAttempt, KnobDomainMessage, KnobDomainWheelGesture, KnobWidget,
        NumericAdjustment, NumericStep, NumericStepDirection, ValueFormat, Widget, WidgetInput,
        WidgetKey,
    },
};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TestError {
    Inverse,
    Forward,
    Policy,
}

#[derive(Clone, Copy)]
enum InverseResult {
    Ok,
    Error,
    NonFinite,
    OutOfRange,
}

#[derive(Clone, Copy)]
enum ForwardResult {
    Ok,
    Error,
    NonFinite,
}

#[derive(Clone)]
struct TestAdjustment {
    inverse_calls: Rc<Cell<usize>>,
    forward_calls: Rc<Cell<usize>>,
    inverse: InverseResult,
    default_inverse: Option<InverseResult>,
    forward: ForwardResult,
    fail_forward_until: usize,
}

impl TestAdjustment {
    fn linear() -> Self {
        Self {
            inverse_calls: Rc::new(Cell::new(0)),
            forward_calls: Rc::new(Cell::new(0)),
            inverse: InverseResult::Ok,
            default_inverse: None,
            forward: ForwardResult::Ok,
            fail_forward_until: 0,
        }
    }
}

impl NumericAdjustment<f32> for TestAdjustment {
    type Error = TestError;

    fn normalized_to_value(&self, normalized: f32) -> Result<f32, Self::Error> {
        let calls = self.forward_calls.get() + 1;
        self.forward_calls.set(calls);
        if calls <= self.fail_forward_until {
            return Err(TestError::Forward);
        }
        match self.forward {
            ForwardResult::Ok => Ok(10.0 + normalized * 100.0),
            ForwardResult::Error => Err(TestError::Forward),
            ForwardResult::NonFinite => Ok(f32::NAN),
        }
    }

    fn value_to_normalized(&self, value: &f32) -> Result<f32, Self::Error> {
        let calls = self.inverse_calls.get() + 1;
        self.inverse_calls.set(calls);
        let inverse = if *value == 10.0 {
            self.default_inverse.unwrap_or(self.inverse)
        } else {
            self.inverse
        };
        match inverse {
            InverseResult::Ok => Ok((*value - 10.0) / 100.0),
            InverseResult::Error => Err(TestError::Inverse),
            InverseResult::NonFinite => Ok(f32::NAN),
            InverseResult::OutOfRange => Ok(2.0),
        }
    }

    fn step(
        &self,
        _value: &f32,
        _direction: NumericStepDirection,
        _step: NumericStep,
    ) -> Result<f32, Self::Error> {
        Err(TestError::Policy)
    }

    fn scrub(
        &self,
        _value: &f32,
        _normalized_delta: f32,
        _step: NumericStep,
    ) -> Result<f32, Self::Error> {
        Err(TestError::Policy)
    }

    fn wheel(&self, _value: &f32, _delta: f32, _step: NumericStep) -> Result<f32, Self::Error> {
        Err(TestError::Policy)
    }
}

fn bounds() -> Rect {
    Rect::from_min_size(Point::default(), Vector2::new(40.0, 40.0))
}

fn domain_knob(adjustment: TestAdjustment) -> RetainedKnobDomainWidget<TestAdjustment> {
    let domain_value = 20.0;
    let default_domain_value = 10.0;
    let normalized_value = initial_normalized(domain_value, &adjustment).expect("current value");
    let default_normalized_value =
        initial_normalized(default_domain_value, &adjustment).expect("default value");
    let knob = KnobWidget::new(1, normalized_value)
        .with_default_value(default_normalized_value)
        .with_sensitivity(0.01);
    RetainedKnobDomainWidget::new(
        knob,
        Rc::new(adjustment),
        domain_value,
        default_domain_value,
        default_normalized_value,
    )
}

fn pointer_move() -> WidgetInput {
    WidgetInput::pointer_move(Point::new(20.0, 10.0))
}

fn phases<E>(message: &KnobDomainMessage<E>) -> Option<Vec<EditPhase>> {
    match message {
        KnobDomainMessage::KeyboardGesture(KnobDomainKeyboardGesture { .. })
        | KnobDomainMessage::WheelGesture(KnobDomainWheelGesture { .. }) => {
            Some(vec![EditPhase::Begin, EditPhase::Update, EditPhase::Commit])
        }
        _ => None,
    }
}

#[test]
fn domain_construction_and_default_inverse_are_checked_once_without_cloning_errors() {
    let adjustment = TestAdjustment::linear();
    let inverse_calls = Rc::clone(&adjustment.inverse_calls);
    let builder = knob_domain(20.0, adjustment).expect("current inverse should succeed");
    assert_eq!(inverse_calls.get(), 1);
    let _builder = builder
        .default_value(10.0)
        .expect("default inverse should succeed");
    assert_eq!(inverse_calls.get(), 2);

    let mut nonfinite_value = TestAdjustment::linear();
    let calls = Rc::clone(&nonfinite_value.inverse_calls);
    let error = knob_domain(f32::NAN, nonfinite_value)
        .err()
        .expect("NaN must be rejected");
    assert!(matches!(error, KnobDomainError::NonFiniteValue { value } if value.is_nan()));
    assert_eq!(calls.get(), 0);

    nonfinite_value = TestAdjustment::linear();
    nonfinite_value.inverse = InverseResult::Error;
    assert_eq!(
        knob_domain(20.0, nonfinite_value)
            .err()
            .expect("inverse errors must propagate"),
        KnobDomainError::ValueToNormalized {
            error: TestError::Inverse
        }
    );

    let mut nonfinite_inverse = TestAdjustment::linear();
    nonfinite_inverse.inverse = InverseResult::NonFinite;
    let error = knob_domain(20.0, nonfinite_inverse)
        .err()
        .expect("NaN inverse must be rejected");
    assert!(
        matches!(error, KnobDomainError::NonFiniteNormalized { normalized } if normalized.is_nan())
    );

    let mut out_of_range = TestAdjustment::linear();
    out_of_range.inverse = InverseResult::OutOfRange;
    assert_eq!(
        knob_domain(20.0, out_of_range)
            .err()
            .expect("out-of-range inverse must be rejected"),
        KnobDomainError::NormalizedOutOfRange { normalized: 2.0 }
    );

    let mut default_error_adjustment = TestAdjustment::linear();
    default_error_adjustment.default_inverse = Some(InverseResult::Error);
    let inverse_calls = Rc::clone(&default_error_adjustment.inverse_calls);
    let builder =
        knob_domain(20.0, default_error_adjustment).expect("current inverse should succeed");
    assert_eq!(inverse_calls.get(), 1);
    let error = builder
        .default_value(10.0)
        .err()
        .expect("default inverse errors must propagate");
    assert_eq!(
        error,
        KnobDomainError::ValueToNormalized {
            error: TestError::Inverse
        }
    );
    assert_eq!(inverse_calls.get(), 2);

    let mut nonfinite_default = TestAdjustment::linear();
    nonfinite_default.default_inverse = Some(InverseResult::NonFinite);
    let error = knob_domain(20.0, nonfinite_default)
        .expect("current inverse should succeed")
        .default_value(10.0)
        .err()
        .expect("nonfinite default inverse must be rejected");
    assert!(matches!(
        error,
        KnobDomainError::NonFiniteNormalized { normalized } if normalized.is_nan()
    ));

    let mut out_of_range_default = TestAdjustment::linear();
    out_of_range_default.default_inverse = Some(InverseResult::OutOfRange);
    assert_eq!(
        knob_domain(20.0, out_of_range_default)
            .expect("current inverse should succeed")
            .default_value(10.0)
            .err()
            .expect("out-of-range default inverse must be rejected"),
        KnobDomainError::NormalizedOutOfRange { normalized: 2.0 }
    );
}

#[test]
fn domain_pointer_lifecycle_maps_once_and_preserves_metadata() {
    let mut adjustment = TestAdjustment::linear();
    let forward_calls = Rc::clone(&adjustment.forward_calls);
    let mut knob = domain_knob(adjustment.clone());
    let bounds = bounds();

    assert!(matches!(
        knob.handle_domain_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0))),
        Some(KnobDomainMessage::GestureStarted { value: 20.0, .. })
    ));
    assert_eq!(forward_calls.get(), 0);
    assert!(matches!(
        knob.handle_domain_input(bounds, pointer_move()),
        Some(KnobDomainMessage::ValueChanged { value, .. }) if (value - 30.0).abs() < 0.0001
    ));
    assert_eq!(forward_calls.get(), 1);
    assert!(matches!(
        knob.handle_domain_input(bounds, WidgetInput::primary_release(Point::new(20.0, 10.0))),
        Some(KnobDomainMessage::GestureEnded { value, .. }) if (value - 30.0).abs() < 0.0001
    ));
    assert_eq!(forward_calls.get(), 1);

    adjustment.forward = ForwardResult::Error;
    let _ = adjustment;
}

#[test]
fn domain_pointer_mapping_failure_restores_full_state_and_external_retry_can_succeed() {
    let mut adjustment = TestAdjustment::linear();
    adjustment.fail_forward_until = 1;
    let forward_calls = Rc::clone(&adjustment.forward_calls);
    let mut knob = domain_knob(adjustment);
    let bounds = bounds();
    let _ = knob.handle_domain_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0)));
    let before = knob.clone();

    let Some(KnobDomainMessage::MappingFailed {
        attempt,
        normalized,
        retained_value,
        error,
        ..
    }) = knob.handle_domain_input(bounds, pointer_move())
    else {
        panic!("failed pointer mapping should be reported");
    };
    assert_eq!(attempt, KnobDomainMappingAttempt::PointerUpdate);
    assert!((normalized - 0.2).abs() < 0.0001);
    assert_eq!(retained_value, 20.0);
    assert_eq!(
        error,
        KnobDomainError::NormalizedToValue {
            error: TestError::Forward
        }
    );
    assert_eq!(forward_calls.get(), 1);
    assert_eq!(knob.knob, before.knob);
    assert_eq!(knob.active_edit, before.active_edit);
    assert_eq!(knob.active_domain_start, before.active_domain_start);
    assert_eq!(knob.domain_value, before.domain_value);

    assert!(matches!(
        knob.handle_domain_input(bounds, pointer_move()),
        Some(KnobDomainMessage::ValueChanged { value, .. }) if (value - 30.0).abs() < 0.0001
    ));
    assert_eq!(forward_calls.get(), 2);
    assert!(matches!(
        knob.handle_domain_input(bounds, WidgetInput::primary_release(Point::new(20.0, 10.0))),
        Some(KnobDomainMessage::GestureEnded { value, .. }) if (value - 30.0).abs() < 0.0001
    ));

    let mut nonfinite = TestAdjustment::linear();
    nonfinite.forward = ForwardResult::NonFinite;
    let mut nonfinite_knob = domain_knob(nonfinite);
    let _ = nonfinite_knob
        .handle_domain_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0)));
    assert!(matches!(
        nonfinite_knob.handle_domain_input(bounds, pointer_move()),
        Some(KnobDomainMessage::MappingFailed {
            error: KnobDomainError::NonFiniteValue { value },
            ..
        }) if value.is_nan()
    ));
}

#[test]
fn domain_focus_capture_and_disabled_cancellation_are_explicit_and_restore_start() {
    let bounds = bounds();
    let mut focus_loss = domain_knob(TestAdjustment::linear());
    let _ =
        focus_loss.handle_domain_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0)));
    let _ = focus_loss.handle_domain_input(bounds, pointer_move());
    assert!(matches!(
        focus_loss.handle_domain_input(bounds, WidgetInput::FocusChanged(false)),
        Some(KnobDomainMessage::GestureCancelled {
            start_value: 20.0,
            previous_value,
            reason: KnobDomainCancellationReason::FocusLoss,
            ..
        }) if (previous_value - 30.0).abs() < 0.0001
    ));
    assert_eq!(focus_loss.domain_value, 20.0);
    assert_eq!(focus_loss.knob.state.value, 0.1);
    assert!(!focus_loss.knob.common.state.pressed);

    let mut capture_loss = domain_knob(TestAdjustment::linear());
    let _ = capture_loss
        .handle_domain_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0)));
    let _ = capture_loss.handle_domain_input(bounds, pointer_move());
    let output = Widget::handle_pointer_capture_cancelled(&mut capture_loss, bounds)
        .expect("capture loss should emit a domain cancellation");
    let message = output
        .typed_cloned::<KnobDomainMessage<TestError>>()
        .expect("capture output should be typed");
    assert!(matches!(
        message,
        KnobDomainMessage::GestureCancelled {
            reason: KnobDomainCancellationReason::PointerCaptureLoss,
            previous_value,
            start_value: 20.0,
            ..
        } if (previous_value - 30.0).abs() < 0.0001
    ));
    assert_eq!(capture_loss.domain_value, 20.0);
    assert!(capture_loss.knob.common.state.focused);

    let mut disabled = domain_knob(TestAdjustment::linear());
    let _ =
        disabled.handle_domain_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0)));
    let _ = disabled.handle_domain_input(bounds, pointer_move());
    disabled.knob.common.state.disabled = true;
    assert!(matches!(
        disabled.handle_domain_input(bounds, WidgetInput::primary_release(Point::new(20.0, 10.0))),
        Some(KnobDomainMessage::GestureCancelled {
            reason: KnobDomainCancellationReason::DisabledOrReadOnly,
            previous_value,
            start_value: 20.0,
            ..
        }) if (previous_value - 30.0).abs() < 0.0001
    ));
    assert_eq!(disabled.domain_value, 20.0);
    assert!(!disabled.knob.common.state.pressed);

    let mut disabled_key = domain_knob(TestAdjustment::linear());
    let _ = disabled_key
        .handle_domain_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0)));
    let _ = disabled_key.handle_domain_input(bounds, pointer_move());
    disabled_key.knob.common.state.read_only = true;
    assert!(matches!(
        disabled_key.handle_domain_input(
            bounds,
            WidgetInput::key_press(WidgetKey::ArrowRight),
        ),
        Some(KnobDomainMessage::GestureCancelled {
            reason: KnobDomainCancellationReason::DisabledOrReadOnly,
            previous_value,
            start_value: 20.0,
            ..
        }) if (previous_value - 30.0).abs() < 0.0001
    ));
    assert_eq!(disabled_key.domain_value, 20.0);
}

#[test]
fn domain_keyboard_and_wheel_gestures_are_atomic_and_mapping_failures_are_typed() {
    let bounds = bounds();
    let mut keyboard = TestAdjustment::linear();
    keyboard.fail_forward_until = 1;
    let forward_calls = Rc::clone(&keyboard.forward_calls);
    let mut keyboard_knob = domain_knob(keyboard);
    let _ = keyboard_knob.handle_domain_input(bounds, WidgetInput::FocusChanged(true));
    let failed =
        keyboard_knob.handle_domain_input(bounds, WidgetInput::key_press(WidgetKey::ArrowRight));
    assert!(matches!(
        failed,
        Some(KnobDomainMessage::MappingFailed {
            attempt: KnobDomainMappingAttempt::KeyboardGesture,
            normalized,
            retained_value: 20.0,
            error: KnobDomainError::NormalizedToValue {
                error: TestError::Forward
            },
            ..
        }) if (normalized - 0.26).abs() < 0.0001
    ));
    assert_eq!(keyboard_knob.domain_value, 20.0);
    assert_eq!(keyboard_knob.knob.state.value, 0.1);
    assert_eq!(forward_calls.get(), 1);
    let success = keyboard_knob
        .handle_domain_input(bounds, WidgetInput::key_press(WidgetKey::ArrowRight))
        .expect("retry should be an independent accepted input");
    let KnobDomainMessage::KeyboardGesture(gesture) = success else {
        panic!("keyboard success should be atomic");
    };
    assert_eq!(
        gesture.events[0],
        crate::widgets::KnobDomainAutomationEvent::GestureStarted { value: 20.0 }
    );
    assert_eq!(
        gesture.events[1],
        crate::widgets::KnobDomainAutomationEvent::ValueChanged { value: 36.0 }
    );
    assert_eq!(
        gesture.events[2],
        crate::widgets::KnobDomainAutomationEvent::GestureEnded { value: 36.0 }
    );
    assert_eq!(
        phases(&KnobDomainMessage::<TestError>::KeyboardGesture(gesture)),
        Some(vec![EditPhase::Begin, EditPhase::Update, EditPhase::Commit])
    );

    let mut wheel_success = domain_knob(TestAdjustment::linear());
    let success = wheel_success
        .handle_domain_input(
            bounds,
            WidgetInput::wheel(
                Point::new(20.0, 20.0),
                Vector2::new(0.0, 120.0),
                crate::widgets::PointerModifiers::default(),
            ),
        )
        .expect("wheel success should be atomic");
    let KnobDomainMessage::WheelGesture(gesture) = success else {
        panic!("wheel success should be atomic");
    };
    assert_eq!(
        gesture.events,
        [
            crate::widgets::KnobDomainAutomationEvent::GestureStarted { value: 20.0 },
            crate::widgets::KnobDomainAutomationEvent::ValueChanged { value: 25.0 },
            crate::widgets::KnobDomainAutomationEvent::GestureEnded { value: 25.0 },
        ]
    );
    assert_eq!(wheel_success.domain_value, 25.0);

    let mut wheel = TestAdjustment::linear();
    wheel.forward = ForwardResult::Error;
    let mut wheel_knob = domain_knob(wheel);
    assert!(matches!(
        wheel_knob.handle_domain_input(
            bounds,
            WidgetInput::wheel(
                Point::new(20.0, 20.0),
                Vector2::new(0.0, 120.0),
                crate::widgets::PointerModifiers::default(),
            ),
        ),
        Some(KnobDomainMessage::MappingFailed {
            attempt: KnobDomainMappingAttempt::WheelGesture,
            normalized,
            retained_value: 20.0,
            error: KnobDomainError::NormalizedToValue {
                error: TestError::Forward
            },
            ..
        }) if (normalized - 0.15).abs() < 0.0001
    ));
    assert_eq!(wheel_knob.domain_value, 20.0);
    assert_eq!(wheel_knob.knob.state.value, 0.1);
}

#[test]
fn domain_reset_uses_cached_default_without_remapping_and_emits_noop_reset() {
    let mut knob = domain_knob(TestAdjustment::linear());
    let bounds = bounds();
    let _ = knob.handle_domain_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0)));
    let _ = knob.handle_domain_input(bounds, pointer_move());
    let _ = knob.handle_domain_input(bounds, WidgetInput::primary_release(Point::new(20.0, 10.0)));
    let forward_calls = knob.adjustment.forward_calls.get();

    assert!(matches!(
        knob.handle_domain_input(
            bounds,
            WidgetInput::primary_double_click(Point::new(20.0, 20.0)),
        ),
        Some(KnobDomainMessage::Reset {
            previous_value,
            value: 10.0,
            ..
        }) if (previous_value - 30.0).abs() < 0.0001
    ));
    assert_eq!(knob.adjustment.forward_calls.get(), forward_calls);
    assert_eq!(knob.domain_value, 10.0);
    assert_eq!(knob.knob.state.value, 0.0);
    assert!(matches!(
        knob.handle_domain_input(
            bounds,
            WidgetInput::primary_double_click(Point::new(20.0, 20.0)),
        ),
        Some(KnobDomainMessage::Reset {
            previous_value: 10.0,
            value: 10.0,
            ..
        })
    ));
    assert_eq!(knob.adjustment.forward_calls.get(), forward_calls);
}

#[test]
fn domain_formatting_and_reprojection_keep_domain_display_and_interaction_state() {
    let mut knob =
        domain_knob(TestAdjustment::linear()).with_value_format(Some(ValueFormat::frequency()));
    assert_eq!(
        Widget::automation_semantics(&knob).value_text.as_deref(),
        Some("20.00 Hz")
    );

    let bounds = bounds();
    let _ = knob.handle_domain_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0)));
    let mut refreshed = domain_knob(TestAdjustment::linear());
    refreshed.domain_value = 30.0;
    refreshed.knob.state.value = 0.2;
    refreshed.synchronize_from_previous(&knob);
    assert!(refreshed.active_edit.is_some());
    assert_eq!(refreshed.domain_value, 30.0);
    assert!(refreshed.knob.common.state.pressed);
    assert!(matches!(
        refreshed.handle_domain_input(bounds, pointer_move()),
        Some(KnobDomainMessage::ValueChanged { value: 40.0, .. })
    ));
}
