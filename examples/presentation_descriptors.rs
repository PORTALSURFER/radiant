//! Typed presentation descriptors for frame clocks and paint-only overlays.

use radiant::prelude as ui;
use ui::{
    PaintFillRect, PaintPrimitive, Point, Rect, Rgba8, TransientOverlayContext, Vector2, View,
};

#[derive(Clone)]
enum Message {
    Toggle,
    Frame,
}

#[derive(Default)]
struct State {
    running: bool,
    frame: u32,
}

fn main() -> radiant::Result {
    ui::app(State::default())
        .title("Radiant Presentation Descriptors")
        .size(320, 160)
        .view(view)
        .presentation(
            ui::presentation()
                .frame_clock(
                    ui::FrameClock::message(Message::Frame)
                        .when(|state: &mut State| state.running)
                        .fps(60)
                        .repaint_scope(
                            |state: &mut State| state.frame,
                            |state: &mut State, before| state.frame == before + 1,
                        ),
                )
                .transient_overlay(
                    ui::TransientOverlay::new(1_u64)
                        .paint_only()
                        .when(|state: &mut State| state.running)
                        .fps(60)
                        .paint(paint_playhead),
                ),
        )
        .handle_message(update)
        .run()
}

fn view(state: &State) -> View<Message> {
    ui::column([
        ui::text(format!("Frame {}", state.frame))
            .height(24.0)
            .fill_width(),
        ui::button(if state.running { "Pause" } else { "Play" })
            .message(Message::Toggle)
            .height(30.0),
    ])
    .spacing(8.0)
    .padding(12.0)
}

fn update(state: &mut State, message: Message, context: &mut ui::UiUpdateContext<Message>) {
    match message {
        Message::Toggle => {
            state.running = !state.running;
            context.request_repaint();
        }
        Message::Frame => {
            state.frame = state.frame.wrapping_add(1);
        }
    }
}

fn paint_playhead(
    state: &mut State,
    _context: TransientOverlayContext<'_>,
    primitives: &mut Vec<PaintPrimitive>,
) {
    let x = 12.0 + (state.frame % 280) as f32;
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: 0,
        rect: Rect::from_min_size(Point::new(x, 12.0), Vector2::new(2.0, 92.0)),
        color: Rgba8::new(255, 96, 64, 220),
    }));
}
