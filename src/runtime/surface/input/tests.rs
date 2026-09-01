use crate::runtime::{LayerKind, SurfaceLayer, surface::WidgetStateSyncPolicy};
use crate::runtime::{SurfaceChild, SurfaceNode, WidgetMessageMapper};
use crate::{
    gui::{
        input::{InputSequence, InputSequenceRange, InputTimestamp},
        types::{Point, Rect, Vector2},
    },
    runtime::surface::{WidgetDispatchResult, WidgetPath},
    widgets::{
        ButtonWidget, EditPhase, KnobDomainCancellationReason, KnobDomainMessage, KnobMessage,
        KnobPointerMetadata, KnobWidget, NumericAdjustment, NumericStep, NumericStepDirection,
        PointerButton, PointerModifiers, RetainedKnobDomainWidget, ScrollbarAxis, ScrollbarWidget,
        Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetRevision, WidgetSizing,
    },
};
use std::collections::HashMap;
use std::{cell::Cell, rc::Rc};

#[test]
fn scene_without_layers_routes_base_widget_at_transparent_path() {
    let mut root: SurfaceNode<()> = SurfaceNode::scene(
        1,
        SurfaceNode::widget(
            ButtonWidget::new(10, "Base", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
            WidgetMessageMapper::none(),
        ),
        Vec::new(),
    );

    let result = root.dispatch_input_at_path(
        10,
        &[],
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0)),
        WidgetInput::pointer_move(Point::new(8.0, 8.0)),
    );

    assert!(matches!(result, Some(WidgetDispatchResult::NoOutput)));
    assert!(
        root.find_widget_at_path(&[])
            .expect("base widget exists at transparent path")
            .widget()
            .common()
            .state
            .hovered
    );
}

fn mapped_knob(value: f32, disabled: bool) -> SurfaceNode<KnobMessage> {
    let mut knob = KnobWidget::new(30, value);
    knob.common.state.disabled = disabled;
    SurfaceNode::widget(
        knob,
        WidgetMessageMapper::typed(|message: KnobMessage| message),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeDomainError {
    Policy,
}

#[derive(Clone, Copy)]
struct RuntimeDomainAdjustment;

impl NumericAdjustment<f32> for RuntimeDomainAdjustment {
    type Error = RuntimeDomainError;

    fn normalized_to_value(&self, normalized: f32) -> Result<f32, Self::Error> {
        Ok(10.0 + normalized * 100.0)
    }

    fn value_to_normalized(&self, value: &f32) -> Result<f32, Self::Error> {
        Ok((*value - 10.0) / 100.0)
    }

    fn step(
        &self,
        _value: &f32,
        _direction: NumericStepDirection,
        _step: NumericStep,
    ) -> Result<f32, Self::Error> {
        Err(RuntimeDomainError::Policy)
    }

    fn scrub(
        &self,
        _value: &f32,
        _normalized_delta: f32,
        _step: NumericStep,
    ) -> Result<f32, Self::Error> {
        Err(RuntimeDomainError::Policy)
    }

    fn wheel(&self, _value: &f32, _delta: f32, _step: NumericStep) -> Result<f32, Self::Error> {
        Err(RuntimeDomainError::Policy)
    }
}

fn mapped_domain_knob() -> SurfaceNode<KnobDomainMessage<RuntimeDomainError>> {
    let adjustment = RuntimeDomainAdjustment;
    let domain_value = 20.0;
    let default_domain_value = 10.0;
    let normalized_value = crate::widgets::domain_initial_normalized(domain_value, &adjustment)
        .expect("runtime current inverse should succeed");
    let default_normalized_value =
        crate::widgets::domain_initial_normalized(default_domain_value, &adjustment)
            .expect("runtime default inverse should succeed");
    let knob = KnobWidget::new(34, normalized_value)
        .with_default_value(default_normalized_value)
        .with_sensitivity(0.01);
    SurfaceNode::widget(
        RetainedKnobDomainWidget::new(
            knob,
            Rc::new(adjustment),
            domain_value,
            default_domain_value,
            default_normalized_value,
        ),
        WidgetMessageMapper::typed(|message: KnobDomainMessage<RuntimeDomainError>| message),
    )
}

#[test]
fn domain_knob_surface_routes_explicit_pointer_capture_cancellation() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(40.0, 40.0));
    let mut root = mapped_domain_knob();
    assert!(matches!(
        root.dispatch_input_at_path(
            34,
            &[],
            bounds,
            WidgetInput::primary_press(Point::new(20.0, 20.0)),
        ),
        Some(WidgetDispatchResult::Message(
            KnobDomainMessage::GestureStarted { value: 20.0, .. }
        ))
    ));
    assert!(matches!(
        root.dispatch_input_at_path(
            34,
            &[],
            bounds,
            WidgetInput::pointer_move(Point::new(20.0, 10.0))
        ),
        Some(WidgetDispatchResult::Message(
            KnobDomainMessage::ValueChanged { value, .. }
        )) if (value - 30.0).abs() < 0.0001
    ));

    let Some(WidgetDispatchResult::Message(cancel)) =
        root.dispatch_pointer_capture_cancelled_at_path(34, &[], bounds)
    else {
        panic!("domain Knob should route an explicit capture cancellation");
    };
    assert!(matches!(
        cancel,
        KnobDomainMessage::GestureCancelled {
            start_value,
            previous_value,
            reason: KnobDomainCancellationReason::PointerCaptureLoss,
            ..
        } if (start_value - 20.0).abs() < 0.0001 && (previous_value - 30.0).abs() < 0.0001
    ));
    let widget = root
        .find_widget_at_path(&[])
        .expect("domain Knob exists")
        .widget()
        .as_any()
        .downcast_ref::<RetainedKnobDomainWidget<RuntimeDomainAdjustment>>()
        .expect("domain Knob type is retained");
    assert_eq!(widget.domain_value, 20.0);
    assert!(!widget.knob.common.state.pressed);
    assert!(widget.knob.common.state.focused);
}

