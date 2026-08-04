use super::{KnobWidget, RetainedKnobWidget};
use crate::{
    gui::{
        input::{InputSequence, InputSequenceRange, InputTimestamp},
        types::{Point, Rect, Vector2},
    },
    runtime::{SurfaceNode, UiSurface},
    widgets::{
        EditEvent, EditPhase, InteractionProvenance, KnobEditBatch, KnobMessage,
        KnobPointerMetadata, PointerButton, PointerModifiers, Widget, WidgetInput, WidgetKey,
        WidgetOutput, WidgetSizing,
    },
};
use std::fmt::Debug;

fn bounds() -> Rect {
    Rect::from_min_size(Point::default(), Vector2::new(40.0, 40.0))
}

fn retained_knob(id: u64, value: f32) -> RetainedKnobWidget {
    RetainedKnobWidget::new(KnobWidget::new(id, value).with_sensitivity(0.01))
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

fn phases(batch: KnobEditBatch) -> Vec<EditPhase> {
    batch.events().iter().map(|event| event.phase).collect()
}

#[test]
fn knob_edit_batch_is_bounded_copyable_and_projects_rollbacks() {
    fn assert_traits<T: Clone + Copy + Debug + PartialEq>() {}
    assert_traits::<KnobEditBatch>();
    assert_eq!(KnobEditBatch::MAX_EVENTS, 4);

    let provenance = pointer_provenance(PointerModifiers::default(), None, None);
    let begin = EditEvent::begin(0.25_f32, provenance);
    let update = begin.update(0.5, provenance).expect("update");
    let commit = update.commit(0.5, provenance).expect("commit");
    let batch = KnobEditBatch::from_events(&[begin, update, commit]).expect("batch");
    assert_eq!(batch.events(), &[begin, update, commit]);
    assert_eq!(batch.transaction(), begin.transaction);
    assert_eq!(batch.value_change(), Some(0.5));
    assert!(format!("{batch:?}").contains("KnobEditBatch"));

    let cancel = update.cancel(provenance).expect("cancel");
    let rollback = KnobEditBatch::from_events(&[begin, update, cancel]).expect("rollback");
    assert_eq!(rollback.value_change(), Some(0.25));
    let explicit_rollback = KnobEditBatch::rollback(cancel);
    let reproduced_rollback = KnobEditBatch::rollback(explicit_rollback.events()[0]);
    assert_eq!(explicit_rollback, reproduced_rollback);
    assert_eq!(explicit_rollback.value_change(), Some(0.25));
    assert_ne!(
        explicit_rollback,
        KnobEditBatch::from_events(&[cancel]).expect("ordinary cancel")
    );
    assert!(KnobEditBatch::from_events(&[]).is_none());
    let other_begin = EditEvent::begin(0.25_f32, provenance);
    assert!(KnobEditBatch::from_events(&[begin, other_begin]).is_none());
}

#[test]
fn retained_knob_pointer_lifecycle_preserves_relative_motion_metadata_and_capture_exclusion() {
    let bounds = bounds();
    let mut knob = retained_knob(1, 0.5);
    let press_modifiers = PointerModifiers {
        command: true,
        ..PointerModifiers::default()
    };
    let press_timestamp = InputTimestamp::capture();
    let Some(press) = knob.handle_edit_input(
        bounds,
        WidgetInput::pointer_press_with_timestamp(
            Point::new(20.0, 20.0),
            PointerButton::Primary,
            press_modifiers,
            Some(press_timestamp),
        ),
    ) else {
        panic!("pointer press should begin a typed transaction");
    };
    assert_eq!(phases(press), vec![EditPhase::Begin]);
    let transaction = press.transaction();
    assert_eq!(
        press.events()[0].provenance,
        pointer_provenance(press_modifiers, Some(press_timestamp), None)
    );

    let move_modifiers = PointerModifiers {
        shift: true,
        alt: true,
        ..PointerModifiers::default()
    };
    let move_timestamp = InputTimestamp::capture();
    let mut sequence_range = InputSequenceRange::singleton(InputSequence::from_runtime_value(4));
    sequence_range.extend_end(InputSequence::from_runtime_value(7));
    let Some(move_one) = knob.handle_edit_input(
        bounds,
        WidgetInput::pointer_move_with_metadata(
            Point::new(20.0, 10.0),
            move_modifiers,
            Some(move_timestamp),
            Some(sequence_range),
        ),
    ) else {
        panic!("pointer move should update the typed transaction");
    };
    assert_eq!(phases(move_one), vec![EditPhase::Update]);
    assert_eq!(move_one.events()[0].transaction, transaction);
    assert_eq!(move_one.events()[0].value, 0.6);
    assert_eq!(
        move_one.events()[0].provenance,
        pointer_provenance(move_modifiers, Some(move_timestamp), Some(sequence_range))
    );

    assert_eq!(
        knob.handle_edit_input(
            bounds,
            WidgetInput::wheel_with_metadata(
                Point::new(20.0, 10.0),
                Vector2::new(0.0, 120.0),
                PointerModifiers::default(),
                None,
                None,
            ),
        ),
        None
    );
    assert_eq!(knob.knob.state.value, 0.6);

    let release_modifiers = PointerModifiers {
        alt: true,
        ..PointerModifiers::default()
    };
    let release_timestamp = InputTimestamp::capture();
    let Some(commit) = knob.handle_edit_input(
        bounds,
        WidgetInput::pointer_release_with_timestamp(
            Point::new(20.0, 10.0),
            PointerButton::Primary,
            release_modifiers,
            Some(release_timestamp),
        ),
    ) else {
        panic!("pointer release should commit the typed transaction");
    };
    assert_eq!(phases(commit), vec![EditPhase::Commit]);
    assert_eq!(commit.events()[0].transaction, transaction);
    assert_eq!(
        commit.events()[0].provenance,
        pointer_provenance(release_modifiers, Some(release_timestamp), None)
    );
    assert_eq!(knob.knob.state.value, 0.6);
    assert!(!knob.knob.common.state.pressed);
}

#[test]
fn retained_knob_cancellation_rolls_back_on_focus_and_capture_loss() {
    let bounds = bounds();
    let mut knob = retained_knob(2, 0.5);
    let _ = knob.handle_edit_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0)));
    let _ = knob.handle_edit_input(bounds, WidgetInput::pointer_move(Point::new(20.0, 0.0)));
    let Some(cancel) = knob.handle_edit_input(bounds, WidgetInput::FocusChanged(false)) else {
        panic!("focus loss should emit a typed cancellation");
    };
    assert_eq!(phases(cancel), vec![EditPhase::Cancel]);
    assert_eq!(cancel.value_change(), Some(0.5));
    assert_eq!(cancel.events()[0].start_value, 0.5);
    assert_eq!(
        cancel.events()[0].provenance,
        pointer_provenance(PointerModifiers::default(), None, None)
    );
    assert_eq!(knob.knob.state.value, 0.5);
    assert_eq!(knob.knob.state.gesture_origin, None);

    let _ = knob.handle_edit_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0)));
    let _ = knob.handle_edit_input(bounds, WidgetInput::pointer_move(Point::new(20.0, 10.0)));
    let Some(output) = Widget::handle_pointer_capture_cancelled(&mut knob, bounds) else {
        panic!("capture loss should use the typed cancellation hook");
    };
    let cancel = output
        .typed_copied::<KnobEditBatch>()
        .expect("typed cancellation batch");
    assert_eq!(phases(cancel), vec![EditPhase::Cancel]);
    assert_eq!(cancel.value_change(), Some(0.5));
    assert_eq!(knob.knob.state.value, 0.5);
    assert!(!knob.knob.common.state.pressed);
    assert_eq!(
        knob.handle_edit_input(bounds, WidgetInput::primary_release(Point::new(20.0, 10.0))),
        None
    );

    let mut no_op_focus = retained_knob(9, 0.5);
    let _ =
        no_op_focus.handle_edit_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0)));
    let Some(no_op_cancel) =
        no_op_focus.handle_edit_input(bounds, WidgetInput::FocusChanged(false))
    else {
        panic!("focus loss should emit a typed cancel for a no-op gesture");
    };
    assert_eq!(phases(no_op_cancel), vec![EditPhase::Cancel]);
    assert_eq!(no_op_cancel.value_change(), None);

    let mut no_op_capture = retained_knob(10, 0.5);
    let _ =
        no_op_capture.handle_edit_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0)));
    let Some(output) = Widget::handle_pointer_capture_cancelled(&mut no_op_capture, bounds) else {
        panic!("capture loss should emit a typed cancel for a no-op gesture");
    };
    let no_op_cancel = output
        .typed_copied::<KnobEditBatch>()
        .expect("typed no-op capture cancellation");
    assert_eq!(phases(no_op_cancel), vec![EditPhase::Cancel]);
    assert_eq!(no_op_cancel.value_change(), None);
}

