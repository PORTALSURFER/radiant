//! Frame-driven animation through the application builder.

use radiant::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AnimationMessage {
    Toggle,
    Frame,
    Reset,
}

#[derive(Clone, Debug)]
struct AnimationState {
    running: bool,
    frame: u64,
    phase: f32,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            running: true,
            frame: 0,
            phase: 0.0,
        }
    }
}

impl AnimationState {
    fn status(&self) -> String {
        if self.running {
            format!("Running | frame {} | phase {:.2}", self.frame, self.phase)
        } else {
            format!("Paused | frame {} | phase {:.2}", self.frame, self.phase)
        }
    }

    fn tick(&mut self) {
        if self.running {
            self.frame = self.frame.saturating_add(1);
            self.phase = ((self.frame % 120) as f32) / 119.0;
        }
    }
}

fn main() -> radiant::Result {
    radiant::app(AnimationState::default())
        .title("Radiant Animation Showcase")
        .size(520, 220)
        .min_size(420, 180)
        .view(animation_view)
        .animation(|state| state.running)
        .on_frame(|| AnimationMessage::Frame)
        .update(|state, message| match message {
            AnimationMessage::Toggle => state.running = !state.running,
            AnimationMessage::Frame => state.tick(),
            AnimationMessage::Reset => {
                state.frame = 0;
                state.phase = 0.0;
            }
        })
        .run()
}

fn animation_view(state: &mut AnimationState) -> View<AnimationMessage> {
    column([
        text("Animation Showcase").height(28.0).fill_width(),
        text(state.status()).height(26.0).fill_width(),
        phase_meter(state.phase),
        row([
            button(if state.running { "Pause" } else { "Run" })
                .primary()
                .message(AnimationMessage::Toggle)
                .width(100.0)
                .height(32.0),
            button("Reset")
                .subtle()
                .message(AnimationMessage::Reset)
                .width(100.0)
                .height(32.0),
        ])
        .spacing(10.0)
        .fill_width(),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(10.0)
    .fill_width()
    .fill_height()
}

fn phase_meter(phase: f32) -> View<AnimationMessage> {
    let fill_width = 32.0 + phase.clamp(0.0, 1.0) * 300.0;
    row([
        spacer()
            .primary()
            .width(fill_width)
            .height(18.0)
            .key("phase-fill"),
        spacer()
            .subtle()
            .fill_width()
            .height(18.0)
            .key("phase-rest"),
    ])
    .style(WidgetStyle::default())
    .height(24.0)
    .fill_width()
}