#[test]
fn slider_capture_cancellation_routes_typed_cancel_while_knob_default_stays_suppressed() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(120.0, 28.0));
    let mut slider = SurfaceNode::slider_edits_mapped(
        31,
        0.25,
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
        |batch| batch,
    );
    assert!(matches!(
        slider.dispatch_input_at_path(
            31,
            &[],
            bounds,
            WidgetInput::primary_press(Point::new(60.0, 14.0)),
        ),
        Some(WidgetDispatchResult::Message(batch))
            if batch.events().iter().map(|event| event.phase).collect::<Vec<_>>()
                == [EditPhase::Begin, EditPhase::Update]
    ));
    let Some(WidgetDispatchResult::Message(cancel)) =
        slider.dispatch_pointer_capture_cancelled_at_path(31, &[], bounds)
    else {
        panic!("Slider should route an opted-in capture cancellation");
    };
    assert_eq!(cancel.events().len(), 1);
    assert_eq!(cancel.events()[0].phase, EditPhase::Cancel);
    assert_eq!(cancel.value_change(), Some(0.25));
    assert!(
        slider
            .find_widget_at_path(&[])
            .expect("Slider exists")
            .widget()
            .common()
            .state
            .focused
    );

    let mut knob = mapped_knob(0.5, false);
    assert!(matches!(
        knob.dispatch_input_at_path(
            30,
            &[],
            Rect::from_min_size(Point::default(), Vector2::new(40.0, 40.0)),
            WidgetInput::primary_press(Point::new(20.0, 20.0)),
        ),
        Some(WidgetDispatchResult::Message(
            KnobMessage::GestureStarted { .. }
        ))
    ));
    assert!(matches!(
        knob.dispatch_pointer_capture_cancelled_at_path(
            30,
            &[],
            Rect::from_min_size(Point::default(), Vector2::new(40.0, 40.0)),
        ),
        Some(WidgetDispatchResult::NoOutput)
    ));
    let knob = knob
        .find_widget_at_path(&[])
        .expect("knob exists")
        .widget()
        .as_any()
        .downcast_ref::<KnobWidget>()
        .expect("knob type is retained");
    assert!(!knob.common.state.pressed);
    assert_eq!(knob.state.gesture_origin, None);
    assert!(knob.common.state.focused);
}

