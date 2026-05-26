use crate::gui::types::Rgba8;

/// A normalized color stop in a visualization color ramp.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorRampStop {
    /// Normalized stop position in the `0.0..=1.0` range.
    pub position: f32,
    /// Color sampled at this stop.
    pub color: Rgba8,
}

impl ColorRampStop {
    /// Create a color stop from a normalized `0.0..=1.0` position.
    pub const fn new(position: f32, color: Rgba8) -> Self {
        Self { position, color }
    }

    /// Create a color stop from an 8-bit normalized position.
    pub const fn byte(position: u8, color: Rgba8) -> Self {
        Self {
            position: position as f32 / 255.0,
            color,
        }
    }
}

/// A reusable normalized color ramp for dense visualization surfaces.
///
/// Stops are expected in ascending position order. Sampling clamps outside
/// `0.0..=1.0` and ignores invalid positions by treating them as zero.
#[derive(Clone, Copy, Debug)]
pub struct ColorRamp<'a> {
    stops: &'a [ColorRampStop],
}

impl<'a> ColorRamp<'a> {
    /// Create a color ramp from ascending normalized stops.
    pub const fn new(stops: &'a [ColorRampStop]) -> Self {
        Self { stops }
    }

    /// Return the color sampled at `position`.
    pub fn sample(self, position: f32) -> Rgba8 {
        let Some(first) = self.stops.first().copied() else {
            return Rgba8::default();
        };
        let position = normalized_position(position);
        if position <= normalized_position(first.position) {
            return first.color;
        }

        for window in self.stops.windows(2) {
            let from = window[0];
            let to = window[1];
            if position <= normalized_position(to.position) {
                return sample_segment(from, to, position);
            }
        }

        self.stops.last().map(|stop| stop.color).unwrap_or_default()
    }

    /// Return the stops backing this ramp.
    pub const fn stops(self) -> &'a [ColorRampStop] {
        self.stops
    }
}

fn normalized_position(position: f32) -> f32 {
    if position.is_finite() {
        position.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn sample_segment(from: ColorRampStop, to: ColorRampStop, position: f32) -> Rgba8 {
    let from_position = normalized_position(from.position);
    let to_position = normalized_position(to.position);
    if to_position <= from_position {
        return to.color;
    }
    let amount = (position - from_position) / (to_position - from_position);
    from.color.blend_toward(to.color, amount)
}
