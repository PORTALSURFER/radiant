use super::interval::micros_matches_projected_nanos;

mod projection;

#[cfg(test)]
#[path = "viewport/tests.rs"]
mod tests;

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
        projection::local_ratio(self, absolute_ratio)
    }

    /// Project one absolute normalized ratio into an x coordinate inside `rect`.
    pub fn x_for_ratio(
        self,
        rect: crate::gui::types::Rect,
        absolute_ratio: f64,
        snap: NormalizedPixelSnap,
    ) -> f32 {
        projection::x_for_ratio(self, rect, absolute_ratio, snap)
    }

    /// Project one absolute micro position into an x coordinate inside `rect`.
    pub fn x_for_micros(
        self,
        rect: crate::gui::types::Rect,
        micros: u32,
        snap: NormalizedPixelSnap,
    ) -> f32 {
        self.x_for_ratio(rect, f64::from(micros.min(1_000_000)) / 1_000_000.0, snap)
    }
}
