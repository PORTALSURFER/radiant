//! Slider pointer and keyboard interaction behavior.

use crate::gui::types::Rect;
use crate::widgets::interaction::{
    EditEvent, InteractionProvenance, PointerButton, PointerModifiers, SliderEditBatch,
    WidgetInput, WidgetKey,
};

use super::{SliderWidget, geometry::value_for_position};

pub(super) fn handle_slider_edit_input(
    slider: &mut SliderWidget,
    bounds: Rect,
    input: WidgetInput,
) -> Option<SliderEditBatch> {
    match input {
        WidgetInput::PointerMove {
            position,
            modifiers,
            timestamp,
            sequence_range,
        } => {
            slider.common.state.hovered = bounds.contains(position);
            if !slider.is_editable() || slider.state.active_edit.is_none() {
                return None;
            }
            let candidate = finite_value_for_position(bounds, position, slider.props.track_height)?;
            if !slider.set_value_if_changed(candidate) {
                return None;
            }
            let provenance = pointer_provenance(modifiers, timestamp, sequence_range);
            let previous = slider.state.active_edit?;
            let update = previous.update(candidate, provenance)?;
            slider.state.active_edit = Some(update);
            Some(SliderEditBatch::single(update))
        }
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
            modifiers,
            timestamp,
        } if bounds.contains(position)
            && slider.is_editable()
            && slider.state.active_edit.is_none()
            && !slider.common.state.pressed =>
        {
            let candidate = finite_value_for_position(bounds, position, slider.props.track_height)?;
            let start_value = slider.state.value;
            let begin_provenance = pointer_provenance(modifiers, timestamp, None);
            let begin = EditEvent::begin(start_value, begin_provenance);
            slider.common.state.hovered = true;
            slider.common.state.pressed = true;
            slider.common.state.focused = true;

            if !values_differ(start_value, candidate) {
                slider.state.active_edit = Some(begin);
                return Some(SliderEditBatch::single(begin));
            }

            slider.state.value = candidate;
            let update = begin.update(candidate, begin_provenance)?;
            slider.state.active_edit = Some(update);
            SliderEditBatch::from_events(&[begin, update])
        }
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            modifiers,
            timestamp,
        } => {
            slider.state.active_edit?;
            if !slider.is_editable() {
                return cancel_active_pointer_edit(slider);
            }

            let candidate = finite_value_for_position(bounds, position, slider.props.track_height)
                .or_else(|| finite_clamped(slider.state.value))?;
            let provenance = pointer_provenance(modifiers, timestamp, None);
            let previous = slider.state.active_edit?;
            let batch = if values_differ(slider.state.value, candidate) {
                let update = previous.update(candidate, provenance)?;
                let commit = update.commit(candidate, provenance)?;
                slider.state.value = candidate;
                SliderEditBatch::from_events(&[update, commit])
            } else {
                previous
                    .commit(candidate, provenance)
                    .map(SliderEditBatch::single)
            };
            slider.state.active_edit = None;
            slider.common.state.pressed = false;
            batch
        }
        WidgetInput::FocusChanged(focused) => {
            slider.common.state.focused = focused;
            if focused {
                None
            } else {
                cancel_active_pointer_edit(slider)
            }
        }
        WidgetInput::KeyPress { key, timestamp }
            if slider.common.state.focused && slider.is_editable() =>
        {
            let candidate =
                keyboard_candidate(slider.state.value, slider.props.keyboard_step, key)?;
            slider.state.active_edit = None;
            let start_value = slider.state.value;
            if !values_differ(start_value, candidate) {
                return None;
            }

            let provenance = InteractionProvenance::Keyboard { timestamp };
            let begin = EditEvent::begin(start_value, provenance);
            let update = begin.update(candidate, provenance)?;
            let commit = update.commit(candidate, provenance)?;
            slider.state.value = candidate;
            SliderEditBatch::from_events(&[begin, update, commit])
        }
        _ => None,
    }
}

fn cancel_active_pointer_edit(slider: &mut SliderWidget) -> Option<SliderEditBatch> {
    let Some(previous) = slider.state.active_edit.take() else {
        slider.common.state.pressed = false;
        return None;
    };
    let meaningful = values_differ(previous.value, previous.start_value);
    slider.state.value = previous.start_value;
    slider.common.state.pressed = false;
    if !meaningful {
        return None;
    }
    let provenance = pointer_provenance(PointerModifiers::default(), None, None);
    previous.cancel(provenance).map(SliderEditBatch::rollback)
}

fn pointer_provenance(
    modifiers: PointerModifiers,
    timestamp: Option<crate::gui::input::InputTimestamp>,
    sequence_range: Option<crate::gui::input::InputSequenceRange>,
) -> InteractionProvenance {
    InteractionProvenance::Pointer {
        modifiers,
        timestamp,
        sequence_range,
    }
}

fn finite_value_for_position(
    bounds: Rect,
    position: crate::gui::types::Point,
    track_height: f32,
) -> Option<f32> {
    finite_clamped(value_for_position(bounds, position, track_height))
}

fn finite_clamped(value: f32) -> Option<f32> {
    value.is_finite().then(|| value.clamp(0.0, 1.0))
}

fn keyboard_candidate(value: f32, keyboard_step: f32, key: WidgetKey) -> Option<f32> {
    let candidate = match key {
        WidgetKey::ArrowLeft | WidgetKey::ArrowDown => value - keyboard_step,
        WidgetKey::ArrowRight | WidgetKey::ArrowUp => value + keyboard_step,
        WidgetKey::Home => 0.0,
        WidgetKey::End => 1.0,
        _ => return None,
    };
    finite_clamped(candidate)
}

fn values_differ(left: f32, right: f32) -> bool {
    if left.is_finite() && right.is_finite() {
        (left - right).abs() > f32::EPSILON
    } else {
        left != right
    }
}