#[test]
fn knob_typed_surface_routes_pointer_transaction_and_capture_cancellation() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(40.0, 40.0));
    let mut root = SurfaceNode::knob_edits_mapped(
        33,
        0.5,
        WidgetSizing::fixed(Vector2::new(40.0, 40.0)),
        |batch| batch,
    );
    let Some(WidgetDispatchResult::Message(press)) = root.dispatch_input_at_path(
        33,
        &[],
        bounds,
        WidgetInput::primary_press(Point::new(20.0, 20.0)),
    ) else {
        panic!("Knob typed press should emit a begin batch");
    };
    assert_eq!(press.events()[0].phase, EditPhase::Begin);
    let transaction = press.transaction();

    let Some(WidgetDispatchResult::Message(update)) = root.dispatch_input_at_path(
        33,
        &[],
        bounds,
        WidgetInput::pointer_move(Point::new(20.0, 10.0)),
    ) else {
        panic!("Knob typed move should emit an update batch");
    };
    assert_eq!(update.events()[0].phase, EditPhase::Update);
    assert_eq!(update.events()[0].transaction, transaction);

    let Some(WidgetDispatchResult::Message(cancel)) =
        root.dispatch_pointer_capture_cancelled_at_path(33, &[], bounds)
    else {
        panic!("Knob capture cancellation should emit a typed cancel batch");
    };
    assert_eq!(cancel.events()[0].phase, EditPhase::Cancel);
    assert_eq!(cancel.value_change(), Some(0.5));
    assert!(
        root.find_widget_at_path(&[])
            .expect("Knob exists")
            .widget()
            .common()
            .state
            .focused
    );
    assert_eq!(
        root.find_widget_at_path(&[])
            .expect("Knob exists")
            .widget()
            .automation_semantics()
            .value_text
            .as_deref(),
        Some("0.500")
    );
}

#[test]
fn slider_typed_reprojection_retains_transaction_but_keeps_fresh_value_authoritative() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(120.0, 28.0));
    let mut previous = SurfaceNode::slider_edits_mapped(
        32,
        0.25,
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
        |batch| batch,
    );
    let Some(WidgetDispatchResult::Message(press)) = previous.dispatch_input_at_path(
        32,
        &[],
        bounds,
        WidgetInput::primary_press(Point::new(60.0, 14.0)),
    ) else {
        panic!("Slider press should emit the typed begin/update batch");
    };
    let transaction = press.events()[0].transaction;
    let _ = previous.dispatch_input_at_path(
        32,
        &[],
        bounds,
        WidgetInput::pointer_move(Point::new(72.0, 14.0)),
    );

    let mut current = SurfaceNode::slider_edits_mapped(
        32,
        0.75,
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
        |batch| batch,
    );
    let paths = HashMap::from([(32, WidgetPath::from_slice(&[]))]);
    current.synchronize_widget_state_from_paths(
        &[32],
        &paths,
        &previous,
        &paths,
        WidgetStateSyncPolicy::default(),
    );
    let current_widget = current
        .find_widget_at_path(&[])
        .expect("current Slider exists")
        .widget();
    assert!(current_widget.common().state.pressed);
    assert_eq!(
        current_widget.automation_semantics().value_text.as_deref(),
        Some("0.750")
    );

    let Some(WidgetDispatchResult::Message(update)) = current.dispatch_input_at_path(
        32,
        &[],
        bounds,
        WidgetInput::pointer_move(Point::new(96.0, 14.0)),
    ) else {
        panic!("retained Slider transaction should accept a typed update");
    };
    assert_eq!(update.events()[0].phase, EditPhase::Update);
    assert_eq!(update.events()[0].transaction, transaction);

    let Some(WidgetDispatchResult::Message(cancel)) =
        current.dispatch_input_at_path(32, &[], bounds, WidgetInput::FocusChanged(false))
    else {
        panic!("retained Slider transaction should cancel on focus loss");
    };
    assert_eq!(cancel.events()[0].phase, EditPhase::Cancel);
    assert_eq!(cancel.events()[0].start_value, 0.25);
    assert_eq!(cancel.value_change(), Some(0.25));
    assert_eq!(
        current
            .find_widget_at_path(&[])
            .expect("cancelled Slider exists")
            .widget()
            .automation_semantics()
            .value_text
            .as_deref(),
        Some("0.250")
    );
}

