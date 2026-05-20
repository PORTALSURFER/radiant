//! Frame-driven animation through the application builder.

use radiant::prelude::*;
use radiant::theme::ThemeTokens;

#[path = "animation_showcase/model.rs"]
mod model;
#[path = "animation_showcase/pulse_meter.rs"]
mod pulse_meter;
#[path = "animation_showcase/view.rs"]
mod view;

use model::{AnimationMessage, AnimationState};
use pulse_meter::{pulse_meter_frame, pulse_meter_revision};
use view::animation_view;

#[cfg(test)]
#[path = "animation_showcase/tests.rs"]
mod tests;

fn main() -> radiant::Result {
    radiant::app(AnimationState::default())
        .title("Radiant Animation Showcase")
        .size(520, 220)
        .min_size(420, 180)
        .view(animation_view)
        .animation(|state| state.running)
        .on_frame(|| AnimationMessage::Frame)
        .retained_painter(30, |state, _descriptor, rect, _viewport| {
            Some(pulse_meter_frame(
                state.phase,
                state.running,
                rect,
                &ThemeTokens::default(),
            ))
        })
        .update(|state, message| match message {
            AnimationMessage::Toggle => state.running = !state.running,
            AnimationMessage::Frame => state.tick(),
            AnimationMessage::Reset => state.reset(),
        })
        .run()
}

fn phase_meter(phase: f32, running: bool) -> View<AnimationMessage> {
    retained_canvas_with(30, pulse_meter_revision(phase, running), 0, true)
        .view()
        .height(42.0)
        .key("phase-meter")
        .fill_width()
}