#[test]
fn retained_knob_keyboard_wheel_and_reset_are_atomic_and_preserve_metadata() {
    let bounds = bounds();
    let mut keyboard = retained_knob(3, 0.5);
    let _ = keyboard.handle_edit_input(bounds, WidgetInput::FocusChanged(true));
    let keyboard_timestamp = InputTimestamp::capture();
    let Some(keyboard_batch) = keyboard.handle_edit_input(
        bounds,
        WidgetInput::KeyPress {
            key: WidgetKey::ArrowRight,
            timestamp: Some(keyboard_timestamp),
        },
    ) else {
        panic!("focused keyboard input should emit an atomic batch");
    };
    assert_eq!(
        phases(keyboard_batch),
        vec![EditPhase::Begin, EditPhase::Update, EditPhase::Commit]
    );
    assert!(keyboard_batch.events().iter().all(|event| event.provenance
        == InteractionProvenance::Keyboard {
            timestamp: Some(keyboard_timestamp)
        }));

    let mut wheel = retained_knob(4, 0.5);
    let wheel_modifiers = PointerModifiers {
        command: true,
        ..PointerModifiers::default()
    };
    let wheel_timestamp = InputTimestamp::capture();
    let mut wheel_range = InputSequenceRange::singleton(InputSequence::from_runtime_value(8));
    wheel_range.extend_end(InputSequence::from_runtime_value(9));
    let Some(wheel_batch) = wheel.handle_edit_input(
        bounds,
        WidgetInput::wheel_with_metadata(
            Point::new(20.0, 20.0),
            Vector2::new(0.0, 120.0),
            wheel_modifiers,
            Some(wheel_timestamp),
            Some(wheel_range),
        ),
    ) else {
        panic!("wheel input should emit an atomic batch");
    };
    assert_eq!(
        phases(wheel_batch),
        vec![EditPhase::Begin, EditPhase::Update, EditPhase::Commit]
    );
    assert!(wheel_batch.events().iter().all(|event| event.provenance
        == pointer_provenance(wheel_modifiers, Some(wheel_timestamp), Some(wheel_range))));
    assert_eq!(wheel_batch.value_change(), Some(0.55));
    let wheel_round_trip =
        KnobEditBatch::from_events(wheel_batch.events()).expect("wheel round trip");
    assert_eq!(wheel_batch, wheel_round_trip);
    assert_eq!(format!("{wheel_batch:?}"), format!("{wheel_round_trip:?}"));
    assert_eq!(wheel_batch.value_change(), wheel_round_trip.value_change());

    let mut reset = retained_knob(5, 0.8);
    reset.knob = reset.knob.with_default_value(0.25);
    let reset_modifiers = PointerModifiers {
        shift: true,
        alt: true,
        ..PointerModifiers::default()
    };
    let reset_timestamp = InputTimestamp::capture();
    let Some(reset_batch) = reset.handle_edit_input(
        bounds,
        WidgetInput::pointer_double_click_with_timestamp(
            Point::new(20.0, 20.0),
            PointerButton::Primary,
            reset_modifiers,
            Some(reset_timestamp),
        ),
    ) else {
        panic!("reset should emit an atomic batch");
    };
    assert_eq!(
        phases(reset_batch),
        vec![EditPhase::Begin, EditPhase::Update, EditPhase::Commit]
    );
    assert_eq!(reset_batch.value_change(), Some(0.25));
    assert!(reset_batch.events().iter().all(|event| event.provenance
        == pointer_provenance(reset_modifiers, Some(reset_timestamp), None)));
    assert_eq!(reset.knob.state.value, 0.25);

    let mut no_op_reset = retained_knob(11, 0.25);
    no_op_reset.knob = no_op_reset.knob.with_default_value(0.25);
    let Some(no_op_reset_batch) = no_op_reset.handle_edit_input(
        bounds,
        WidgetInput::pointer_double_click(
            Point::new(20.0, 20.0),
            PointerButton::Primary,
            PointerModifiers::default(),
        ),
    ) else {
        panic!("no-op reset should emit an atomic batch");
    };
    let no_op_reset_round_trip =
        KnobEditBatch::from_events(no_op_reset_batch.events()).expect("reset round trip");
    assert_eq!(no_op_reset_batch, no_op_reset_round_trip);
    assert_eq!(
        format!("{no_op_reset_batch:?}"),
        format!("{no_op_reset_round_trip:?}")
    );
    assert_eq!(no_op_reset_batch.value_change(), Some(0.25));
    assert_eq!(
        no_op_reset_batch.value_change(),
        no_op_reset_round_trip.value_change()
    );
}

