use super::*;
use crate::gui::types::Vector2;
use crate::widgets::{EditPhase, InteractionSource, WheelDelta, WidgetKey, WidgetSizing};

fn fixture() -> (RetainedScrollbarWidget, Rect) {
    let mut widget = ScrollbarWidget::new(
        7,
        ScrollbarAxis::Vertical,
        WidgetSizing::fixed(Vector2::new(12.0, 120.0)),
    );
    widget.props.viewport_fraction = 0.25;
    widget.common.state.focused = true;
    widget.state.offset_fraction = 0.25;
    (
        RetainedScrollbarWidget::new(widget),
        Rect::from_min_size(Point::default(), Vector2::new(12.0, 120.0)),
    )
}
fn phases(batch: ScrollbarEditBatch) -> Vec<EditPhase> {
    batch.events().iter().map(|e| e.phase).collect()
}
fn grip(widget: &RetainedScrollbarWidget, bounds: Rect) -> Point {
    let thumb = widget.scrollbar.thumb_rect(bounds);
    Point::new(6.0, thumb.min.y + thumb.height() / 2.0)
}
fn wheel(phase: Option<WheelPhase>, y: f32) -> WheelSample {
    WheelSample::new(
        WheelDelta::pixels(Vector2::new(0.0, y)).unwrap(),
        phase,
        Default::default(),
    )
    .unwrap()
}

#[test]
fn pointer_release_includes_final_motion_and_commits_once() {
    let (mut widget, bounds) = fixture();
    let point = grip(&widget, bounds);
    let begin = widget
        .edit_input(bounds, WidgetInput::primary_press(point))
        .unwrap();
    assert_eq!(phases(begin), [EditPhase::Begin]);
    let end = widget
        .edit_input(
            bounds,
            WidgetInput::primary_release(Point::new(6.0, point.y + 45.0)),
        )
        .unwrap();
    assert_eq!(phases(end), [EditPhase::Update, EditPhase::Commit]);
    assert_eq!(end.transaction(), begin.transaction());
    assert_eq!(end.offset_change(), Some(0.75));
    assert!(
        widget
            .edit_input(bounds, WidgetInput::primary_release(point))
            .is_none()
    );
    assert!(widget.cancel().is_none());
}

#[test]
fn capture_loss_rolls_back_once_and_noop_cancel_is_still_typed() {
    for moved in [false, true] {
        let (mut widget, bounds) = fixture();
        let point = grip(&widget, bounds);
        let begin = widget
            .edit_input(bounds, WidgetInput::primary_press(point))
            .unwrap();
        if moved {
            widget
                .edit_input(
                    bounds,
                    WidgetInput::pointer_move(Point::new(6.0, point.y + 45.0)),
                )
                .unwrap();
        }
        let output = Widget::handle_pointer_capture_cancelled(&mut widget, bounds).unwrap();
        let cancel = output.typed_cloned::<ScrollbarEditBatch>().unwrap();
        assert_eq!(phases(cancel), [EditPhase::Cancel]);
        assert_eq!(cancel.transaction(), begin.transaction());
        assert_eq!(cancel.offset_change(), moved.then_some(0.25));
        assert_eq!(widget.scrollbar.state.offset_fraction, 0.25);
        assert!(widget.cancel().is_none());
        assert!(
            widget
                .edit_input(bounds, WidgetInput::primary_release(point))
                .is_none()
        );
    }
}

#[test]
fn returning_to_start_is_an_effective_update_not_suppressed() {
    let (mut widget, bounds) = fixture();
    let point = grip(&widget, bounds);
    widget
        .edit_input(bounds, WidgetInput::primary_press(point))
        .unwrap();
    widget
        .edit_input(
            bounds,
            WidgetInput::pointer_move(Point::new(6.0, point.y + 45.0)),
        )
        .unwrap();
    let back = widget
        .edit_input(bounds, WidgetInput::pointer_move(point))
        .unwrap();
    assert_eq!(phases(back), [EditPhase::Update]);
    assert_eq!(back.offset_change(), Some(0.25));
    assert_eq!(widget.cancel().unwrap().offset_change(), None);
}

