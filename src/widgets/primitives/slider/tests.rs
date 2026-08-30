use super::{geometry::value_for_position, *};
use crate::widgets::WidgetPointerMotion;
use crate::{
    gui::input::{InputSequence, InputSequenceRange, InputTimestamp},
    gui::types::{Point, Vector2},
    layout::LayoutOutput,
    runtime::PaintPrimitive,
    theme::ThemeTokens,
    widgets::contract::Widget,
    widgets::interaction::{
        EditEvent, EditPhase, InteractionProvenance, KeyboardModifiers, NumericAdjustment,
        NumericStep, NumericStepDirection, PointerButton, PointerModifiers, SliderDomainError,
        SliderDomainMessage, SliderEditBatch, SliderMessage, ValueFormat, WidgetInput, WidgetKey,
        WidgetOutput,
    },
};
use std::{cell::Cell, fmt::Debug, rc::Rc};

fn bounds() -> Rect {
    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 28.0))
}

fn pointer_provenance(
    modifiers: PointerModifiers,
    timestamp: Option<InputTimestamp>,
    sequence_range: Option<InputSequenceRange>,
) -> InteractionProvenance {
    InteractionProvenance::Pointer {
        modifiers,
        timestamp,
        sequence_range,
    }
}

fn phases(batch: SliderEditBatch) -> Vec<EditPhase> {
    batch.events().iter().map(|event| event.phase).collect()
}

