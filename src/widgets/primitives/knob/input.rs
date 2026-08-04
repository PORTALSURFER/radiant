//! Knob input behavior for the retained shared-edit adapter.

use crate::{
    gui::{
        input::{InputSequenceRange, InputTimestamp},
        types::Rect,
    },
    widgets::interaction::{
        EditEvent, InteractionProvenance, KnobEditBatch, PointerButton, PointerModifiers,
        WidgetInput, WidgetKey,
    },
};

use super::KnobWidget;

pub(super) fn handle_knob_edit_input(
    knob: &mut KnobWidget,
    active_edit: &mut Option<EditEvent<f32>>,
    bounds: Rect,
    input: WidgetInput,
) -> Option<KnobEditBatch> {
    match input {
        WidgetInput::PointerMove {
            position,
            modifiers,
            timestamp,
            sequence_range,
        } => {
            knob.common.state.hovered = bounds.contains(position);
            if !knob.is_editable() || !knob.common.state.pressed || active_edit.is_none() {
                return None;
            }

            let origin = knob.state.gesture_origin.unwrap_or(position);
            knob.state.gesture_origin = Some(position);
            let sensitivity = if knob.state.fine_adjustment {
                knob.props.sensitivity * 0.1
            } else {
                knob.props.sensitivity
            };
            let candidate =
                finite_clamped(knob.state.value + (origin.y - position.y) * sensitivity)?;
            if !set_value_if_changed(knob, candidate) {
                return None;
            }

            let provenance = pointer_provenance(modifiers, timestamp, sequence_range);
            let previous = (*active_edit)?;
            let update = previous.update(candidate, provenance)?;
            *active_edit = Some(update);
            KnobEditBatch::pointer(&[update])
        }
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
            modifiers,
            timestamp,
        } if bounds.contains(position)
            && knob.is_editable()
            && !knob.common.state.pressed
            && active_edit.is_none() =>
        {
            knob.common.state.hovered = true;
            knob.common.state.pressed = true;
            knob.common.state.focused = true;
            knob.state.fine_adjustment = modifiers.shift;
            knob.state.gesture_origin = Some(position);

            let begin = EditEvent::begin(
                knob.state.value,
                pointer_provenance(modifiers, timestamp, None),
            );
            *active_edit = Some(begin);
            KnobEditBatch::pointer(&[begin])
        }
        WidgetInput::PointerRelease {
            button: PointerButton::Primary,
            modifiers,
            timestamp,
            ..
        }
        | WidgetInput::PointerDrop {
            button: PointerButton::Primary,
            modifiers,
            timestamp,
            ..
        } => {
            if active_edit.is_none() {
                return None;
            }
            if !knob.is_editable() {
                return cancel_active_pointer_edit(
                    knob,
                    active_edit,
                    pointer_provenance(modifiers, timestamp, None),
                    None,
                );
            }

            let previous = (*active_edit)?;
            let commit = previous.commit(
                knob.state.value,
                pointer_provenance(modifiers, timestamp, None),
            )?;
            *active_edit = None;
            clear_pointer_state(knob);
            KnobEditBatch::pointer(&[commit])
        }
        WidgetInput::PointerModifiersChanged { modifiers, .. } => {
            if knob.common.state.pressed && active_edit.is_some() {
                knob.state.fine_adjustment = modifiers.shift;
            }
            None
        }
        WidgetInput::Wheel {
            position,
            delta,
            modifiers,
            timestamp,
            sequence_range,
        } if knob.is_editable()
            && active_edit.is_none()
            && !knob.common.state.pressed
            && knob.state.gesture_origin.is_none()
            && bounds.contains(position) =>
        {
            let direction = if delta.y > 0.0 {
                1.0
            } else if delta.y < 0.0 {
                -1.0
            } else {
                return None;
            };
            let step = if modifiers.shift {
                super::super::knob::WHEEL_FINE_STEP
            } else {
                super::super::knob::WHEEL_STEP
            };
            let start_value = knob.state.value;
            let final_value = finite_clamped(start_value + direction * step)?;
            if !set_value_if_changed(knob, final_value) {
                return None;
            }

            let provenance = pointer_provenance(modifiers, timestamp, sequence_range);
            let begin = EditEvent::begin(start_value, provenance);
            let update = begin.update(final_value, provenance)?;
            let commit = update.commit(final_value, provenance)?;
            KnobEditBatch::wheel(&[begin, update, commit])
        }
        WidgetInput::PointerDoubleClick {
            position,
            button: PointerButton::Primary,
            modifiers,
            timestamp,
        } if knob.props.reset_on_double_click
            && knob.is_editable()
            && active_edit.is_none()
            && !knob.common.state.pressed
            && knob.state.gesture_origin.is_none()
            && bounds.contains(position) =>
        {
            let start_value = knob.state.value;
            let final_value = knob.props.default_value;
            let provenance = pointer_provenance(modifiers, timestamp, None);
            let begin = EditEvent::begin(start_value, provenance);
            let update = begin.update(final_value, provenance)?;
            let commit = update.commit(final_value, provenance)?;
            knob.state.value = final_value;
            clear_pointer_state(knob);
            KnobEditBatch::reset(&[begin, update, commit])
        }
        WidgetInput::FocusChanged(focused) => {
            knob.common.state.focused = focused;
            if focused {
                None
            } else {
                cancel_active_pointer_edit(
                    knob,
                    active_edit,
                    pointer_provenance_empty(),
                    Some(CancellationReason::FocusLoss),
                )
            }
        }
        WidgetInput::KeyPress { key, timestamp }
            if knob.common.state.focused
                && knob.is_editable()
                && active_edit.is_none()
                && !knob.common.state.pressed
                && knob.state.gesture_origin.is_none() =>
        {
            let candidate = keyboard_candidate(knob.state.value, knob.props.sensitivity, key)?;
            let start_value = knob.state.value;
            if !values_differ(start_value, candidate) {
                return None;
            }
            let provenance = InteractionProvenance::Keyboard { timestamp };
            let begin = EditEvent::begin(start_value, provenance);
            let update = begin.update(candidate, provenance)?;
            let commit = update.commit(candidate, provenance)?;
            knob.state.value = candidate;
            KnobEditBatch::keyboard(&[begin, update, commit])
        }
        _ => None,
    }
}

