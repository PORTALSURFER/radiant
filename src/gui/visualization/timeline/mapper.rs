use crate::gui::{
    range::{NormalizedPixelSnap, NormalizedRange},
    types::Rect,
};

use super::TimelineViewport;

/// Mapper between normalized timeline coordinates and local canvas pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineCoordinateMapper {
    /// Normalized timeline viewport.
    pub viewport: TimelineViewport,
    /// Local canvas rect used for projection.
    pub rect: Rect,
    /// Pixel snapping policy.
    pub snap: NormalizedPixelSnap,
}

impl TimelineCoordinateMapper {
    /// Build a mapper for one timeline viewport and canvas rect.
    pub fn new(viewport: TimelineViewport, rect: Rect, snap: NormalizedPixelSnap) -> Self {
        Self {
            viewport,
            rect,
            snap,
        }
    }

    /// Project one normalized micro position into local x coordinates.
    pub fn x_for_micros(self, micros: u32) -> f32 {
        self.viewport
            .normalized_viewport()
            .x_for_micros(self.rect, micros, self.snap)
    }

    /// Project one normalized range into local x bounds.
    pub fn x_range_for(self, range: NormalizedRange) -> (f32, f32) {
        (
            self.x_for_micros(range.start_micros),
            self.x_for_micros(range.end_micros),
        )
    }

    /// Convert a local x coordinate back into normalized micro units.
    pub fn micros_for_x(self, x: f32) -> u32 {
        if self.rect.width() <= f32::EPSILON {
            return self.viewport.start_micros.min(1_000_000);
        }
        let local_ratio = ((x - self.rect.min.x) / self.rect.width()).clamp(0.0, 1.0) as f64;
        let viewport = self.viewport.normalized_viewport();
        ((viewport.start_ratio + (local_ratio * viewport.width_ratio)).clamp(0.0, 1.0)
            * 1_000_000.0)
            .round() as u32
    }
}