fn retained_slider(id: u64, value: f32) -> RetainedSliderWidget {
    RetainedSliderWidget::new(SliderWidget::new(
        id,
        value,
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DomainAdjustmentError {
    Inverse,
    Forward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DomainAdjustmentMode {
    Linear,
    InverseError,
    InverseNonFinite,
    InverseOutOfRange,
    ForwardError,
    ForwardErrorAfterFirst,
    ForwardNonFinite,
}

#[derive(Clone)]
struct TestDomainAdjustment {
    mode: DomainAdjustmentMode,
    inverse_calls: Rc<Cell<usize>>,
    forward_calls: Rc<Cell<usize>>,
}

impl TestDomainAdjustment {
    fn new(mode: DomainAdjustmentMode) -> (Self, Rc<Cell<usize>>, Rc<Cell<usize>>) {
        let inverse_calls = Rc::new(Cell::new(0));
        let forward_calls = Rc::new(Cell::new(0));
        (
            Self {
                mode,
                inverse_calls: Rc::clone(&inverse_calls),
                forward_calls: Rc::clone(&forward_calls),
            },
            inverse_calls,
            forward_calls,
        )
    }
}

impl NumericAdjustment<f32> for TestDomainAdjustment {
    type Error = DomainAdjustmentError;

    fn normalized_to_value(&self, normalized: f32) -> Result<f32, Self::Error> {
        let call = self.forward_calls.get() + 1;
        self.forward_calls.set(call);
        match self.mode {
            DomainAdjustmentMode::ForwardError => Err(DomainAdjustmentError::Forward),
            DomainAdjustmentMode::ForwardErrorAfterFirst if call > 1 => {
                Err(DomainAdjustmentError::Forward)
            }
            DomainAdjustmentMode::ForwardNonFinite => Ok(f32::NAN),
            _ => Ok(10.0 + normalized * 100.0),
        }
    }

    fn value_to_normalized(&self, value: &f32) -> Result<f32, Self::Error> {
        self.inverse_calls.set(self.inverse_calls.get() + 1);
        match self.mode {
            DomainAdjustmentMode::InverseError => Err(DomainAdjustmentError::Inverse),
            DomainAdjustmentMode::InverseNonFinite => Ok(f32::NAN),
            DomainAdjustmentMode::InverseOutOfRange => Ok(2.0),
            _ => Ok((*value - 10.0) / 100.0),
        }
    }

    fn step(
        &self,
        _value: &f32,
        _direction: NumericStepDirection,
        _step: NumericStep,
    ) -> Result<f32, Self::Error> {
        panic!("domain slider must not invoke adjustment steps")
    }

    fn scrub(
        &self,
        _value: &f32,
        _normalized_delta: f32,
        _step: NumericStep,
    ) -> Result<f32, Self::Error> {
        panic!("domain slider must not invoke adjustment scrubbing")
    }

    fn wheel(&self, _value: &f32, _delta: f32, _step: NumericStep) -> Result<f32, Self::Error> {
        panic!("domain slider must not invoke adjustment wheel changes")
    }
}

fn retained_domain_slider(
    adjustment: TestDomainAdjustment,
    domain_value: f32,
) -> RetainedSliderDomainWidget<TestDomainAdjustment> {
    let normalized = initial_normalized(domain_value, &adjustment).expect("valid domain value");
    RetainedSliderDomainWidget::new(
        SliderWidget::new(
            140,
            normalized,
            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
        ),
        Rc::new(adjustment),
        domain_value,
    )
}

#[test]
fn slider_domain_maps_endpoints_and_midpoint_once_without_adjustment_actions() {
    let (adjustment, inverse_calls, forward_calls) =
        TestDomainAdjustment::new(DomainAdjustmentMode::Linear);
    let mut slider = retained_domain_slider(adjustment, 10.0);
    assert_eq!(inverse_calls.get(), 1);

    let Some(SliderDomainMessage::ValueChanged { value }) =
        slider.handle_domain_input(bounds(), WidgetInput::primary_press(Point::new(60.0, 14.0)))
    else {
        panic!("midpoint should map to a domain value");
    };
    assert_eq!(value, 60.0);
    assert_eq!(forward_calls.get(), 1);

    let _ = slider.handle_domain_input(
        bounds(),
        WidgetInput::primary_release(Point::new(60.0, 14.0)),
    );
    let Some(SliderDomainMessage::ValueChanged { value }) = slider.handle_domain_input(
        bounds(),
        WidgetInput::primary_press(Point::new(120.0, 14.0)),
    ) else {
        panic!("upper endpoint should map to a domain value");
    };
    assert_eq!(value, 110.0);
    assert_eq!(forward_calls.get(), 2);
}

#[test]
fn slider_domain_inverse_results_are_checked_without_clamping() {
    let (adjustment, inverse_calls, _) =
        TestDomainAdjustment::new(DomainAdjustmentMode::InverseError);
    assert_eq!(
        initial_normalized(10.0, &adjustment),
        Err(SliderDomainError::ValueToNormalized {
            error: DomainAdjustmentError::Inverse,
        })
    );
    assert_eq!(inverse_calls.get(), 1);

    let (adjustment, inverse_calls, _) = TestDomainAdjustment::new(DomainAdjustmentMode::Linear);
    assert!(matches!(
        initial_normalized(f32::NAN, &adjustment),
        Err(SliderDomainError::NonFiniteValue { value }) if value.is_nan()
    ));
    assert_eq!(inverse_calls.get(), 0);

    let (adjustment, inverse_calls, _) =
        TestDomainAdjustment::new(DomainAdjustmentMode::InverseNonFinite);
    assert!(matches!(
        initial_normalized(10.0, &adjustment),
        Err(SliderDomainError::NonFiniteNormalized { normalized }) if normalized.is_nan()
    ));
    assert_eq!(inverse_calls.get(), 1);

    let (adjustment, inverse_calls, _) =
        TestDomainAdjustment::new(DomainAdjustmentMode::InverseOutOfRange);
    assert_eq!(
        initial_normalized(10.0, &adjustment),
        Err(SliderDomainError::NormalizedOutOfRange { normalized: 2.0 })
    );
    assert_eq!(inverse_calls.get(), 1);
}

#[test]
fn slider_domain_forward_failures_do_not_advance_normalized_or_displayed_value() {
    let (adjustment, _, forward_calls) =
        TestDomainAdjustment::new(DomainAdjustmentMode::ForwardError);
    let mut slider =
        retained_domain_slider(adjustment, 10.0).with_value_format(Some(ValueFormat::decimal(1)));
    let before = slider.slider.slider.state.value;
    assert_eq!(
        slider
            .handle_domain_input(bounds(), WidgetInput::primary_press(Point::new(60.0, 14.0)),)
            .expect("mapping failure should be typed"),
        SliderDomainMessage::MappingFailed {
            normalized: 0.5,
            error: SliderDomainError::NormalizedToValue {
                error: DomainAdjustmentError::Forward,
            },
        }
    );
    assert_eq!(forward_calls.get(), 1);
    assert_eq!(slider.slider.slider.state.value, before);
    assert!(!slider.slider.slider.common.state.pressed);
    assert_eq!(
        slider.automation_semantics().value_text.as_deref(),
        Some("10.0")
    );

    let (adjustment, _, forward_calls) =
        TestDomainAdjustment::new(DomainAdjustmentMode::ForwardNonFinite);
    let mut slider = retained_domain_slider(adjustment, 10.0);
    assert!(matches!(
        slider
            .handle_domain_input(
                bounds(),
                WidgetInput::primary_press(Point::new(60.0, 14.0)),
            )
            .expect("mapping failure should be typed"),
        SliderDomainMessage::MappingFailed {
            normalized: 0.5,
            error: SliderDomainError::NonFiniteValue { value }
        } if value.is_nan()
    ));
    assert_eq!(forward_calls.get(), 1);
    assert_eq!(slider.slider.slider.state.value, 0.0);
    assert_eq!(
        slider.handle_domain_input(bounds(), WidgetInput::pointer_move(Point::new(96.0, 14.0)),),
        None
    );
    assert_eq!(forward_calls.get(), 1);
}

#[test]
fn slider_domain_terminal_release_failure_preserves_value_but_finishes_edit() {
    let (adjustment, _, forward_calls) =
        TestDomainAdjustment::new(DomainAdjustmentMode::ForwardErrorAfterFirst);
    let mut slider = retained_domain_slider(adjustment, 10.0);

    assert_eq!(
        slider.handle_domain_input(bounds(), WidgetInput::primary_press(Point::new(60.0, 14.0)),),
        Some(SliderDomainMessage::ValueChanged { value: 60.0 })
    );
    assert_eq!(forward_calls.get(), 1);
    assert!(slider.slider.slider.common.state.pressed);
    assert_eq!(
        slider.automation_semantics().value_text.as_deref(),
        Some("60.000")
    );

    assert!(matches!(
        slider.handle_domain_input(
            bounds(),
            WidgetInput::primary_release(Point::new(72.0, 14.0)),
        ),
        Some(SliderDomainMessage::MappingFailed {
            normalized: 0.6,
            error: SliderDomainError::NormalizedToValue {
                error: DomainAdjustmentError::Forward,
            },
        })
    ));
    assert_eq!(forward_calls.get(), 2);
    assert_eq!(slider.slider.slider.state.value, 0.5);
    assert!(!slider.slider.slider.common.state.pressed);
    assert_eq!(
        slider.automation_semantics().value_text.as_deref(),
        Some("60.000")
    );

    assert_eq!(
        slider.handle_domain_input(bounds(), WidgetInput::pointer_move(Point::new(96.0, 14.0)),),
        None
    );
    assert_eq!(forward_calls.get(), 2);
}

#[test]
fn slider_domain_terminal_capture_failure_keeps_focus_cleanup() {
    let (adjustment, _, forward_calls) =
        TestDomainAdjustment::new(DomainAdjustmentMode::ForwardErrorAfterFirst);
    let mut slider = retained_domain_slider(adjustment, 10.0);

    assert!(matches!(
        slider.handle_domain_input(bounds(), WidgetInput::primary_press(Point::new(60.0, 14.0)),),
        Some(SliderDomainMessage::ValueChanged { value: 60.0 })
    ));
    assert!(slider.slider.slider.common.state.pressed);
    assert_eq!(
        slider.automation_semantics().value_text.as_deref(),
        Some("60.000")
    );

    assert!(Widget::handle_pointer_capture_cancelled(&mut slider, bounds()).is_some());
    assert_eq!(forward_calls.get(), 2);
    assert_eq!(slider.slider.slider.state.value, 0.5);
    assert!(!slider.slider.slider.common.state.pressed);
    assert_eq!(
        slider.automation_semantics().value_text.as_deref(),
        Some("60.000")
    );
    assert_eq!(
        slider.handle_domain_input(bounds(), WidgetInput::pointer_move(Point::new(96.0, 14.0)),),
        None
    );
    assert_eq!(forward_calls.get(), 2);
}

#[test]
fn retained_slider_formats_automation_text_and_keeps_fallback_text() {
    let mut slider = retained_slider(14, 0.125).with_value_format(Some(ValueFormat::percent(1)));

    assert_eq!(slider.slider.state.value, 0.125);
    assert_eq!(
        slider.automation_semantics().value_text.as_deref(),
        Some("12.5%")
    );

    slider.slider.state.value = f32::NAN;
    assert_eq!(
        slider.automation_semantics().value_text.as_deref(),
        Some("NaN")
    );
}

#[test]
fn slider_pointer_drag_emits_clamped_values() {
    let mut slider = SliderWidget::new(9, 0.25, WidgetSizing::fixed(Vector2::new(120.0, 28.0)));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 28.0));

    assert_eq!(
        slider.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(60.0, 14.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        ),
        Some(SliderMessage::ValueChanged { value: 0.5 })
    );
    assert_eq!(
        slider.handle_input(bounds, WidgetInput::pointer_move(Point::new(180.0, 14.0)),),
        Some(SliderMessage::ValueChanged { value: 1.0 })
    );
}

#[test]
fn slider_accepts_runtime_pointer_move_for_live_dragging() {
    let slider = SliderWidget::new(9, 0.25, WidgetSizing::fixed(Vector2::new(120.0, 28.0)));

    assert!(WidgetPointerMotion::accepts_pointer_move(&slider));
}

#[test]
fn slider_paints_progress_track_without_thumb_handle() {
    let slider = SliderWidget::new(9, 0.25, WidgetSizing::fixed(Vector2::new(120.0, 28.0)));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 28.0));
    let mut primitives = Vec::new();

    slider.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    let fill_rects = primitives
        .iter()
        .filter(|primitive| matches!(primitive, PaintPrimitive::FillRect(_)))
        .count();
    let stroke_rects = primitives
        .iter()
        .filter(|primitive| matches!(primitive, PaintPrimitive::StrokeRect(_)))
        .count();

    assert_eq!(fill_rects, 2);
    assert_eq!(stroke_rects, 0);
}

