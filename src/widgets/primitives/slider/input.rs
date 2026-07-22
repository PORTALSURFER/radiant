//! Slider pointer and keyboard interaction behavior.

use crate::gui::types::Rect;
use crate::widgets::interaction::{PointerButton, SliderMessage, WidgetInput, WidgetKey};

use super::{SliderWidget, geometry::value_for_position};

pub(super) fn handle_slider_input(
    slider: &mut SliderWidget,
    bounds: Rect,
    input: WidgetInput,
) -> Option<SliderMessage> {
    match input {
        WidgetInput::PointerMove { position } => {
            slider.common.state.hovered = bounds.contains(position);
            slider
                .common
                .state
                .pressed
                .then(|| {
                    slider.set_value(value_for_position(
                        bounds,
                        position,
                        slider.props.track_height,
                    ))
                })
                .flatten()
        }
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
            ..
        } if bounds.contains(position) => {
            slider.common.state.hovered = true;
            slider.common.state.pressed = true;
            slider.common.state.focused = true;
            slider.set_value(value_for_position(
                bounds,
                position,
                slider.props.track_height,
            ))
        }
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            ..
        } => {
            let was_pressed = slider.common.state.pressed;
            slider.common.state.pressed = false;
            was_pressed
                .then(|| {
                    slider.set_value(value_for_position(
                        bounds,
                        position,
                        slider.props.track_height,
                    ))
                })
                .flatten()
        }
        WidgetInput::FocusChanged(focused) => {
            slider.common.state.focused = focused;
            None
        }
        WidgetInput::KeyPress(key) if slider.common.state.focused => match key {
            WidgetKey::ArrowLeft | WidgetKey::ArrowDown => {
                slider.set_value(slider.state.value - slider.props.keyboard_step)
            }
            WidgetKey::ArrowRight | WidgetKey::ArrowUp => {
                slider.set_value(slider.state.value + slider.props.keyboard_step)
            }
            WidgetKey::Home => slider.set_value(0.0),
            WidgetKey::End => slider.set_value(1.0),
            _ => None,
        },
        _ => None,
    }
}
