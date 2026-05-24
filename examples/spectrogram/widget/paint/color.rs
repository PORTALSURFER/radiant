use radiant::prelude::Rgba8;

pub(super) fn spectrogram_color(energy: f32) -> Rgba8 {
    let value = energy.clamp(0.0, 1.0);
    let cold = rgba(10, 18, 30, 255);
    let blue = rgba(16, 74, 118, 255);
    let green = rgba(36, 168, 116, 255);
    let amber = rgba(246, 176, 64, 255);
    let hot = rgba(255, 240, 184, 255);

    if value < 0.28 {
        lerp_color(cold, blue, value / 0.28)
    } else if value < 0.58 {
        lerp_color(blue, green, (value - 0.28) / 0.30)
    } else if value < 0.84 {
        lerp_color(green, amber, (value - 0.58) / 0.26)
    } else {
        lerp_color(amber, hot, (value - 0.84) / 0.16)
    }
}

pub(super) fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

pub(super) fn translucent(mut color: Rgba8, alpha: u8) -> Rgba8 {
    color.a = alpha;
    color
}

fn lerp_color(a: Rgba8, b: Rgba8, t: f32) -> Rgba8 {
    let t = t.clamp(0.0, 1.0);
    rgba(
        lerp_channel(a.r, b.r, t),
        lerp_channel(a.g, b.g, t),
        lerp_channel(a.b, b.b, t),
        255,
    )
}

fn lerp_channel(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}