#[test]
fn slider_can_paint_a_custom_height_track_border() {
    let slider = SliderWidget::new(9, 0.0, WidgetSizing::fixed(Vector2::new(112.0, 16.0)))
        .with_track_height(8.0)
        .with_track_border(true);
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(112.0, 16.0));
    let mut primitives = Vec::new();

    slider.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    let borders = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::StrokeRect(stroke) => Some(stroke),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(borders.len(), 1);
    assert_eq!(borders[0].rect.width(), 112.0);
    assert_eq!(borders[0].rect.height(), 8.0);
    assert_eq!(borders[0].width, 1.0);
    assert_eq!(borders[0].color, ThemeTokens::default().border_emphasis);
}

#[test]
fn focused_slider_responds_to_keyboard_steps() {
    let mut slider = SliderWidget::new(10, 0.5, WidgetSizing::fixed(Vector2::new(120.0, 28.0)));

    let _ = slider.handle_input(Rect::default(), WidgetInput::FocusChanged(true));
    let Some(SliderMessage::ValueChanged { value }) = slider.handle_input(
        Rect::default(),
        WidgetInput::key_press(WidgetKey::ArrowRight),
    ) else {
        panic!("focused slider should emit an arrow-key change");
    };
    assert!((value - 0.55).abs() < f32::EPSILON);
    assert_eq!(
        slider.handle_input(Rect::default(), WidgetInput::key_press(WidgetKey::Home)),
        Some(SliderMessage::ValueChanged { value: 0.0 })
    );
}