#[test]
fn controlled_replacement_retires_old_pointer_without_overwriting_new_offset() {
    let (mut previous, bounds) = fixture();
    let point = grip(&previous, bounds);
    let begin = previous
        .edit_input(bounds, WidgetInput::primary_press(point))
        .unwrap();
    previous
        .edit_input(bounds, WidgetInput::pointer_move(Point::new(6.0, 70.0)))
        .unwrap();
    let (mut next, _) = fixture();
    next.scrollbar.state.offset_fraction = 0.9;
    next.synchronize_from_previous(&previous);
    let cancel = next
        .edit_input(bounds, WidgetInput::pointer_move(Point::new(6.0, 20.0)))
        .unwrap();
    assert_eq!(phases(cancel), [EditPhase::Cancel]);
    assert_eq!(cancel.transaction(), begin.transaction());
    assert_eq!(cancel.offset_change(), None);
    assert_eq!(next.scrollbar.state.offset_fraction, 0.9);
    assert!(
        next.edit_input(bounds, WidgetInput::primary_release(point))
            .is_none()
    );
}

#[test]
fn controlled_echo_preserves_live_transaction_and_geometry_replacement_retires_it() {
    let (mut previous, bounds) = fixture();
    let point = grip(&previous, bounds);
    let begin = previous
        .edit_input(bounds, WidgetInput::primary_press(point))
        .unwrap();
    previous
        .edit_input(
            bounds,
            WidgetInput::pointer_move(Point::new(6.0, point.y + 45.0)),
        )
        .unwrap();
    let (mut next, _) = fixture();
    next.scrollbar.state.offset_fraction = 0.75;
    next.synchronize_from_previous(&previous);
    assert_eq!(next.active.unwrap().1.transaction, begin.transaction());
    let (mut resized, _) = fixture();
    resized.scrollbar.state.offset_fraction = 0.75;
    resized.scrollbar.props.viewport_fraction = 0.5;
    resized.synchronize_from_previous(&next);
    assert!(resized.active.is_none());
    assert_eq!(phases(resized.cancel().unwrap()), [EditPhase::Cancel]);
    assert_eq!(resized.scrollbar.state.offset_fraction, 0.75);
}

#[test]
fn keyboard_repeats_are_distinct_atomic_edits_and_bounds_are_noops() {
    let (mut widget, bounds) = fixture();
    widget.scrollbar.common.state.focused = true;
    let mut previous = None;
    for _ in 0..2 {
        let edit = widget
            .edit_input(bounds, WidgetInput::key_press(WidgetKey::ArrowDown))
            .unwrap();
        assert_eq!(
            phases(edit),
            [EditPhase::Begin, EditPhase::Update, EditPhase::Commit]
        );
        assert_ne!(Some(edit.transaction()), previous);
        previous = Some(edit.transaction());
        assert!(
            edit.events()
                .iter()
                .all(|e| e.provenance.source() == InteractionSource::Keyboard)
        );
    }
    widget
        .edit_input(bounds, WidgetInput::key_press(WidgetKey::End))
        .unwrap();
    assert!(
        widget
            .edit_input(bounds, WidgetInput::key_press(WidgetKey::ArrowDown))
            .is_none()
    );
    assert!(widget.active.is_none());
}

#[test]
fn explicit_wheel_burst_has_one_transaction_and_duplicate_end_is_inert() {
    let (mut widget, bounds) = fixture();
    let point = Point::new(6.0, 60.0);
    let begin = widget
        .wheel(bounds, point, wheel(Some(WheelPhase::Started), 36.0))
        .unwrap();
    assert_eq!(phases(begin), [EditPhase::Begin, EditPhase::Update]);
    assert!(widget.retains_managed_wheel_sequence());
    assert!(
        widget
            .wheel(bounds, point, wheel(Some(WheelPhase::Started), 36.0))
            .is_none()
    );
    for phase in [WheelPhase::Changed, WheelPhase::Ended] {
        let edit = widget
            .wheel(bounds, Point::new(40.0, 150.0), wheel(Some(phase), 36.0))
            .unwrap();
        assert_eq!(edit.transaction(), begin.transaction());
        assert_eq!(
            phases(edit),
            if phase == WheelPhase::Ended {
                vec![EditPhase::Update, EditPhase::Commit]
            } else {
                vec![EditPhase::Update]
            }
        );
    }
    assert!(!widget.retains_managed_wheel_sequence());
    assert!(
        widget
            .wheel(bounds, point, wheel(Some(WheelPhase::Ended), 36.0))
            .is_none()
    );
    assert!(
        widget
            .wheel(bounds, point, wheel(Some(WheelPhase::Changed), 36.0))
            .is_none()
    );
}

