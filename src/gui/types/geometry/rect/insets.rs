use super::{Point, Rect};

impl Rect {
    /// Return this rectangle with horizontal insets applied.
    pub fn inset_horizontal(self, left: f32, right: f32) -> Self {
        let min_x = (self.min.x + left.max(0.0)).min(self.max.x);
        let max_x = (self.max.x - right.max(0.0)).max(min_x);
        Self::from_min_max(
            Point::new(min_x, self.min.y),
            Point::new(max_x, self.max.y.max(self.min.y)),
        )
    }

    /// Return this rectangle with vertical insets applied.
    pub fn inset_vertical(self, top: f32, bottom: f32) -> Self {
        let min_y = (self.min.y + top.max(0.0)).min(self.max.y);
        let max_y = (self.max.y - bottom.max(0.0)).max(min_y);
        Self::from_min_max(
            Point::new(self.min.x, min_y),
            Point::new(self.max.x.max(self.min.x), max_y),
        )
    }

    /// Split this rectangle into upper and lower rectangles at `y`.
    pub fn split_at_y(self, y: f32) -> (Self, Self) {
        let y = y.max(self.min.y).min(self.max.y);
        (
            Self::from_min_max(self.min, Point::new(self.max.x, y)),
            Self::from_min_max(Point::new(self.min.x, y), self.max),
        )
    }

    /// Return this rectangle with a symmetric horizontal inset capped at half width.
    pub fn inset_horizontal_saturating(self, inset: f32) -> Self {
        let inset = inset.max(0.0).min((self.width() * 0.5).max(0.0));
        Self::from_min_max(
            Point::new(self.min.x + inset, self.min.y),
            Point::new(self.max.x - inset, self.max.y),
        )
    }

    /// Return this rectangle with a symmetric inset capped at half width and height.
    pub fn inset_uniform_saturating(self, inset: f32) -> Self {
        let inset_x = inset.max(0.0).min((self.width() * 0.5).max(0.0));
        let inset_y = inset.max(0.0).min((self.height() * 0.5).max(0.0));
        Self::from_min_max(
            Point::new(self.min.x + inset_x, self.min.y + inset_y),
            Point::new(self.max.x - inset_x, self.max.y - inset_y),
        )
    }
}