#[test]
fn slider_automation_marker_is_state_gated() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(120.0, 28.0));
    let slider = SliderWidget::new(11, 0.5, WidgetSizing::fixed(Vector2::new(120.0, 28.0)));
    let mut active = slider.clone();
    active.common.state.automation_active = true;
    let mut passive_primitives = Vec::new();
    let mut active_primitives = Vec::new();
    slider.append_paint(
        &mut passive_primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    active.append_paint(
        &mut active_primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert_eq!(active_primitives.len(), passive_primitives.len() + 1);
}

#[test]
fn slider_edit_batch_is_copyable_bounded_and_projects_lifecycle_values() {
    fn assert_traits<T: Clone + Copy + Debug + PartialEq>() {}
    assert_traits::<SliderEditBatch>();

    let provenance = pointer_provenance(PointerModifiers::default(), None, None);
    let begin = EditEvent::begin(0.25_f32, provenance);
    assert_eq!(SliderEditBatch::single(begin).len(), 1);
    assert_eq!(SliderEditBatch::single(begin).value_change(), None);
    let no_op_update = begin.update(0.25, provenance).expect("no-op update");
    assert_eq!(
        SliderEditBatch::from_events(&[no_op_update])
            .expect("single no-op update")
            .value_change(),
        None
    );

    let update = begin.update(0.5, provenance).expect("pointer update");
    let commit = update.commit(0.5, provenance).expect("pointer commit");
    let batch = SliderEditBatch::from_events(&[begin, update, commit]).expect("three events");
    assert_eq!(batch.len(), SliderEditBatch::MAX_EVENTS);
    assert_eq!(batch.events(), &[begin, update, commit]);
    assert_eq!(batch.transaction(), begin.transaction);
    assert_eq!(batch.value_change(), Some(0.5));

    let no_op_cancel = begin.cancel(provenance).expect("cancel");
    assert_eq!(
        SliderEditBatch::from_events(&[no_op_cancel])
            .unwrap()
            .value_change(),
        None
    );
    assert_eq!(
        SliderEditBatch::rollback(no_op_cancel).value_change(),
        Some(0.25)
    );
    assert_eq!(
        SliderEditBatch::from_events(&[begin, no_op_cancel])
            .expect("begin and cancel")
            .value_change(),
        None
    );

    let ordinary_cancel = SliderEditBatch::from_events(&[no_op_cancel]).expect("ordinary cancel");
    let rollback_cancel = SliderEditBatch::rollback(no_op_cancel);
    assert_ne!(ordinary_cancel, rollback_cancel);
    assert_ne!(
        format!("{ordinary_cancel:?}"),
        format!("{rollback_cancel:?}")
    );
    assert_eq!(ordinary_cancel.value_change(), None);
    assert_eq!(rollback_cancel.value_change(), Some(0.25));
}

#[test]
fn slider_pointer_batches_preserve_boundaries_and_exact_sample_provenance() {
    let mut slider = retained_slider(12, 0.25);
    let bounds = bounds();
    let press_modifiers = PointerModifiers {
        command: true,
        ..PointerModifiers::default()
    };
    let press_timestamp = InputTimestamp::capture();
    let Some(press) = slider.handle_edit_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(60.0, 14.0),
            button: PointerButton::Primary,
            modifiers: press_modifiers,
            timestamp: Some(press_timestamp),
        },
    ) else {
        panic!("changed press should emit a batch");
    };
    assert_eq!(phases(press), vec![EditPhase::Begin, EditPhase::Update]);
    assert_eq!(
        press.events()[0].provenance,
        pointer_provenance(press_modifiers, Some(press_timestamp), None)
    );
    assert_eq!(press.events()[1].provenance, press.events()[0].provenance);
    assert_eq!(press.events()[0].transaction, press.events()[1].transaction);

    let move_modifiers = PointerModifiers {
        shift: true,
        alt: true,
        ..PointerModifiers::default()
    };
    let move_timestamp = InputTimestamp::capture();
    let mut sequence_range = InputSequenceRange::singleton(InputSequence::from_runtime_value(4));
    sequence_range.extend_end(InputSequence::from_runtime_value(7));
    let Some(moved) = slider.handle_edit_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(90.0, 14.0),
            modifiers: move_modifiers,
            timestamp: Some(move_timestamp),
            sequence_range: Some(sequence_range),
        },
    ) else {
        panic!("changed move should emit a batch");
    };
    assert_eq!(phases(moved), vec![EditPhase::Update]);
    assert_eq!(
        moved.events()[0].provenance,
        pointer_provenance(move_modifiers, Some(move_timestamp), Some(sequence_range))
    );
    assert_eq!(moved.events()[0].transaction, press.events()[0].transaction);

    assert_eq!(
        slider.handle_edit_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(90.0, 14.0),
                modifiers: move_modifiers,
                timestamp: Some(move_timestamp),
                sequence_range: Some(sequence_range),
            },
        ),
        None
    );

    let release_modifiers = PointerModifiers {
        alt: true,
        ..PointerModifiers::default()
    };
    let release_timestamp = InputTimestamp::capture();
    let Some(released) = slider.handle_edit_input(
        bounds,
        WidgetInput::PointerRelease {
            position: Point::new(240.0, 14.0),
            button: PointerButton::Primary,
            modifiers: release_modifiers,
            timestamp: Some(release_timestamp),
        },
    ) else {
        panic!("changed release should emit a batch");
    };
    assert_eq!(phases(released), vec![EditPhase::Update, EditPhase::Commit]);
    assert!(released.events().iter().all(|event| event.provenance
        == pointer_provenance(release_modifiers, Some(release_timestamp), None)));
    assert_eq!(
        released.events()[0].transaction,
        press.events()[0].transaction
    );
    assert_eq!(slider.slider.state.value, 1.0);
    assert!(!slider.slider.common.state.pressed);
    assert_eq!(
        slider.handle_edit_input(
            bounds,
            WidgetInput::primary_release(Point::new(240.0, 14.0))
        ),
        None
    );
}