pub(super) fn handle_pointer_capture_cancelled(
    knob: &mut KnobWidget,
    active_edit: &mut Option<EditEvent<f32>>,
) -> Option<KnobEditBatch> {
    knob.common.state.focused = false;
    cancel_active_pointer_edit(
        knob,
        active_edit,
        pointer_provenance_empty(),
        Some(CancellationReason::PointerCapture),
    )
}

#[derive(Clone, Copy)]
enum CancellationReason {
    FocusLoss,
    PointerCapture,
}

fn cancel_active_pointer_edit(
    knob: &mut KnobWidget,
    active_edit: &mut Option<EditEvent<f32>>,
    provenance: InteractionProvenance,
    reason: Option<CancellationReason>,
) -> Option<KnobEditBatch> {
    let Some(previous) = active_edit.take() else {
        clear_pointer_state(knob);
        return None;
    };
    let legacy_terminal_value = knob.state.value;
    let meaningful = values_differ(legacy_terminal_value, previous.start_value);
    knob.state.value = previous.start_value;
    clear_pointer_state(knob);
    let cancel = previous.cancel(provenance)?;
    match reason {
        Some(CancellationReason::FocusLoss) => Some(KnobEditBatch::focus_loss(
            cancel,
            meaningful,
            legacy_terminal_value,
        )),
        Some(CancellationReason::PointerCapture) => Some(KnobEditBatch::pointer_capture(
            cancel,
            meaningful,
            legacy_terminal_value,
        )),
        None if meaningful => Some(KnobEditBatch::rollback(cancel)),
        None => None,
    }
}

fn clear_pointer_state(knob: &mut KnobWidget) {
    knob.common.state.pressed = false;
    knob.state.fine_adjustment = false;
    knob.state.gesture_origin = None;
}

fn set_value_if_changed(knob: &mut KnobWidget, value: f32) -> bool {
    if !values_differ(knob.state.value, value) {
        return false;
    }
    knob.state.value = value;
    true
}

fn keyboard_candidate(value: f32, sensitivity: f32, key: WidgetKey) -> Option<f32> {
    let candidate = match key {
        WidgetKey::ArrowLeft | WidgetKey::ArrowDown => value - sensitivity * 16.0,
        WidgetKey::ArrowRight | WidgetKey::ArrowUp => value + sensitivity * 16.0,
        WidgetKey::Home => 0.0,
        WidgetKey::End => 1.0,
        _ => return None,
    };
    finite_clamped(candidate)
}

fn finite_clamped(value: f32) -> Option<f32> {
    value.is_finite().then(|| value.clamp(0.0, 1.0))
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

fn pointer_provenance_empty() -> InteractionProvenance {
    pointer_provenance(PointerModifiers::default(), None, None)
}

fn values_differ(left: f32, right: f32) -> bool {
    if left.is_finite() && right.is_finite() {
        (left - right).abs() > f32::EPSILON
    } else {
        left != right
    }
}