#[test]
fn mapped_knob_routes_double_click_reset_metadata_from_second_sample() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(40.0, 40.0));
    let mut root = SurfaceNode::widget(
        KnobWidget::new(30, 0.8).with_default_value(0.25),
        WidgetMessageMapper::typed(|message: KnobMessage| message),
    );
    let modifiers = PointerModifiers {
        command: true,
        shift: true,
        ..PointerModifiers::default()
    };
    let timestamp = InputTimestamp::capture();
    let metadata = KnobPointerMetadata {
        modifiers,
        timestamp: Some(timestamp),
        sequence_range: None,
    };

    let Some(WidgetDispatchResult::Message(message)) = root.dispatch_input_at_path(
        30,
        &[],
        bounds,
        WidgetInput::pointer_double_click_with_timestamp(
            Point::new(20.0, 20.0),
            PointerButton::Primary,
            modifiers,
            Some(timestamp),
        ),
    ) else {
        panic!("mapped accepted double-click reset should emit one message");
    };
    assert_eq!(
        message,
        KnobMessage::Reset {
            value: 0.25,
            metadata,
        }
    );
    assert_eq!(message.pointer_gesture_metadata(), Some(metadata));
    assert!(matches!(
        root.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::primary_release(Point::new(20.0, 20.0)),
        ),
        Some(WidgetDispatchResult::NoOutput)
    ));
}

#[test]
fn mapped_knob_reprojection_preserves_pointer_gesture_and_authoritative_value() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(40.0, 40.0));
    let paths = HashMap::from([(30, WidgetPath::from_slice(&[]))]);
    let mut previous = mapped_knob(0.5, false);
    let mut current = mapped_knob(0.5, false);
    let press_metadata = KnobPointerMetadata {
        modifiers: PointerModifiers {
            command: true,
            ..PointerModifiers::default()
        },
        timestamp: Some(InputTimestamp::capture()),
        sequence_range: None,
    };
    let move_metadata = KnobPointerMetadata {
        modifiers: PointerModifiers {
            shift: true,
            alt: true,
            ..PointerModifiers::default()
        },
        timestamp: Some(InputTimestamp::capture()),
        sequence_range: {
            let mut range = InputSequenceRange::singleton(InputSequence::from_runtime_value(61));
            range.extend_end(InputSequence::from_runtime_value(64));
            Some(range)
        },
    };
    let release_metadata = KnobPointerMetadata {
        modifiers: PointerModifiers {
            alt: true,
            ..PointerModifiers::default()
        },
        timestamp: Some(InputTimestamp::capture()),
        sequence_range: None,
    };

    assert!(matches!(
        previous.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::pointer_press_with_timestamp(
                Point::new(20.0, 20.0),
                PointerButton::Primary,
                press_metadata.modifiers,
                press_metadata.timestamp,
            ),
        ),
        Some(WidgetDispatchResult::Message(KnobMessage::GestureStarted {
            value: 0.5,
            metadata,
        }))
            if metadata == press_metadata
    ));
    current.synchronize_widget_state_from_paths(
        &[30],
        &paths,
        &previous,
        &paths,
        WidgetStateSyncPolicy::default(),
    );
    assert!(matches!(
        current.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::pointer_move_with_metadata(
                Point::new(20.0, 10.0),
                move_metadata.modifiers,
                move_metadata.timestamp,
                move_metadata.sequence_range,
            ),
        ),
        Some(WidgetDispatchResult::Message(KnobMessage::ValueChanged {
            value,
            metadata,
        })) if value > 0.5 && metadata == move_metadata
    ));

    // The reducer's fresh projection owns the value while the active pointer
    // gesture remains runtime-owned across this refresh.
    previous = current;
    current = mapped_knob(0.62, false);
    current.synchronize_widget_state_from_paths(
        &[30],
        &paths,
        &previous,
        &paths,
        WidgetStateSyncPolicy::default(),
    );
    let final_value = current
        .find_widget_at_path(&[])
        .expect("knob exists")
        .widget()
        .as_any()
        .downcast_ref::<KnobWidget>()
        .expect("knob type is retained")
        .state
        .value;
    assert_eq!(final_value, 0.62);
    assert!(matches!(
        current.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::pointer_release_with_timestamp(
                Point::new(20.0, 10.0),
                PointerButton::Primary,
                release_metadata.modifiers,
                release_metadata.timestamp,
            )
        ),
        Some(WidgetDispatchResult::Message(KnobMessage::GestureEnded {
            value: 0.62,
            metadata,
        }))
            if metadata == release_metadata
    ));
}