#[test]
fn slider_same_value_press_and_release_keep_typed_boundaries_without_concise_changes() {
    let mut slider = retained_slider(13, 0.5);
    let bounds = bounds();
    let Some(begin) =
        slider.handle_edit_input(bounds, WidgetInput::primary_press(Point::new(60.0, 14.0)))
    else {
        panic!("same-value press should preserve Begin");
    };
    assert_eq!(phases(begin), vec![EditPhase::Begin]);
    assert_eq!(
        slider
            .handle_edit_input(bounds, WidgetInput::pointer_move(Point::new(60.0, 14.0)))
            .and_then(|batch| batch.value_change()),
        None
    );
    let Some(commit) =
        slider.handle_edit_input(bounds, WidgetInput::primary_release(Point::new(60.0, 14.0)))
    else {
        panic!("same-value release should preserve Commit");
    };
    assert_eq!(phases(commit), vec![EditPhase::Commit]);
    assert_eq!(
        slider
            .handle_edit_input(bounds, WidgetInput::primary_press(Point::new(60.0, 14.0)))
            .and_then(|batch| batch.value_change()),
        None
    );
}

#[test]
fn slider_pointer_admission_ignores_repeated_secondary_and_noop_cancellation_inputs() {
    let bounds = bounds();
    let mut slider = retained_slider(131, 0.25);
    assert_eq!(
        slider.handle_edit_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(60.0, 14.0),
                button: PointerButton::Secondary,
                modifiers: PointerModifiers::default(),
                timestamp: None,
            },
        ),
        None
    );
    assert_eq!(
        slider.handle_edit_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(60.0, 14.0),
                button: PointerButton::Secondary,
                modifiers: PointerModifiers::default(),
                timestamp: None,
            },
        ),
        None
    );
    assert_eq!(
        slider.handle_edit_input(bounds, WidgetInput::primary_release(Point::new(60.0, 14.0))),
        None
    );

    let _ = slider.handle_edit_input(bounds, WidgetInput::primary_press(Point::new(60.0, 14.0)));
    assert_eq!(
        slider.handle_edit_input(bounds, WidgetInput::primary_press(Point::new(96.0, 14.0))),
        None
    );
    assert_eq!(
        slider.handle_edit_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(96.0, 14.0),
                button: PointerButton::Secondary,
                modifiers: PointerModifiers::default(),
                timestamp: None,
            },
        ),
        None
    );
    assert!(slider.slider.common.state.pressed);
    let _ = slider.handle_edit_input(bounds, WidgetInput::primary_release(Point::new(96.0, 14.0)));

    let mut noop = retained_slider(132, 0.5);
    let _ = noop.handle_edit_input(bounds, WidgetInput::primary_press(Point::new(60.0, 14.0)));
    assert_eq!(
        noop.handle_edit_input(bounds, WidgetInput::FocusChanged(false)),
        None
    );
    assert_eq!(noop.slider.state.value, 0.5);
    assert!(!noop.slider.common.state.pressed);
}

