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

    /// Return a horizontal strip centered on this rectangle's vertical center.
    pub fn horizontal_center_strip(self, thickness: f32) -> Self {
        self.horizontal_strip_around_y(self.center().y, thickness)
    }

    /// Return a vertical strip centered on this rectangle's horizontal center.
    pub fn vertical_center_strip(self, thickness: f32) -> Self {
        self.vertical_strip_around_x(self.center().x, thickness)
    }

    /// Return a horizontal strip centered around `y`, shifted inside this rectangle.
    pub fn horizontal_strip_around_y(self, y: f32, thickness: f32) -> Self {
        let (min_y, max_y) = strip_bounds_around(self.min.y, self.max.y, y, thickness);
        Self::from_min_max(Point::new(self.min.x, min_y), Point::new(self.max.x, max_y))
    }

    /// Return a vertical strip centered around `x`, shifted inside this rectangle.
    pub fn vertical_strip_around_x(self, x: f32, thickness: f32) -> Self {
        let (min_x, max_x) = strip_bounds_around(self.min.x, self.max.x, x, thickness);
        Self::from_min_max(Point::new(min_x, self.min.y), Point::new(max_x, self.max.y))
    }
}

fn strip_bounds_around(min: f32, max: f32, center: f32, thickness: f32) -> (f32, f32) {
    let span = (max - min).max(0.0);
    let thickness = thickness.max(0.0).min(span);
    if thickness <= 0.0 {
        let center = center.clamp(min, max);
        return (center, center);
    }
    let start = (center - thickness * 0.5).clamp(min, max - thickness);
    (start, start + thickness)
}