#[test]
fn mapped_knob_routes_hover_wheel_to_a_distinct_automation_gesture() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(40.0, 40.0));
    let mut root = mapped_knob(0.5, false);

    assert!(matches!(
        root.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::plain_wheel(Point::new(20.0, 20.0), Vector2::new(0.0, 120.0)),
        ),
        Some(WidgetDispatchResult::Message(KnobMessage::WheelGesture(batch)))
            if batch.events
                == [
                    crate::widgets::KnobAutomationEvent::GestureStarted { value: 0.5 },
                    crate::widgets::KnobAutomationEvent::ValueChanged { value: 0.55 },
                    crate::widgets::KnobAutomationEvent::GestureEnded { value: 0.55 },
                ]
    ));
}

#[test]
fn mapped_knob_routes_keyboard_timestamp_for_an_accepted_edit() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(40.0, 40.0));
    let mut root = mapped_knob(0.5, false);
    assert!(matches!(
        root.dispatch_input_at_path(30, &[], bounds, WidgetInput::FocusChanged(true)),
        Some(WidgetDispatchResult::NoOutput)
    ));

    let timestamp = InputTimestamp::capture();
    let Some(WidgetDispatchResult::Message(KnobMessage::KeyboardGesture(batch))) = root
        .dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::key_press_with_timestamp(
                crate::widgets::WidgetKey::ArrowRight,
                Some(timestamp),
            ),
        )
    else {
        panic!("mapped focused keyboard edit should emit a lifecycle batch");
    };
    assert_eq!(
        batch.events[0],
        crate::widgets::KnobAutomationEvent::GestureStarted { value: 0.5 }
    );
    assert!(matches!(
        batch.events[1],
        crate::widgets::KnobAutomationEvent::ValueChanged { value } if (value - 0.596).abs() < 0.00001
    ));
    assert!(matches!(
        batch.events[2],
        crate::widgets::KnobAutomationEvent::GestureEnded { value } if (value - 0.596).abs() < 0.00001
    ));
    assert_eq!(batch.input_metadata().timestamp, Some(timestamp));
}

#[test]
fn disabled_knob_reprojection_clears_pointer_gesture_state() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(40.0, 40.0));
    let paths = HashMap::from([(30, WidgetPath::from_slice(&[]))]);
    let mut previous = mapped_knob(0.5, false);
    assert!(matches!(
        previous.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::primary_press(Point::new(20.0, 20.0)),
        ),
        Some(WidgetDispatchResult::Message(
            KnobMessage::GestureStarted { .. }
        ))
    ));
    let mut current = mapped_knob(0.7, true);
    current.synchronize_widget_state_from_paths(
        &[30],
        &paths,
        &previous,
        &paths,
        WidgetStateSyncPolicy::default(),
    );
    assert!(matches!(
        current.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::pointer_move(Point::new(20.0, 10.0))
        ),
        Some(WidgetDispatchResult::NoOutput)
    ));
    assert!(matches!(
        current.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::primary_double_click(Point::new(20.0, 20.0))
        ),
        Some(WidgetDispatchResult::NoOutput)
    ));
    assert!(matches!(
        current.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::key_press(crate::widgets::WidgetKey::ArrowRight)
        ),
        Some(WidgetDispatchResult::NoOutput)
    ));
    assert!(matches!(
        current.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::pointer_release_with_timestamp(
                Point::new(20.0, 10.0),
                PointerButton::Primary,
                PointerModifiers {
                    command: true,
                    ..PointerModifiers::default()
                },
                Some(InputTimestamp::capture()),
            )
        ),
        Some(WidgetDispatchResult::Message(KnobMessage::GestureEnded {
            value: 0.7,
            metadata,
        }))
            if metadata.modifiers.command
                && metadata.timestamp.is_some()
                && metadata.sequence_range.is_none()
    ));
    assert!(matches!(
        current.dispatch_input_at_path(
            30,
            &[],
            bounds,
            WidgetInput::primary_release(Point::new(20.0, 10.0))
        ),
        Some(WidgetDispatchResult::NoOutput)
    ));
    assert!(matches!(
        current.dispatch_input_at_path(30, &[], bounds, WidgetInput::FocusChanged(false)),
        Some(WidgetDispatchResult::NoOutput)
    ));
    let knob = current
        .find_widget_at_path(&[])
        .expect("disabled knob exists")
        .widget()
        .as_any()
        .downcast_ref::<KnobWidget>()
        .expect("knob type is retained");
    assert!(!knob.common.state.pressed);
    assert_eq!(knob.state.gesture_origin, None);
    assert_eq!(knob.state.value, 0.7);
}

