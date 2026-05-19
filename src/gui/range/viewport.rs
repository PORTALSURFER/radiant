use super::interval::micros_matches_projected_nanos;
use crate::gui::types::Rect;

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

/// Named precision bounds for constructing a normalized viewport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NormalizedViewportParts {
    /// Visible normalized start in micro-units (`0..=1_000_000`).
    pub start_micros: u32,
    /// Visible normalized end in micro-units (`0..=1_000_000`).
    pub end_micros: u32,
    /// Optional start in normalized nanounits (`0..=1_000_000_000`).
    pub start_nanos: Option<u32>,
    /// Optional end in normalized nanounits (`0..=1_000_000_000`).
    pub end_nanos: Option<u32>,
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
    pub fn from_parts(parts: NormalizedViewportParts) -> Self {
        parts
            .start_nanos
            .zip(parts.end_nanos)
            .and_then(|(start_nanos, end_nanos)| {
                Self::from_projected_nanos(
                    parts.start_micros,
                    parts.end_micros,
                    start_nanos,
                    end_nanos,
                )
            })
            .unwrap_or_else(|| Self::from_micros(parts.start_micros, parts.end_micros))
    }

    /// Build a viewport from micro bounds, preferring nano bounds when they are
    /// present and consistent with the micro projections.
    pub fn from_bounds(
        start_micros: u32,
        end_micros: u32,
        start_nanos: Option<u32>,
        end_nanos: Option<u32>,
    ) -> Self {
        Self::from_parts(NormalizedViewportParts {
            start_micros,
            end_micros,
            start_nanos,
            end_nanos,
        })
    }

    /// Return the local `0.0..=1.0` ratio for one absolute normalized ratio.
    pub fn local_ratio(self, absolute_ratio: f64) -> f32 {
        if !absolute_ratio.is_finite() || self.width_ratio <= f64::EPSILON {
            return 0.0;
        }
        ((absolute_ratio.clamp(0.0, 1.0) - self.start_ratio) / self.width_ratio).clamp(0.0, 1.0)
            as f32
    }

    /// Project one absolute normalized ratio into an x coordinate inside `rect`.
    pub fn x_for_ratio(self, rect: Rect, absolute_ratio: f64, snap: NormalizedPixelSnap) -> f32 {
        let Some((min_x, max_x)) = finite_ordered_x_bounds(rect) else {
            return 0.0;
        };
        if max_x <= min_x {
            return min_x;
        }
        let raw_x = rect.min.x + (rect.width() * self.local_ratio(absolute_ratio));
        match snap {
            NormalizedPixelSnap::None => raw_x,
            NormalizedPixelSnap::Nearest => raw_x.round(),
        }
        .clamp(min_x, max_x)
    }

    /// Project one absolute micro position into an x coordinate inside `rect`.
    pub fn x_for_micros(self, rect: Rect, micros: u32, snap: NormalizedPixelSnap) -> f32 {
        self.x_for_ratio(rect, f64::from(micros.min(1_000_000)) / 1_000_000.0, snap)
    }
}

fn finite_ordered_x_bounds(rect: Rect) -> Option<(f32, f32)> {
    if !rect.min.x.is_finite() || !rect.max.x.is_finite() {
        return None;
    }
    Some(if rect.min.x <= rect.max.x {
        (rect.min.x, rect.max.x)
    } else {
        (rect.max.x, rect.min.x)
    })
}

#[cfg(test)]
mod tests {
    use super::{NormalizedPixelSnap, NormalizedViewport, NormalizedViewportParts};
    use crate::gui::types::{Point, Rect};

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

    #[test]
    fn normalized_viewport_supports_named_parts_construction() {
        let viewport = NormalizedViewport::from_parts(NormalizedViewportParts {
            start_micros: 500_123,
            end_micros: 500_124,
            start_nanos: Some(500_123_000),
            end_nanos: Some(500_123_200),
        });

        assert_eq!(viewport.start_ratio, 0.500123);
        assert!((viewport.width_ratio - 0.0000002).abs() < f64::EPSILON);
    }
}