#[test]
fn wheel_cancel_and_focus_loss_restore_start_with_no_duplicate_terminal() {
    for focus_loss in [false, true] {
        let (mut widget, bounds) = fixture();
        let point = Point::new(6.0, 60.0);
        widget
            .wheel(bounds, point, wheel(Some(WheelPhase::Started), 36.0))
            .unwrap();
        let cancel = if focus_loss {
            widget.edit_input(bounds, WidgetInput::FocusChanged(false))
        } else {
            widget.wheel(bounds, point, wheel(Some(WheelPhase::Cancelled), 0.0))
        }
        .unwrap();
        assert_eq!(phases(cancel), [EditPhase::Cancel]);
        assert_eq!(cancel.offset_change(), Some(0.25));
        assert_eq!(widget.scrollbar.state.offset_fraction, 0.25);
        assert!(
            widget
                .wheel(bounds, point, wheel(Some(WheelPhase::Ended), 36.0))
                .is_none()
        );
    }
}

#[test]
fn discrete_and_phaseless_wheel_are_atomic_and_cannot_join_pointer_owner() {
    for phase in [None, Some(WheelPhase::Discrete)] {
        let (mut widget, bounds) = fixture();
        let point = grip(&widget, bounds);
        let edit = widget.wheel(bounds, point, wheel(phase, 36.0)).unwrap();
        assert_eq!(
            phases(edit),
            [EditPhase::Begin, EditPhase::Update, EditPhase::Commit]
        );
        assert!(!widget.retains_managed_wheel_sequence());
        let point = grip(&widget, bounds);
        widget
            .edit_input(bounds, WidgetInput::primary_press(point))
            .unwrap();
        assert!(widget.wheel(bounds, point, wheel(phase, 36.0)).is_none());
        assert_eq!(widget.scrollbar.state.offset_fraction, 0.35);
    }
}

#[test]
fn semantic_edits_preserve_source_and_reject_invalid_or_conflicting_requests() {
    for source in [
        SemanticActionSource::Accessibility,
        SemanticActionSource::Programmatic,
    ] {
        let (mut widget, bounds) = fixture();
        let WidgetSemanticActionResult::Accepted(Some(output)) = widget.dispatch(
            SemanticAction::Numeric(NumericAccessibilityAction::Increment),
            source,
        ) else {
            panic!("accepted edit");
        };
        let batch = output.typed_cloned::<ScrollbarEditBatch>().unwrap();
        assert_eq!(
            phases(batch),
            [EditPhase::Begin, EditPhase::Update, EditPhase::Commit]
        );
        assert!(
            batch
                .events()
                .iter()
                .all(|e| e.provenance == source.provenance())
        );
        for text in ["NaN", "inf", "nonsense"] {
            assert!(matches!(
                widget.dispatch(
                    SemanticAction::Numeric(NumericAccessibilityAction::SetValueText(text.into())),
                    source
                ),
                WidgetSemanticActionResult::Unsupported
            ));
        }
        let point = grip(&widget, bounds);
        widget
            .edit_input(bounds, WidgetInput::primary_press(point))
            .unwrap();
        assert!(matches!(
            widget.dispatch(
                SemanticAction::Numeric(NumericAccessibilityAction::Increment),
                source
            ),
            WidgetSemanticActionResult::Unsupported
        ));
    }
}

#[test]
fn nonfinite_pointer_and_readonly_state_cannot_publish_late_update() {
    let (mut widget, bounds) = fixture();
    let point = grip(&widget, bounds);
    widget
        .edit_input(bounds, WidgetInput::primary_press(point))
        .unwrap();
    assert!(
        widget
            .edit_input(bounds, WidgetInput::pointer_move(Point::new(6.0, f32::NAN)))
            .is_none()
    );
    widget.scrollbar.common.state.read_only = true;
    assert_eq!(
        phases(
            widget
                .edit_input(bounds, WidgetInput::pointer_move(Point::new(6.0, 90.0)))
                .unwrap()
        ),
        [EditPhase::Cancel]
    );
    assert!(
        widget
            .edit_input(bounds, WidgetInput::primary_release(point))
            .is_none()
    );
    assert_eq!(widget.scrollbar.state.offset_fraction, 0.25);
}

#[test]
fn cancellation_does_not_relabel_the_previous_native_timestamp_as_a_loss_timestamp() {
    let (mut widget, bounds) = fixture();
    let point = grip(&widget, bounds);
    let stamp = crate::gui::input::InputTimestamp::capture();
    let begin = widget
        .edit_input(
            bounds,
            WidgetInput::PointerPress {
                position: point,
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: Some(stamp),
            },
        )
        .unwrap();
    assert!(matches!(
        begin.events()[0].provenance,
        InteractionProvenance::Pointer {
            timestamp: Some(_),
            ..
        }
    ));
    let cancel = widget.cancel().unwrap();
    assert!(matches!(
        cancel.events()[0].provenance,
        InteractionProvenance::Pointer {
            timestamp: None,
            sequence_range: None,
            ..
        }
    ));
}
