/// Normalized range with deterministic milli, micro, and nano projections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NormalizedRange {
    /// Start position in normalized milli-units.
    pub start_milli: u16,
    /// End position in normalized milli-units.
    pub end_milli: u16,
    /// Start position in normalized micro-units (`0..=1_000_000`).
    pub start_micros: u32,
    /// End position in normalized micro-units (`0..=1_000_000`).
    pub end_micros: u32,
    /// Start position in normalized nanounits (`0..=1_000_000_000`).
    pub start_nanos: u32,
    /// End position in normalized nanounits (`0..=1_000_000_000`).
    pub end_nanos: u32,
}

/// Named milli-unit bounds for constructing a normalized range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NormalizedRangeParts {
    /// Start position in normalized milli-units.
    pub start_milli: u16,
    /// End position in normalized milli-units.
    pub end_milli: u16,
}

impl NormalizedRange {
    /// Build a normalized range from named milli-unit bounds.
    pub fn from_parts(parts: NormalizedRangeParts) -> Self {
        Self::from_micros(
            u32::from(parts.start_milli.min(1000)) * 1000,
            u32::from(parts.end_milli.min(1000)) * 1000,
        )
    }

    /// Build a normalized range, clamping bounds to `0..=1000` and ordering them.
    pub fn new(start_milli: u16, end_milli: u16) -> Self {
        Self::from_parts(NormalizedRangeParts {
            start_milli,
            end_milli,
        })
    }

    /// Build a normalized range from micro precision while preserving ordered milli mirrors.
    pub fn from_micros(start_micros: u32, end_micros: u32) -> Self {
        Self::from_nanos(
            start_micros.min(1_000_000).saturating_mul(1000),
            end_micros.min(1_000_000).saturating_mul(1000),
        )
    }

    /// Build a normalized range from floating point fractions.
    ///
    /// Inputs are clamped to `0.0..=1.0`, non-finite values become `0.0`, and
    /// bounds are ordered in the returned range. The stored milli, micro, and
    /// nano projections are derived from the same clamped values so callers do
    /// not need local conversion helpers before using timeline or canvas APIs.
    pub fn from_fractions(start_fraction: f32, end_fraction: f32) -> Self {
        Self::from_nanos(
            normalized_fraction_to_nanos(start_fraction),
            normalized_fraction_to_nanos(end_fraction),
        )
    }

    /// Build a normalized range from nano precision while preserving ordered mirrors.
    pub fn from_nanos(start_nanos: u32, end_nanos: u32) -> Self {
        let start = start_nanos.min(1_000_000_000);
        let end = end_nanos.min(1_000_000_000);
        let ordered_start = start.min(end);
        let ordered_end = end.max(start);
        Self {
            start_milli: nanos_to_milli(ordered_start),
            end_milli: nanos_to_milli(ordered_end),
            start_micros: nanos_to_micros(ordered_start),
            end_micros: nanos_to_micros(ordered_end),
            start_nanos: ordered_start,
            end_nanos: ordered_end,
        }
    }

    /// Return the start as a floating point fraction.
    pub fn start_fraction(self) -> f32 {
        self.start_nanos as f32 / 1_000_000_000.0
    }

    /// Return the end as a floating point fraction.
    pub fn end_fraction(self) -> f32 {
        self.end_nanos as f32 / 1_000_000_000.0
    }

    /// Return the range width as a floating point fraction.
    pub fn width_fraction(self) -> f32 {
        (self.end_nanos - self.start_nanos) as f32 / 1_000_000_000.0
    }
}

/// Convert a floating point fraction into normalized milli-units.
pub fn normalized_fraction_to_milli(value: f32) -> u16 {
    ((normalized_fraction(value) * 1000.0).round() as u16).min(1000)
}

/// Convert a floating point fraction into normalized micro-units.
pub fn normalized_fraction_to_micros(value: f32) -> u32 {
    ((normalized_fraction(value) * 1_000_000.0).round() as u32).min(1_000_000)
}

/// Convert a floating point fraction into normalized nano-units.
pub fn normalized_fraction_to_nanos(value: f32) -> u32 {
    ((normalized_fraction(value) * 1_000_000_000.0).round() as u32).min(1_000_000_000)
}

fn normalized_fraction(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn micros_to_milli(value_micros: u32) -> u16 {
    ((value_micros.min(1_000_000) + 500) / 1000) as u16
}

fn nanos_to_micros(value_nanos: u32) -> u32 {
    ((value_nanos.min(1_000_000_000) + 500) / 1000).min(1_000_000)
}

fn nanos_to_milli(value_nanos: u32) -> u16 {
    micros_to_milli(nanos_to_micros(value_nanos))
}

pub(crate) fn micros_matches_projected_nanos(value_micros: u32, value_nanos: u32) -> bool {
    let projected_micros = nanos_to_micros(value_nanos);
    projected_micros.abs_diff(value_micros.min(1_000_000)) <= 1
}

#[cfg(test)]
#[path = "interval/tests.rs"]
mod tests;
