use super::{Point, Rect};

impl Rect {
    /// Return a strip along the top edge, clamped to this rectangle.
    pub fn top_edge_strip(self, thickness: f32) -> Self {
        let thickness = thickness.max(0.0).min(self.height().max(0.0));
        Self::from_min_max(self.min, Point::new(self.max.x, self.min.y + thickness))
    }

    /// Return a strip along the bottom edge, clamped to this rectangle.
    pub fn bottom_edge_strip(self, thickness: f32) -> Self {
        let thickness = thickness.max(0.0).min(self.height().max(0.0));
        Self::from_min_max(Point::new(self.min.x, self.max.y - thickness), self.max)
    }

    /// Return a strip along the left edge, clamped to this rectangle.
    pub fn left_edge_strip(self, thickness: f32) -> Self {
        let thickness = thickness.max(0.0).min(self.width().max(0.0));
        Self::from_min_max(self.min, Point::new(self.min.x + thickness, self.max.y))
    }

    /// Return a strip along the right edge, clamped to this rectangle.
    pub fn right_edge_strip(self, thickness: f32) -> Self {
        let thickness = thickness.max(0.0).min(self.width().max(0.0));
        Self::from_min_max(Point::new(self.max.x - thickness, self.min.y), self.max)
    }
}
