//! Backend-neutral color types.

/// RGBA color in 8-bit per channel sRGB space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgba8 {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel.
    pub a: u8,
}

impl Rgba8 {
    /// Create an sRGB color from 8-bit red, green, blue, and alpha channels.
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Return this color with a replaced alpha channel.
    pub const fn with_alpha(self, alpha: u8) -> Self {
        Self { a: alpha, ..self }
    }

    /// Linearly blend this color toward another color.
    pub fn blend_toward(self, other: Self, amount: f32) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        Self {
            r: blend_channel(self.r, other.r, amount),
            g: blend_channel(self.g, other.g, amount),
            b: blend_channel(self.b, other.b, amount),
            a: blend_channel(self.a, other.a, amount),
        }
    }
}

fn blend_channel(from: u8, to: u8, amount: f32) -> u8 {
    ((from as f32) + (((to as f32) - (from as f32)) * amount)).round() as u8
}