#[test]
fn legacy_knob_paths_preserve_focus_terminal_value_and_suppress_capture_cancel() {
    use crate::application::IntoView;

    let bounds = bounds();
    let mut builder_surface: UiSurface<KnobMessage> = crate::application::knob(0.5)
        .sensitivity(0.01)
        .message(|message| message)
        .id(20)
        .into_surface();
    let press = builder_surface
        .dispatch_widget_input(
            20,
            bounds,
            WidgetInput::primary_press(Point::new(20.0, 20.0)),
        )
        .expect("builder press output");
    assert_eq!(
        builder_surface.dispatch_widget_output(20, press),
        Some(KnobMessage::GestureStarted {
            value: 0.5,
            metadata: KnobPointerMetadata::default(),
        })
    );
    let update = builder_surface
        .dispatch_widget_input(
            20,
            bounds,
            WidgetInput::pointer_move(Point::new(20.0, 10.0)),
        )
        .expect("builder update output");
    assert!(matches!(
        builder_surface.dispatch_widget_output(20, update),
        Some(KnobMessage::ValueChanged { value, .. }) if (value - 0.6).abs() < f32::EPSILON
    ));
    let focus_loss = builder_surface
        .dispatch_widget_input(20, bounds, WidgetInput::FocusChanged(false))
        .expect("builder focus-loss output");
    assert_eq!(
        builder_surface.dispatch_widget_output(20, focus_loss),
        Some(KnobMessage::GestureEnded {
            value: 0.6,
            metadata: KnobPointerMetadata::empty(),
        })
    );

    let mut builder_no_op: UiSurface<KnobMessage> = crate::application::knob(0.5)
        .message(|message| message)
        .id(21)
        .into_surface();
    let _ = builder_no_op.dispatch_widget_input(
        21,
        bounds,
        WidgetInput::primary_press(Point::new(20.0, 20.0)),
    );
    let focus_loss = builder_no_op
        .dispatch_widget_input(21, bounds, WidgetInput::FocusChanged(false))
        .expect("builder no-op focus-loss output");
    assert_eq!(
        builder_no_op.dispatch_widget_output(21, focus_loss),
        Some(KnobMessage::GestureEnded {
            value: 0.5,
            metadata: KnobPointerMetadata::empty(),
        })
    );

    let mut builder_capture_source = retained_knob(22, 0.5);
    let _ = builder_capture_source
        .handle_edit_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0)));
    let builder_capture_cancel =
        Widget::handle_pointer_capture_cancelled(&mut builder_capture_source, bounds)
            .expect("builder capture cancellation output");
    assert_eq!(
        builder_no_op.dispatch_widget_output(21, builder_capture_cancel),
        None
    );

    let mut surface: UiSurface<KnobMessage> = UiSurface::new(SurfaceNode::knob_mapped(
        23,
        0.5,
        WidgetSizing::fixed(Vector2::new(40.0, 40.0)),
        |message| message,
    ));
    let _ = surface.dispatch_widget_input(
        23,
        bounds,
        WidgetInput::primary_press(Point::new(20.0, 20.0)),
    );
    let focus_loss = surface
        .dispatch_widget_input(23, bounds, WidgetInput::FocusChanged(false))
        .expect("SurfaceNode focus-loss output");
    assert_eq!(
        surface.dispatch_widget_output(23, focus_loss),
        Some(KnobMessage::GestureEnded {
            value: 0.5,
            metadata: KnobPointerMetadata::empty(),
        })
    );

    let mut capture_source = retained_knob(24, 0.5);
    let _ = capture_source
        .handle_edit_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0)));
    let capture_cancel = Widget::handle_pointer_capture_cancelled(&mut capture_source, bounds)
        .expect("capture cancellation output");
    assert_eq!(surface.dispatch_widget_output(23, capture_cancel), None);
}

