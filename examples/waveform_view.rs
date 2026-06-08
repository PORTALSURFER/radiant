//! Load one WAV file and display it as an interactive mono waveform view.

#[cfg(test)]
use radiant::{
    gui::types::{Point, Rect, Vector2},
    runtime::PaintPrimitive,
};
use std::sync::Arc;
#[cfg(test)]
use std::time::Duration;

const WAVEFORM_WIDTH: usize = 1200;
const WAVEFORM_HEIGHT: usize = 320;
const WAVEFORM_WIDGET_ID: u64 = 10;

#[path = "waveform_view/source.rs"]
mod source;
use source::*;

#[path = "waveform_view/model.rs"]
mod model;

#[path = "waveform_view/view.rs"]
mod surface_view;

#[path = "waveform_view/widget.rs"]
mod widget;

use model::*;
pub(crate) use surface_view::{paint_playhead_overlay, view};
#[cfg(test)]
use widget::WaveformWidget;

fn main() -> radiant::Result {
    let file = Arc::new(load_waveform_source(resolve_sample_path())?);
    let viewport = WaveformViewport::full(file.frames);

    radiant::app(WaveformApp {
        file,
        viewport,
        zoom_anchor_ratio: 0.5,
        playing: false,
        playhead_ratio: 0.5,
    })
    .title("Radiant Waveform View")
    .size(1280, 560)
    .min_size(820, 420)
    .view(view)
    .animated_transient_overlay_at(
        60,
        |state| state.playing,
        |state, context, primitives| {
            paint_playhead_overlay(state, context.plan, context.animation_time, primitives);
        },
    )
    .reducer(|state, message, context| {
        state.apply_interaction(message);
        context.request_repaint();
    })
    .run()
}

#[cfg(test)]
#[path = "waveform_view/tests.rs"]
mod tests;
