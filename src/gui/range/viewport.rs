use super::micros_matches_projected_nanos;
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