#[test]
fn slider_focus_and_capture_cancellation_restore_start_without_commit() {
    let mut slider = retained_slider(14, 0.25);
    let bounds = bounds();
    let _ = slider.handle_edit_input(bounds, WidgetInput::primary_press(Point::new(60.0, 14.0)));
    let _ = slider.handle_edit_input(bounds, WidgetInput::pointer_move(Point::new(96.0, 14.0)));
    let Some(cancel) = slider.handle_edit_input(bounds, WidgetInput::FocusChanged(false)) else {
        panic!("focus loss should cancel an effective pointer edit");
    };
    assert_eq!(phases(cancel), vec![EditPhase::Cancel]);
    assert_eq!(cancel.value_change(), Some(0.25));
    assert_eq!(
        cancel.events()[0].provenance,
        pointer_provenance(PointerModifiers::default(), None, None)
    );
    assert_eq!(slider.slider.state.value, 0.25);
    assert!(!slider.slider.common.state.pressed);
    assert_eq!(
        slider.handle_edit_input(bounds, WidgetInput::primary_release(Point::new(96.0, 14.0))),
        None
    );

    let _ = slider.handle_edit_input(bounds, WidgetInput::primary_press(Point::new(96.0, 14.0)));
    let _ = slider.handle_edit_input(bounds, WidgetInput::pointer_move(Point::new(110.0, 14.0)));
    let Some(output) = Widget::handle_pointer_capture_cancelled(&mut slider, bounds) else {
        panic!("Slider capture cancellation should opt into typed cancellation");
    };
    let cancel = output
        .typed_copied::<SliderEditBatch>()
        .expect("typed cancel batch");
    assert_eq!(phases(cancel), vec![EditPhase::Cancel]);
    assert_eq!(slider.slider.state.value, 0.25);
    assert!(!slider.slider.common.state.pressed);
    assert_eq!(
        slider.handle_edit_input(
            bounds,
            WidgetInput::primary_release(Point::new(110.0, 14.0))
        ),
        None
    );
}

