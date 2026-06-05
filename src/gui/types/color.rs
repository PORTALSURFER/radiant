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

    /// Return this color with one of two alpha channels based on `condition`.
    pub const fn with_alpha_if(self, condition: bool, true_alpha: u8, false_alpha: u8) -> Self {
        self.with_alpha(if condition { true_alpha } else { false_alpha })
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

    /// Linearly blend this color's RGB channels toward another color after
    /// resolving both colors as fully opaque.
    pub fn blend_opaque_toward(self, other: Self, amount: f32) -> Self {
        self.with_alpha(255)
            .blend_toward(other.with_alpha(255), amount)
    }
}

fn blend_channel(from: u8, to: u8, amount: f32) -> u8 {
    ((from as f32) + (((to as f32) - (from as f32)) * amount)).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_opaque_toward_ignores_source_alpha() {
        let from = Rgba8::new(10, 20, 30, 40);
        let to = Rgba8::new(110, 220, 130, 120);

        assert_eq!(
            from.blend_opaque_toward(to, 0.5),
            Rgba8::new(60, 120, 80, 255)
        );
    }

    #[test]
    fn with_alpha_if_selects_alpha_from_condition() {
        let color = Rgba8::new(10, 20, 30, 40);

        assert_eq!(
            color.with_alpha_if(true, 200, 80),
            Rgba8::new(10, 20, 30, 200)
        );
        assert_eq!(
            color.with_alpha_if(false, 200, 80),
            Rgba8::new(10, 20, 30, 80)
        );
    }
}