#[test]
fn retained_knob_reprojection_keeps_transaction_and_fresh_value_authority() {
    let bounds = bounds();
    let mut previous = retained_knob(6, 0.5);
    let Some(press) =
        previous.handle_edit_input(bounds, WidgetInput::primary_press(Point::new(20.0, 20.0)))
    else {
        panic!("pointer press should begin a retained transaction");
    };
    let transaction = press.transaction();
    let _ = previous.handle_edit_input(bounds, WidgetInput::pointer_move(Point::new(20.0, 10.0)));

    let mut current = retained_knob(6, 0.75);
    current.synchronize_from_previous(&previous);
    assert_eq!(current.knob.state.value, 0.75);
    assert!(current.knob.common.state.pressed);
    assert_eq!(
        current.knob.state.gesture_origin,
        Some(Point::new(20.0, 10.0))
    );

    let Some(update) =
        current.handle_edit_input(bounds, WidgetInput::pointer_move(Point::new(20.0, 0.0)))
    else {
        panic!("reprojected pointer move should continue the transaction");
    };
    assert_eq!(update.events()[0].transaction, transaction);
    assert_eq!(current.knob.state.value, 0.85);

    let Some(cancel) = current.handle_edit_input(bounds, WidgetInput::FocusChanged(false)) else {
        panic!("focus loss should cancel the retained transaction");
    };
    assert_eq!(cancel.events()[0].phase, EditPhase::Cancel);
    assert_eq!(cancel.events()[0].start_value, 0.5);
    assert_eq!(current.knob.state.value, 0.5);

    let mut disabled = retained_knob(6, 0.75);
    disabled.knob.common.state.disabled = true;
    disabled.synchronize_from_previous(&previous);
    assert!(!disabled.knob.common.state.pressed);
    assert_eq!(
        disabled.handle_edit_input(
            bounds,
            WidgetInput::pointer_release(
                Point::new(20.0, 0.0),
                PointerButton::Primary,
                Default::default()
            ),
        ),
        None
    );
    assert_eq!(disabled.knob.state.value, 0.75);
}