#[test]
fn slider_keyboard_batches_are_atomic_timestamped_and_noop_keys_emit_nothing() {
    let mut slider = retained_slider(15, 0.5);
    let bounds = bounds();
    let _ = slider.handle_edit_input(bounds, WidgetInput::FocusChanged(true));
    let timestamp = InputTimestamp::capture();
    let Some(batch) = slider.handle_edit_input(
        bounds,
        WidgetInput::KeyPress {
            key: WidgetKey::ArrowRight,
            modifiers: KeyboardModifiers::default(),
            repeat: false,
            timestamp: Some(timestamp),
        },
    ) else {
        panic!("changed keyboard edit should emit a batch");
    };
    assert_eq!(
        phases(batch),
        vec![EditPhase::Begin, EditPhase::Update, EditPhase::Commit]
    );
    assert!(batch.events().iter().all(|event| event.provenance
        == InteractionProvenance::Keyboard {
            timestamp: Some(timestamp)
        }));
    assert_eq!(batch.value_change(), Some(0.55));
    assert_eq!(slider.slider.state.value, 0.55);

    let mut endpoint = retained_slider(16, 1.0);
    let _ = endpoint.handle_edit_input(bounds, WidgetInput::FocusChanged(true));
    assert_eq!(
        endpoint.handle_edit_input(bounds, WidgetInput::key_press(WidgetKey::End)),
        None
    );
    assert_eq!(endpoint.slider.state.value, 1.0);
}