#[test]
fn dispatch_input_at_child_path_routes_without_tree_search() {
    let mut root: SurfaceNode<()> = SurfaceNode::column(
        1,
        0.0,
        vec![
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(10, "First", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )),
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(20, "Second", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )),
        ],
    );

    let result = root.dispatch_input_at_path(
        20,
        &[1],
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0)),
        WidgetInput::pointer_move(Point::new(8.0, 8.0)),
    );

    assert!(matches!(result, Some(WidgetDispatchResult::NoOutput)));
    assert!(
        root.find_widget(20)
            .expect("target widget exists")
            .widget()
            .common()
            .state
            .hovered
    );
    assert!(
        !root
            .find_widget(10)
            .expect("sibling widget exists")
            .widget()
            .common()
            .state
            .hovered
    );
}

#[test]
fn find_widget_at_child_path_returns_only_the_target_leaf() {
    let root: SurfaceNode<()> = SurfaceNode::column(
        1,
        0.0,
        vec![
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(10, "First", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )),
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(20, "Second", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )),
        ],
    );

    assert_eq!(
        root.find_widget_at_path(&[1])
            .expect("target widget exists")
            .id(),
        20
    );
    assert!(root.find_widget_at_path(&[2]).is_none());
}

#[test]
fn synchronize_widget_state_from_paths_preserves_state_after_reorder() {
    let mut previous: SurfaceNode<()> = SurfaceNode::column(
        1,
        0.0,
        vec![
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(10, "First", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )),
            SurfaceChild::fill(SurfaceNode::widget(
                ScrollbarWidget::new(
                    20,
                    ScrollbarAxis::Vertical,
                    WidgetSizing::fixed(Vector2::new(16.0, 100.0)),
                ),
                WidgetMessageMapper::none(),
            )),
        ],
    );
    let mut current: SurfaceNode<()> = SurfaceNode::column(
        1,
        0.0,
        vec![
            SurfaceChild::fill(SurfaceNode::widget(
                ScrollbarWidget::new(
                    20,
                    ScrollbarAxis::Vertical,
                    WidgetSizing::fixed(Vector2::new(16.0, 100.0)),
                ),
                WidgetMessageMapper::none(),
            )),
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(10, "First", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )),
        ],
    );

    let _ = previous.dispatch_input_at_path(
        20,
        &[1],
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(16.0, 100.0)),
        WidgetInput::PointerPress {
            position: Point::new(8.0, 8.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
            timestamp: None,
        },
    );

    let previous_paths = HashMap::from([
        (10, WidgetPath::from_slice(&[0])),
        (20, WidgetPath::from_slice(&[1])),
    ]);
    let current_paths = HashMap::from([
        (20, WidgetPath::from_slice(&[0])),
        (10, WidgetPath::from_slice(&[1])),
    ]);
    current.synchronize_widget_state_from_paths(
        &[20],
        &current_paths,
        &previous,
        &previous_paths,
        WidgetStateSyncPolicy::default(),
    );

    let moved = current
        .find_widget_at_path(&[0])
        .expect("moved widget exists")
        .widget()
        .as_any()
        .downcast_ref::<ScrollbarWidget>()
        .expect("moved widget stays a scrollbar");
    assert_eq!(moved.state.drag_grip_fraction, Some(0.08));
}

