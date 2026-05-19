use crate::gui::range::NormalizedViewport;

/// Explicit normalized bounds used to build a timeline viewport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimelineViewportParts {
    /// Visible viewport start in normalized milli-units.
    pub start_milli: u16,
    /// Visible viewport end in normalized milli-units.
    pub end_milli: u16,
    /// Visible viewport start in normalized micro-units.
    pub start_micros: u32,
    /// Visible viewport end in normalized micro-units.
    pub end_micros: u32,
    /// Visible viewport start in normalized nanounits.
    pub start_nanos: u32,
    /// Visible viewport end in normalized nanounits.
    pub end_nanos: u32,
}

impl Default for TimelineViewportParts {
    fn default() -> Self {
        Self {
            start_milli: 0,
            end_milli: 1000,
            start_micros: 0,
            end_micros: 1_000_000,
            start_nanos: 0,
            end_nanos: 1_000_000_000,
        }
    }
}

/// Visible normalized viewport for a timeline or signal visualization.
///
/// The same range is kept at milli, micro, and nano precision so hosts can
/// use coarse labels and deep-zoom pointer mapping without recomputing bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimelineViewport {
    /// Visible viewport start in normalized milli-units.
    pub start_milli: u16,
    /// Visible viewport end in normalized milli-units.
    pub end_milli: u16,
    /// Visible viewport start in normalized micro-units.
    pub start_micros: u32,
    /// Visible viewport end in normalized micro-units.
    pub end_micros: u32,
    /// Visible viewport start in normalized nanounits.
    pub start_nanos: u32,
    /// Visible viewport end in normalized nanounits.
    pub end_nanos: u32,
}

impl TimelineViewport {
    /// Build a timeline viewport from named normalized bounds.
    pub fn from_parts(parts: TimelineViewportParts) -> Self {
        Self {
            start_milli: parts.start_milli,
            end_milli: parts.end_milli,
            start_micros: parts.start_micros,
            end_micros: parts.end_micros,
            start_nanos: parts.start_nanos,
            end_nanos: parts.end_nanos,
        }
    }

    /// Build a timeline viewport from explicit normalized bounds.
    pub fn new(
        start_milli: u16,
        end_milli: u16,
        start_micros: u32,
        end_micros: u32,
        start_nanos: u32,
        end_nanos: u32,
    ) -> Self {
        Self::from_parts(TimelineViewportParts {
            start_milli,
            end_milli,
            start_micros,
            end_micros,
            start_nanos,
            end_nanos,
        })
    }

    /// Return this viewport as a generic normalized viewport projector.
    pub fn normalized_viewport(self) -> NormalizedViewport {
        NormalizedViewport::from_bounds(
            self.start_micros,
            self.end_micros,
            Some(self.start_nanos),
            Some(self.end_nanos),
        )
    }
}

impl Default for TimelineViewport {
    fn default() -> Self {
        Self::from_parts(TimelineViewportParts::default())
    }
}
