use crate::gui::types::{Point, Rect};

/// Named fields for constructing reusable timeline lane geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineLaneLayoutParts {
    /// Rect that contains all lanes.
    pub rect: Rect,
    /// Number of lanes in the layout.
    pub lane_count: usize,
    /// Optional explicit lane height in logical pixels.
    pub lane_height: Option<f32>,
}

impl TimelineLaneLayoutParts {
    /// Build lane layout parts that divide the rect evenly by lane count.
    pub const fn even(rect: Rect, lane_count: usize) -> Self {
        Self {
            rect,
            lane_count,
            lane_height: None,
        }
    }

    /// Build lane layout parts with an explicit lane height.
    pub const fn fixed_height(rect: Rect, lane_count: usize, lane_height: f32) -> Self {
        Self {
            rect,
            lane_count,
            lane_height: Some(lane_height),
        }
    }
}

/// Reusable vertical lane geometry for timelines, arrangements, and editors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineLaneLayout {
    /// Rect that contains all lanes.
    pub rect: Rect,
    /// Number of lanes in the layout.
    pub lane_count: usize,
    /// Optional explicit lane height in logical pixels.
    pub lane_height: Option<f32>,
}

impl TimelineLaneLayout {
    /// Build a lane layout from named parts.
    pub const fn from_parts(parts: TimelineLaneLayoutParts) -> Self {
        Self {
            rect: parts.rect,
            lane_count: parts.lane_count,
            lane_height: parts.lane_height,
        }
    }

    /// Build a layout that divides the rect evenly by lane count.
    pub const fn even(rect: Rect, lane_count: usize) -> Self {
        Self::from_parts(TimelineLaneLayoutParts::even(rect, lane_count))
    }

    /// Build a layout with an explicit lane height.
    pub const fn fixed_height(rect: Rect, lane_count: usize, lane_height: f32) -> Self {
        Self::from_parts(TimelineLaneLayoutParts::fixed_height(
            rect,
            lane_count,
            lane_height,
        ))
    }

    /// Return the resolved lane height.
    pub fn lane_height(self) -> f32 {
        self.lane_height
            .filter(|height| height.is_finite() && *height > f32::EPSILON)
            .unwrap_or_else(|| self.rect.height() / self.lane_count.max(1) as f32)
    }

    /// Return the rect for a lane index, clamped inside the layout rect.
    pub fn lane_rect(self, lane: usize) -> Rect {
        let lane = lane.min(self.lane_count.saturating_sub(1));
        let height = self.lane_height();
        let y = self.rect.min.y + lane as f32 * height;
        Rect::from_min_max(
            Point::new(self.rect.min.x, y),
            Point::new(self.rect.max.x, (y + height).min(self.rect.max.y)),
        )
    }

    /// Return a label-gutter rect aligned with a lane's vertical span.
    pub fn lane_label_rect(self, label_bounds: Rect, lane: usize) -> Rect {
        let lane_rect = self.lane_rect(lane);
        Rect::from_min_max(
            Point::new(label_bounds.min.x, lane_rect.min.y),
            Point::new(label_bounds.max.x, lane_rect.max.y),
        )
        .clamp_to(label_bounds)
    }

    /// Return the lane at a pointer position.
    pub fn lane_at(self, position: Point) -> Option<usize> {
        if self.lane_count == 0 || !self.rect.contains(position) {
            return None;
        }
        let height = self.lane_height().max(1.0);
        Some(((position.y - self.rect.min.y) / height).floor() as usize)
            .map(|lane| lane.min(self.lane_count - 1))
    }
}