#[test]
fn synchronize_widget_state_from_paths_skips_incompatible_replacement() {
    let mut previous: SurfaceNode<()> = SurfaceNode::widget(
        ButtonWidget::new(
            20,
            "Previous",
            WidgetSizing::fixed(Vector2::new(80.0, 28.0)),
        ),
        WidgetMessageMapper::none(),
    );
    let mut current: SurfaceNode<()> = SurfaceNode::widget(
        ScrollbarWidget::new(
            20,
            ScrollbarAxis::Vertical,
            WidgetSizing::fixed(Vector2::new(16.0, 100.0)),
        ),
        WidgetMessageMapper::none(),
    );
    let _ = previous.dispatch_input_at_path(
        20,
        &[],
        Rect::from_min_size(Point::default(), Vector2::new(80.0, 28.0)),
        WidgetInput::PointerPress {
            position: Point::new(8.0, 8.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
            timestamp: None,
        },
    );
    let paths = HashMap::from([(20, WidgetPath::from_slice(&[]))]);

    current.synchronize_widget_state_from_paths(
        &[20],
        &paths,
        &previous,
        &paths,
        WidgetStateSyncPolicy::default(),
    );

    assert!(
        !current
            .find_widget_at_path(&[])
            .expect("replacement exists")
            .widget()
            .common()
            .state
            .pressed
    );
}

#[derive(Clone)]
struct SyncProbeWidget {
    common: WidgetCommon,
    synchronize_calls: Rc<Cell<u32>>,
}

impl SyncProbeWidget {
    fn new(id: u64, synchronize_calls: Rc<Cell<u32>>) -> Self {
        Self {
            common: WidgetCommon::fixed(id, 40.0, 20.0),
            synchronize_calls,
        }
    }
}

impl Widget for SyncProbeWidget {
    fn revision(&self) -> WidgetRevision {
        WidgetRevision::exact((), (), (), ())
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn synchronize_from_previous(&mut self, _previous: &dyn Widget) {
        self.synchronize_calls
            .set(self.synchronize_calls.get().saturating_add(1));
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<crate::runtime::PaintPrimitive>,
        _bounds: Rect,
        _layout: &crate::layout::LayoutOutput,
        _theme: &crate::theme::ThemeTokens,
    ) {
    }
}

#[test]
fn synchronize_skips_widget_with_invalidated_cached_compatibility() {
    let synchronize_calls = Rc::new(Cell::new(0));
    let previous: SurfaceNode<()> = SurfaceNode::widget(
        SyncProbeWidget::new(20, Rc::clone(&synchronize_calls)),
        WidgetMessageMapper::none(),
    );
    let mut current: SurfaceNode<()> = SurfaceNode::widget(
        SyncProbeWidget::new(20, Rc::clone(&synchronize_calls)),
        WidgetMessageMapper::none(),
    );
    let Some(widget) = current.find_widget_mut(20) else {
        panic!("current widget exists");
    };
    widget.widget_mut().common_mut().state.hovered = true;

    let paths = HashMap::from([(20, WidgetPath::from_slice(&[]))]);
    current.synchronize_widget_state_from_paths(
        &[20],
        &paths,
        &previous,
        &paths,
        WidgetStateSyncPolicy::default(),
    );
    assert_eq!(synchronize_calls.get(), 0);
}

#[test]
fn scene_widget_state_sync_finds_widgets_inside_layers() {
    let mut previous: SurfaceNode<()> = SurfaceNode::scene(
        1,
        SurfaceNode::widget(
            ButtonWidget::new(10, "Base", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
            WidgetMessageMapper::none(),
        ),
        vec![SurfaceLayer::new(
            LayerKind::Modal,
            SurfaceNode::widget(
                ScrollbarWidget::new(
                    20,
                    ScrollbarAxis::Vertical,
                    WidgetSizing::fixed(Vector2::new(16.0, 100.0)),
                ),
                WidgetMessageMapper::none(),
            ),
        )],
    );
    let mut current: SurfaceNode<()> = SurfaceNode::scene(
        1,
        SurfaceNode::widget(
            ButtonWidget::new(10, "Base", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
            WidgetMessageMapper::none(),
        ),
        vec![SurfaceLayer::new(
            LayerKind::Modal,
            SurfaceNode::widget(
                ScrollbarWidget::new(
                    20,
                    ScrollbarAxis::Vertical,
                    WidgetSizing::fixed(Vector2::new(16.0, 100.0)),
                ),
                WidgetMessageMapper::none(),
            ),
        )],
    );

    let _ = previous.dispatch_input_at_path(
        20,
        &[1],
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(16.0, 100.0)),
        WidgetInput::PointerPress {
            position: Point::new(8.0, 8.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
            timestamp: None,
        },
    );

    let previous_paths = HashMap::from([(20, WidgetPath::from_slice(&[1]))]);
    let current_paths = HashMap::from([(20, WidgetPath::from_slice(&[1]))]);
    current.synchronize_widget_state_from_paths(
        &[20],
        &current_paths,
        &previous,
        &previous_paths,
        WidgetStateSyncPolicy::default(),
    );

    let synced = current
        .find_widget_at_path(&[1])
        .expect("layer widget exists")
        .widget()
        .as_any()
        .downcast_ref::<ScrollbarWidget>()
        .expect("layer widget stays a scrollbar");
    assert_eq!(synced.state.drag_grip_fraction, Some(0.08));
}

#[test]
fn exclusive_pointer_capture_sync_clears_non_captured_hover_state() {
    let mut previous: SurfaceNode<()> = SurfaceNode::column(
        1,
        0.0,
        vec![
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(10, "Hover", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )),
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(
                    20,
                    "Captured",
                    WidgetSizing::fixed(Vector2::new(80.0, 28.0)),
                ),
                WidgetMessageMapper::none(),
            )),
        ],
    );
    let mut current = previous.clone();

    let _ = previous.dispatch_input_at_path(
        10,
        &[0],
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0)),
        WidgetInput::pointer_move(Point::new(8.0, 8.0)),
    );
    let _ = previous.dispatch_input_at_path(
        20,
        &[1],
        Rect::from_min_size(Point::new(0.0, 28.0), Vector2::new(80.0, 28.0)),
        WidgetInput::PointerPress {
            position: Point::new(8.0, 36.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
            timestamp: None,
        },
    );

    let previous_paths = HashMap::from([
        (10, WidgetPath::from_slice(&[0])),
        (20, WidgetPath::from_slice(&[1])),
    ]);
    let current_paths = previous_paths.clone();
    current.synchronize_widget_state_from_paths(
        &[10, 20],
        &current_paths,
        &previous,
        &previous_paths,
        WidgetStateSyncPolicy::exclusive_pointer_capture(20),
    );

    assert!(
        !current
            .find_widget_at_path(&[0])
            .expect("hover widget exists")
            .widget()
            .common()
            .state
            .hovered
    );
    assert!(
        current
            .find_widget_at_path(&[1])
            .expect("captured widget exists")
            .widget()
            .common()
            .state
            .pressed
    );
}