#[test]
fn slider_reprojection_preserves_pointer_transaction_but_keeps_fresh_value_authoritative() {
    let bounds = bounds();
    let mut previous = retained_slider(17, 0.25);
    let Some(press) =
        previous.handle_edit_input(bounds, WidgetInput::primary_press(Point::new(60.0, 14.0)))
    else {
        panic!("pointer press should begin the retained transaction");
    };
    let _ = previous.handle_edit_input(bounds, WidgetInput::pointer_move(Point::new(72.0, 14.0)));
    let transaction = press.events()[0].transaction;

    let mut current = retained_slider(17, 0.75);
    current.synchronize_from_previous(&previous);
    assert_eq!(current.slider.state.value, 0.75);
    assert!(current.slider.common.state.pressed);
    assert!(current.slider.common.state.focused);

    let mut continuing = retained_slider(17, 0.75);
    continuing.synchronize_from_previous(&previous);
    let Some(update) =
        continuing.handle_edit_input(bounds, WidgetInput::pointer_move(Point::new(96.0, 14.0)))
    else {
        panic!("retained pointer edit should accept a changed move");
    };
    assert_eq!(update.events()[0].phase, EditPhase::Update);
    assert_eq!(update.events()[0].transaction, transaction);
    assert_eq!(
        update.events()[0].value,
        value_for_position(
            bounds,
            Point::new(96.0, 14.0),
            continuing.slider.props.track_height
        )
    );
    assert_eq!(continuing.slider.state.value, update.events()[0].value);
    let Some(commit) =
        continuing.handle_edit_input(bounds, WidgetInput::primary_release(Point::new(96.0, 14.0)))
    else {
        panic!("retained pointer edit should commit");
    };
    assert_eq!(commit.events().len(), 1);
    assert_eq!(commit.events()[0].phase, EditPhase::Commit);
    assert_eq!(commit.events()[0].value, continuing.slider.state.value);

    let Some(cancel) = current.handle_edit_input(bounds, WidgetInput::FocusChanged(false)) else {
        panic!("fresh value differing from the original start should cancel");
    };
    assert_eq!(phases(cancel), vec![EditPhase::Cancel]);
    assert_eq!(cancel.events()[0].start_value, 0.25);
    assert_eq!(current.slider.state.value, 0.25);
}

#[test]
fn slider_disabled_or_read_only_reprojection_drops_retained_pointer_transaction() {
    let bounds = bounds();
    let mut previous = retained_slider(18, 0.25);
    let _ = previous.handle_edit_input(bounds, WidgetInput::primary_press(Point::new(60.0, 14.0)));
    let _ = previous.handle_edit_input(bounds, WidgetInput::pointer_move(Point::new(96.0, 14.0)));

    for read_only in [false, true] {
        let mut current = retained_slider(18, 0.75);
        current.slider.common.state.disabled = !read_only;
        current.slider.common.state.read_only = read_only;
        current.synchronize_from_previous(&previous);
        assert!(!current.slider.common.state.pressed);
        assert_eq!(
            current.handle_edit_input(
                bounds,
                WidgetInput::primary_release(Point::new(110.0, 14.0))
            ),
            None
        );
        assert_eq!(current.slider.state.value, 0.75);
    }
}

#[test]
fn slider_typed_mapper_accepts_batches_and_direct_messages() {
    use crate::runtime::{SurfaceNode, UiSurface, WidgetMessageMapper};

    let typed_surface: UiSurface<SliderEditBatch> =
        UiSurface::new(SurfaceNode::slider_edits_mapped(
            19,
            0.25,
            WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            |batch| batch,
        ));
    let provenance = pointer_provenance(PointerModifiers::default(), None, None);
    let begin = EditEvent::begin(0.25, provenance);
    let update = begin.update(0.5, provenance).expect("update");
    let batch = SliderEditBatch::from_events(&[begin, update]).expect("batch");
    assert_eq!(
        typed_surface.dispatch_widget_output(19, WidgetOutput::typed(batch)),
        Some(batch)
    );

    let concise_surface: UiSurface<f32> = UiSurface::new(SurfaceNode::widget(
        SliderWidget::new(20, 0.25, WidgetSizing::fixed(Vector2::new(120.0, 28.0))),
        WidgetMessageMapper::slider(|message| match message {
            SliderMessage::ValueChanged { value } => value,
        }),
    ));
    assert_eq!(
        concise_surface.dispatch_widget_output(
            20,
            WidgetOutput::typed(SliderMessage::ValueChanged { value: 0.75 }),
        ),
        Some(0.75)
    );
    assert_eq!(
        concise_surface.dispatch_widget_output(20, WidgetOutput::typed(batch)),
        Some(0.5)
    );

    let cancel = update.cancel(provenance).expect("cancel");
    let rollback_batch =
        SliderEditBatch::from_events(&[begin, update, cancel]).expect("rollback batch");
    assert_eq!(rollback_batch.value_change(), Some(0.25));
    assert_eq!(
        concise_surface.dispatch_widget_output(20, WidgetOutput::typed(rollback_batch)),
        Some(0.25)
    );
    assert_eq!(
        concise_surface
            .dispatch_widget_output(20, WidgetOutput::typed(SliderEditBatch::single(begin))),
        None
    );
}
