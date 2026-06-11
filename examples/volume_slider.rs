//! Focused horizontal volume slider example.

use radiant::prelude::*;

#[derive(Clone, Debug, PartialEq)]
enum VolumeMessage {
    SetVolume(f32),
    SetMuted(bool),
}

struct VolumeState {
    volume: f32,
    muted: bool,
}

impl Default for VolumeState {
    fn default() -> Self {
        Self {
            volume: 0.72,
            muted: false,
        }
    }
}

fn main() -> radiant::Result {
    radiant::app(VolumeState::default())
        .title("Radiant Volume Slider")
        .size(420, 170)
        .min_size(320, 140)
        .view(|state| {
            let audible_volume = if state.muted { 0.0 } else { state.volume };
            column([
                text("Output volume").height(26.0).fill_width(),
                row([
                    text(format!("{:>3}%", (audible_volume * 100.0).round() as u32))
                        .size(58.0, 28.0)
                        .align_text(TextAlign::Right),
                    slider(audible_volume)
                        .primary()
                        .message(VolumeMessage::SetVolume)
                        .fill_width(),
                ])
                .spacing(10.0)
                .fill_width(),
                row([
                    checkbox(state.muted).message(VolumeMessage::SetMuted),
                    text(if state.muted { "Muted" } else { "Live output" })
                        .height(28.0)
                        .fill_width(),
                ])
                .spacing(8.0)
                .fill_width(),
            ])
            .padding(18.0)
            .spacing(12.0)
            .fill()
        })
        .update(update)
        .run()
}

fn update(state: &mut VolumeState, message: VolumeMessage) {
    match message {
        VolumeMessage::SetVolume(volume) => {
            state.volume = volume;
            state.muted = false;
        }
        VolumeMessage::SetMuted(muted) => state.muted = muted,
    }
}