#[test]
fn retained_state_sync_keeps_only_current_hover_owner() {
    let mut previous: SurfaceNode<()> = SurfaceNode::column(
        1,
        0.0,
        vec![
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(
                    10,
                    "Previous",
                    WidgetSizing::fixed(Vector2::new(80.0, 28.0)),
                ),
                WidgetMessageMapper::none(),
            )),
            SurfaceChild::fill(SurfaceNode::widget(
                ButtonWidget::new(20, "Current", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )),
        ],
    );
    let mut current = previous.clone();

    let _ = previous.dispatch_input_at_path(
        10,
        &[0],
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0)),
        WidgetInput::pointer_move(Point::new(8.0, 8.0)),
    );
    let _ = previous.dispatch_input_at_path(
        20,
        &[1],
        Rect::from_min_size(Point::new(0.0, 28.0), Vector2::new(80.0, 28.0)),
        WidgetInput::pointer_move(Point::new(8.0, 36.0)),
    );

    let previous_paths = HashMap::from([
        (10, WidgetPath::from_slice(&[0])),
        (20, WidgetPath::from_slice(&[1])),
    ]);
    let current_paths = previous_paths.clone();
    current.synchronize_widget_state_from_paths(
        &[10, 20],
        &current_paths,
        &previous,
        &previous_paths,
        WidgetStateSyncPolicy::retained_hover_owner(Some(20)),
    );

    assert!(
        !current
            .find_widget_at_path(&[0])
            .expect("previous hover widget exists")
            .widget()
            .common()
            .state
            .hovered
    );
    assert!(
        current
            .find_widget_at_path(&[1])
            .expect("current hover widget exists")
            .widget()
            .common()
            .state
            .hovered
    );
}

#[test]
fn retained_state_sync_clears_all_hover_when_pointer_has_no_owner() {
    let mut previous: SurfaceNode<()> = SurfaceNode::column(
        1,
        0.0,
        vec![SurfaceChild::fill(SurfaceNode::widget(
            ButtonWidget::new(
                10,
                "Previous",
                WidgetSizing::fixed(Vector2::new(80.0, 28.0)),
            ),
            WidgetMessageMapper::none(),
        ))],
    );
    let mut current = previous.clone();

    let _ = previous.dispatch_input_at_path(
        10,
        &[0],
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0)),
        WidgetInput::pointer_move(Point::new(8.0, 8.0)),
    );

    let paths = HashMap::from([(10, WidgetPath::from_slice(&[0]))]);
    current.synchronize_widget_state_from_paths(
        &[10],
        &paths,
        &previous,
        &paths,
        WidgetStateSyncPolicy::retained_hover_owner(None),
    );

    assert!(
        !current
            .find_widget_at_path(&[0])
            .expect("previous hover widget exists")
            .widget()
            .common()
            .state
            .hovered
    );
}