#[test]
fn knob_mappers_preserve_legacy_projection_and_accept_typed_batches() {
    let bounds = bounds();
    let mut typed: UiSurface<KnobEditBatch> = UiSurface::new(SurfaceNode::knob_edits_mapped(
        7,
        0.5,
        WidgetSizing::fixed(Vector2::new(40.0, 40.0)),
        |batch| batch,
    ));
    let Some(batch) = typed
        .dispatch_widget_input(
            7,
            bounds,
            WidgetInput::primary_press(Point::new(20.0, 20.0)),
        )
        .and_then(|output| output.typed_copied::<KnobEditBatch>())
    else {
        panic!("typed SurfaceNode constructor should emit KnobEditBatch");
    };
    assert_eq!(phases(batch), vec![EditPhase::Begin]);

    let concise: UiSurface<KnobMessage> = UiSurface::new(SurfaceNode::knob_mapped(
        8,
        0.5,
        WidgetSizing::fixed(Vector2::new(40.0, 40.0)),
        |message| message,
    ));
    assert_eq!(
        concise.dispatch_widget_output(8, WidgetOutput::typed(batch)),
        Some(KnobMessage::GestureStarted {
            value: 0.5,
            metadata: KnobPointerMetadata::default(),
        })
    );
    assert_eq!(
        concise.dispatch_widget_output(
            8,
            WidgetOutput::typed(KnobMessage::Reset {
                value: 0.25,
                metadata: KnobPointerMetadata::default(),
            }),
        ),
        Some(KnobMessage::Reset {
            value: 0.25,
            metadata: KnobPointerMetadata::default(),
        })
    );

    let keyboard_provenance = InteractionProvenance::Keyboard { timestamp: None };
    let begin = EditEvent::begin(0.25, keyboard_provenance);
    let update = begin.update(0.5, keyboard_provenance).expect("update");
    let keyboard = update.commit(0.5, keyboard_provenance).expect("commit");
    let keyboard_batch = KnobEditBatch::from_events(&[begin, update, keyboard]).expect("batch");
    assert!(matches!(
        concise.dispatch_widget_output(8, WidgetOutput::typed(keyboard_batch)),
        Some(KnobMessage::KeyboardGesture(_))
    ));
}
