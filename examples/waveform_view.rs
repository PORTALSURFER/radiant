//! Load one WAV file and display it as an interactive mono waveform view.

use radiant::prelude as ui;
use radiant::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    runtime::{PaintFillRect, PaintPrimitive, SurfacePaintPlan},
    widgets::{ScrollbarAxis, ScrollbarMessage, ScrollbarWidget, WidgetSizing},
};
use std::{sync::Arc, time::Duration};

const WAVEFORM_WIDTH: usize = 1200;
const WAVEFORM_HEIGHT: usize = 320;
const WAVEFORM_WIDGET_ID: u64 = 10;

#[path = "waveform_view/source.rs"]
mod source;
use source::*;

#[path = "waveform_view/model.rs"]
mod model;

#[path = "waveform_view/widget.rs"]
mod widget;

use model::*;
use widget::*;

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
    .update_with(|state, message, context| {
        state.apply_interaction(message);
        context.request_repaint();
    })
    .run()
}

fn view(state: &mut WaveformApp) -> ui::View<WaveformInteraction> {
    let title = format!(
        "{} | {} Hz | {} channel{} -> mono | {} frames | {:.1} ms visible",
        state.file.path.display(),
        state.file.sample_rate,
        state.file.channels,
        if state.file.channels == 1 { "" } else { "s" },
        state.file.frames,
        state.viewport.visible_seconds(state.file.sample_rate) * 1000.0,
    );

    ui::column([
        ui::text("Waveform").height(28.0).fill_width(),
        ui::text(title).height(24.0).fill_width().truncate(),
        waveform_viewport(
            Arc::clone(&state.file),
            state.viewport,
            (!state.playing).then_some(state.zoom_anchor_ratio),
        )
        .id(WAVEFORM_WIDGET_ID)
        .size(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32)
        .fill_width()
        .height(WAVEFORM_HEIGHT as f32),
        waveform_scrollbar(state),
        waveform_controls(),
        ui::spacer().fill(),
    ])
    .padding(16.0)
    .spacing(10.0)
    .fill()
}

fn paint_playhead_overlay(
    state: &WaveformApp,
    plan: &SurfacePaintPlan,
    animation_time: Duration,
    primitives: &mut Vec<PaintPrimitive>,
) {
    let Some(bounds) = plan.first_widget_rect(WAVEFORM_WIDGET_ID) else {
        return;
    };
    let ratio = (state.playhead_ratio + animation_time.as_secs_f32() * 0.18).fract();
    let x = bounds.min.x + bounds.width() * ratio;
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: WAVEFORM_WIDGET_ID,
        rect: Rect::from_min_max(
            Point::new(x - 1.0, bounds.min.y),
            Point::new(x + 1.0, bounds.max.y),
        ),
        color: Rgba8 {
            r: 255,
            g: 232,
            b: 180,
            a: 245,
        },
    }));
}

fn waveform_scrollbar(state: &WaveformApp) -> ui::View<WaveformInteraction> {
    if !state.viewport.is_zoomed_in(state.file.frames) {
        return ui::spacer().fill_width().height(14.0);
    }

    let mut scrollbar = ScrollbarWidget::new(
        0,
        ScrollbarAxis::Horizontal,
        WidgetSizing::fixed(Vector2::new(WAVEFORM_WIDTH as f32, 14.0)),
    );
    scrollbar.props.viewport_fraction = state.viewport.visible_fraction(state.file.frames);
    scrollbar.state.offset_fraction = state.viewport.offset_fraction(state.file.frames);
    ui::custom_widget(scrollbar, |output| {
        output
            .typed_ref::<ScrollbarMessage>()
            .copied()
            .map(|message| match message {
                ScrollbarMessage::OffsetChanged { offset_fraction } => {
                    WaveformInteraction::ScrollTo { offset_fraction }
                }
            })
    })
    .fill_width()
    .height(14.0)
}

fn waveform_controls() -> ui::View<WaveformInteraction> {
    ui::row([
        ui::button("Zoom -")
            .subtle()
            .message(WaveformInteraction::Zoom { factor: 2.0 }),
        ui::button("Zoom +")
            .primary()
            .message(WaveformInteraction::Zoom { factor: 0.5 }),
        ui::button("Pan <")
            .subtle()
            .message(WaveformInteraction::Pan {
                visible_fraction: -0.25,
            }),
        ui::button("Pan >")
            .subtle()
            .message(WaveformInteraction::Pan {
                visible_fraction: 0.25,
            }),
        ui::button("Play")
            .subtle()
            .message(WaveformInteraction::TogglePlayback),
        ui::button("Reset")
            .subtle()
            .message(WaveformInteraction::Reset),
        ui::spacer().fill(),
    ])
    .spacing(8.0)
    .fill_width()
    .height(40.0)
}

#[cfg(test)]
#[path = "waveform_view/tests.rs"]
mod tests;
