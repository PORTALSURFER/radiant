//! Normalized interval primitives for reusable UI models.

use super::types::Rect;

/// Pixel-snapping policy for normalized range coordinates projected into a rect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalizedPixelSnap {
    /// Keep the projected coordinate as-is.
    None,
    /// Snap the projected coordinate to the nearest device pixel.
    Nearest,
}

/// Visible normalized viewport used to project absolute normalized anchors into
/// local surface coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizedViewport {
    /// Normalized start ratio in `0.0..=1.0`.
    pub start_ratio: f64,
    /// Normalized visible width ratio.
    pub width_ratio: f64,
}

impl NormalizedViewport {
    /// Build a viewport from micro precision bounds.
    pub fn from_micros(start_micros: u32, end_micros: u32) -> Self {
        let start_micros = start_micros.min(1_000_000);
        let end_micros = end_micros.min(1_000_000).max(start_micros);
        let start_ratio = f64::from(start_micros) / 1_000_000.0;
        let width_ratio =
            (f64::from(end_micros.saturating_sub(start_micros)) / 1_000_000.0).max(f64::EPSILON);
        Self {
            start_ratio,
            width_ratio,
        }
    }

    /// Build a viewport from nano precision bounds when the provided micro
    /// mirrors match the nano projections.
    pub fn from_projected_nanos(
        start_micros: u32,
        end_micros: u32,
        start_nanos: u32,
        end_nanos: u32,
    ) -> Option<Self> {
        let start_nanos = start_nanos.min(1_000_000_000);
        let end_nanos = end_nanos.min(1_000_000_000).max(start_nanos);
        if !micros_matches_projected_nanos(start_micros, start_nanos)
            || !micros_matches_projected_nanos(end_micros, end_nanos)
        {
            return None;
        }
        Some(Self {
            start_ratio: f64::from(start_nanos) / 1_000_000_000.0,
            width_ratio: (f64::from(end_nanos.saturating_sub(start_nanos)) / 1_000_000_000.0)
                .max(f64::EPSILON),
        })
    }

    /// Build a viewport from micro bounds, preferring nano bounds when they are
    /// present and consistent with the micro projections.
    pub fn from_bounds(
        start_micros: u32,
        end_micros: u32,
        start_nanos: Option<u32>,
        end_nanos: Option<u32>,
    ) -> Self {
        start_nanos
            .zip(end_nanos)
            .and_then(|(start_nanos, end_nanos)| {
                Self::from_projected_nanos(start_micros, end_micros, start_nanos, end_nanos)
            })
            .unwrap_or_else(|| Self::from_micros(start_micros, end_micros))
    }

    /// Return the local `0.0..=1.0` ratio for one absolute normalized ratio.
    pub fn local_ratio(self, absolute_ratio: f64) -> f32 {
        if self.width_ratio <= f64::EPSILON {
            return 0.0;
        }
        ((absolute_ratio.clamp(0.0, 1.0) - self.start_ratio) / self.width_ratio).clamp(0.0, 1.0)
            as f32
    }

    /// Project one absolute normalized ratio into an x coordinate inside `rect`.
    pub fn x_for_ratio(self, rect: Rect, absolute_ratio: f64, snap: NormalizedPixelSnap) -> f32 {
        let raw_x = rect.min.x + (rect.width() * self.local_ratio(absolute_ratio));
        match snap {
            NormalizedPixelSnap::None => raw_x,
            NormalizedPixelSnap::Nearest => raw_x.round(),
        }
        .clamp(rect.min.x, rect.max.x)
    }

    /// Project one absolute micro position into an x coordinate inside `rect`.
    pub fn x_for_micros(self, rect: Rect, micros: u32, snap: NormalizedPixelSnap) -> f32 {
        self.x_for_ratio(rect, f64::from(micros.min(1_000_000)) / 1_000_000.0, snap)
    }
}

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

fn micros_matches_projected_nanos(value_micros: u32, value_nanos: u32) -> bool {
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
