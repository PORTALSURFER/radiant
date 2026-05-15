//! Normalized interval primitives for reusable UI models.

mod index_viewport;
mod viewport;

pub use index_viewport::IndexViewport;
pub use viewport::{NormalizedPixelSnap, NormalizedViewport};

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

impl NormalizedRange {
    /// Build a normalized range, clamping bounds to `0..=1000` and ordering them.
    pub fn new(start_milli: u16, end_milli: u16) -> Self {
        Self::from_micros(
            u32::from(start_milli.min(1000)) * 1000,
            u32::from(end_milli.min(1000)) * 1000,
        )
    }

    /// Build a normalized range from micro precision while preserving ordered milli mirrors.
    pub fn from_micros(start_micros: u32, end_micros: u32) -> Self {
        Self::from_nanos(
            start_micros.min(1_000_000).saturating_mul(1000),
            end_micros.min(1_000_000).saturating_mul(1000),
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
mod tests {
    use super::{NormalizedPixelSnap, NormalizedRange, NormalizedViewport};
    use crate::gui::types::{Point, Rect};

    #[test]
    fn normalized_range_orders_and_clamps_nano_bounds() {
        let range = NormalizedRange::from_nanos(1_200_000_000, 125_600_000);

        assert_eq!(range.start_nanos, 125_600_000);
        assert_eq!(range.end_nanos, 1_000_000_000);
        assert_eq!(range.start_micros, 125_600);
        assert_eq!(range.end_micros, 1_000_000);
        assert_eq!(range.start_milli, 126);
        assert_eq!(range.end_milli, 1000);
    }

    #[test]
    fn normalized_range_from_milli_preserves_mirror_fields() {
        let range = NormalizedRange::new(800, 200);

        assert_eq!(range.start_milli, 200);
        assert_eq!(range.end_milli, 800);
        assert_eq!(range.start_micros, 200_000);
        assert_eq!(range.end_micros, 800_000);
        assert_eq!(range.start_nanos, 200_000_000);
        assert_eq!(range.end_nanos, 800_000_000);
    }

    #[test]
    fn normalized_viewport_projects_absolute_ratios_into_rect() {
        let rect = Rect::from_min_max(Point::new(10.0, 0.0), Point::new(110.0, 20.0));
        let viewport = NormalizedViewport::from_micros(250_000, 750_000);

        assert_eq!(
            viewport.x_for_ratio(rect, 0.25, NormalizedPixelSnap::Nearest),
            10.0
        );
        assert_eq!(
            viewport.x_for_ratio(rect, 0.5, NormalizedPixelSnap::Nearest),
            60.0
        );
        assert_eq!(
            viewport.x_for_ratio(rect, 0.75, NormalizedPixelSnap::Nearest),
            110.0
        );
    }

    #[test]
    fn normalized_viewport_projection_sanitizes_invalid_inputs() {
        let rect = Rect::from_min_max(Point::new(10.0, 0.0), Point::new(110.0, 20.0));
        let viewport = NormalizedViewport::from_micros(250_000, 750_000);

        assert_eq!(
            viewport.x_for_ratio(rect, f64::NAN, NormalizedPixelSnap::Nearest),
            10.0
        );
        assert_eq!(
            viewport.x_for_ratio(
                Rect::from_min_max(Point::new(f32::NAN, 0.0), Point::new(110.0, 20.0)),
                0.5,
                NormalizedPixelSnap::Nearest
            ),
            0.0
        );
        assert_eq!(
            viewport.x_for_ratio(
                Rect::from_min_max(Point::new(10.0, 0.0), Point::new(f32::INFINITY, 20.0)),
                0.5,
                NormalizedPixelSnap::Nearest
            ),
            0.0
        );
    }

    #[test]
    fn normalized_viewport_uses_nanos_only_when_they_match_micro_mirrors() {
        let viewport =
            NormalizedViewport::from_bounds(500_123, 500_124, Some(500_123_000), Some(500_123_200));

        assert_eq!(viewport.start_ratio, 0.500123);
        assert!((viewport.width_ratio - 0.0000002).abs() < f64::EPSILON);

        let fallback =
            NormalizedViewport::from_bounds(500_123, 500_124, Some(400_000_000), Some(400_100_000));

        assert_eq!(fallback, NormalizedViewport::from_micros(500_123, 500_124));
    }
}
