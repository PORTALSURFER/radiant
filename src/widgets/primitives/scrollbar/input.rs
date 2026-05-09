//! Scrollbar pointer and keyboard interaction behavior.

use crate::gui::types::{Point, Rect};
use crate::widgets::interaction::{PointerButton, ScrollbarMessage, WidgetInput, WidgetKey};
use crate::widgets::primitives::support::{
    clamp_fraction, leading_arrow_for_axis, trailing_arrow_for_axis,
};

use super::ScrollbarWidget;
use super::geometry::{axis_length, axis_position, axis_start};

pub(super) fn handle_scrollbar_input(
    scrollbar: &mut ScrollbarWidget,
    bounds: Rect,
    input: WidgetInput,
) -> Option<ScrollbarMessage> {
    if scrollbar.common.state.disabled {
        scrollbar.common.state.pressed = false;
        scrollbar.state.drag_grip_fraction = None;
        return None;
    }
    match input {
        WidgetInput::PointerMove { position } => {
            scrollbar.common.state.hovered = bounds.contains(position);
            drag_to(scrollbar, bounds, position)
        }
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
        } if bounds.contains(position) => {
            scrollbar.common.state.focused = true;
            scrollbar.common.state.hovered = true;
            let thumb = scrollbar.thumb_rect(bounds);
            if thumb.contains(position) {
                scrollbar.common.state.pressed = true;
                scrollbar.state.drag_grip_fraction =
                    Some(pointer_grip_fraction(scrollbar, thumb, position));
                None
            } else {
                scrollbar.set_offset_fraction(centered_offset_fraction(scrollbar, bounds, position))
            }
        }
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
        } => {
            scrollbar.common.state.hovered = bounds.contains(position);
            scrollbar.common.state.pressed = false;
            scrollbar.state.drag_grip_fraction = None;
            None
        }
        WidgetInput::FocusChanged(focused) => {
            scrollbar.common.state.focused = focused;
            if !focused {
                scrollbar.common.state.pressed = false;
                scrollbar.state.drag_grip_fraction = None;
            }
            None
        }
        WidgetInput::KeyPress(key) if scrollbar.common.state.focused => {
            handle_key_input(scrollbar, key)
        }
        _ => None,
    }
}

fn handle_key_input(scrollbar: &mut ScrollbarWidget, key: WidgetKey) -> Option<ScrollbarMessage> {
    let delta = if key == leading_arrow_for_axis(scrollbar.props.axis) {
        Some(-scrollbar.props.step_fraction)
    } else if key == trailing_arrow_for_axis(scrollbar.props.axis) {
        Some(scrollbar.props.step_fraction)
    } else {
        None
    };
    match key {
        WidgetKey::Home => scrollbar.set_offset_fraction(0.0),
        WidgetKey::End => scrollbar.set_offset_fraction(1.0),
        _ => delta
            .and_then(|step| scrollbar.set_offset_fraction(scrollbar.state.offset_fraction + step)),
    }
}

fn drag_to(
    scrollbar: &mut ScrollbarWidget,
    bounds: Rect,
    position: Point,
) -> Option<ScrollbarMessage> {
    let grip_fraction = scrollbar.state.drag_grip_fraction?;
    let track_length = axis_length(scrollbar.props.axis, bounds);
    let thumb_fraction = scrollbar.thumb_fraction(track_length);
    let thumb_length = track_length * thumb_fraction;
    let pointer_axis = axis_position(scrollbar.props.axis, position);
    let start = pointer_axis - thumb_length * grip_fraction;
    let free_track = (track_length - thumb_length).max(0.0);
    let offset = if free_track <= f32::EPSILON {
        0.0
    } else {
        (start - axis_start(scrollbar.props.axis, bounds)) / free_track
    };
    scrollbar.set_offset_fraction(offset)
}

fn centered_offset_fraction(scrollbar: &ScrollbarWidget, bounds: Rect, position: Point) -> f32 {
    let track_length = axis_length(scrollbar.props.axis, bounds);
    let thumb_fraction = scrollbar.thumb_fraction(track_length);
    let thumb_length = track_length * thumb_fraction;
    let centered_start = axis_position(scrollbar.props.axis, position)
        - axis_start(scrollbar.props.axis, bounds)
        - thumb_length * 0.5;
    let free_track = (track_length - thumb_length).max(0.0);
    if free_track <= f32::EPSILON {
        0.0
    } else {
        centered_start / free_track
    }
}

fn pointer_grip_fraction(scrollbar: &ScrollbarWidget, thumb: Rect, position: Point) -> f32 {
    let grip =
        axis_position(scrollbar.props.axis, position) - axis_start(scrollbar.props.axis, thumb);
    let thumb_length = axis_length(scrollbar.props.axis, thumb).max(1.0);
    clamp_fraction(grip / thumb_length)
}
