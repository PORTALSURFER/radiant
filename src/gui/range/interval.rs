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

/// Editable edge of a normalized range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NormalizedRangeEdge {
    /// Leading/start edge.
    Start,
    /// Trailing/end edge.
    End,
}

/// Thresholded drag state for creating a normalized range from one anchor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizedRangeDrag {
    /// Fixed anchor fraction where the drag started.
    pub anchor_fraction: f32,
    /// Current active drag fraction.
    pub current_fraction: f32,
    /// Whether the drag exceeded the supplied movement threshold.
    pub moved: bool,
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

    /// Return a copy with one edge moved to `fraction`.
    ///
    /// The opposite edge stays fixed. The returned range is clamped to
    /// `0.0..=1.0` and ordered, so dragging past the fixed edge naturally flips
    /// the resulting start/end order without leaking negative widths to hosts.
    pub fn with_edge_fraction(self, edge: NormalizedRangeEdge, fraction: f32) -> Self {
        match edge {
            NormalizedRangeEdge::Start => Self::from_fractions(fraction, self.end_fraction()),
            NormalizedRangeEdge::End => Self::from_fractions(self.start_fraction(), fraction),
        }
    }

    /// Return a copy shifted by `delta_fraction`, clamped to the normalized span.
    ///
    /// The range width is preserved where possible. Non-finite deltas leave the
    /// range unchanged.
    pub fn shifted_by_fraction(self, delta_fraction: f32) -> Self {
        if !delta_fraction.is_finite() {
            return self;
        }
        let width = self.width_fraction().clamp(0.0, 1.0);
        if width >= 1.0 {
            return Self::from_fractions(0.0, 1.0);
        }
        let start = (self.start_fraction() + delta_fraction).clamp(0.0, 1.0 - width);
        Self::from_fractions(start, start + width)
    }
}

impl NormalizedRangeDrag {
    /// Start a normalized range drag from one anchor fraction.
    pub fn new(anchor_fraction: f32) -> Self {
        let anchor_fraction = normalized_fraction(anchor_fraction);
        Self {
            anchor_fraction,
            current_fraction: anchor_fraction,
            moved: false,
        }
    }

    /// Update the active fraction and movement flag.
    pub fn update(&mut self, current_fraction: f32, move_threshold: f32) {
        self.current_fraction = normalized_fraction(current_fraction);
        self.moved |= (self.current_fraction - self.anchor_fraction).abs()
            > finite_non_negative(move_threshold);
    }

    /// Return the current ordered range between the anchor and active fraction.
    pub fn range(self) -> NormalizedRange {
        NormalizedRange::from_fractions(self.anchor_fraction, self.current_fraction)
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

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
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
