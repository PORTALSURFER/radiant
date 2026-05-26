use radiant::prelude::{ColorRamp, ColorRampStop, Rgba8};

const SPECTROGRAM_STOPS: [ColorRampStop; 5] = [
    ColorRampStop::new(0.0, Rgba8::new(10, 18, 30, 255)),
    ColorRampStop::new(0.28, Rgba8::new(16, 74, 118, 255)),
    ColorRampStop::new(0.58, Rgba8::new(36, 168, 116, 255)),
    ColorRampStop::new(0.84, Rgba8::new(246, 176, 64, 255)),
    ColorRampStop::new(1.0, Rgba8::new(255, 240, 184, 255)),
];

pub(super) fn spectrogram_color(energy: f32) -> Rgba8 {
    ColorRamp::new(&SPECTROGRAM_STOPS).sample(energy)
}

pub(super) fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

pub(super) fn translucent(mut color: Rgba8, alpha: u8) -> Rgba8 {
    color.a = alpha;
    color
}
