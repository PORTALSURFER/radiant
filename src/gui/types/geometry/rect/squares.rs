use super::{Point, Rect};

impl Rect {
    /// Return a centered square of side `side` constrained to this rectangle.
    ///
    /// Empty rectangles or non-positive sides return the original rectangle.
    pub fn centered_square(self, side: f32) -> Self {
        if self.width() <= 0.0 || self.height() <= 0.0 || side <= 0.0 {
            return self;
        }
        let clamped_side = side.min(self.width()).min(self.height());
        let min_x = self.min.x + ((self.width() - clamped_side) * 0.5);
        let min_y = self.min.y + ((self.height() - clamped_side) * 0.5);
        Self::from_min_max(
            Point::new(min_x, min_y),
            Point::new(min_x + clamped_side, min_y + clamped_side),
        )
    }

    /// Return a pixel-snapped centered square with a clamped side length.
    pub fn centered_pixel_square(
        self,
        preferred_side: f32,
        min_side: f32,
        max_side: f32,
    ) -> Option<Self> {
        if self.width() <= 0.0 || self.height() <= 0.0 {
            return None;
        }
        let side = preferred_side
            .floor()
            .clamp(min_side.max(0.0), max_side.max(0.0));
        if side <= 0.0 {
            return None;
        }
        let min_x = self.min.x + ((self.width() - side) * 0.5).floor();
        let min_y = self.min.y + ((self.height() - side) * 0.5).floor();
        Some(Self::from_min_max(
            Point::new(min_x, min_y),
            Point::new(min_x + side, min_y + side),
        ))
    }

    /// Return a pixel-snapped centered square with an odd side length.
    pub fn centered_odd_pixel_square(self, min_side: f32, max_side: f32) -> Option<Self> {
        if self.width() <= 1.0 || self.height() <= 1.0 {
            return None;
        }
        let mut side = self
            .width()
            .min(self.height())
            .floor()
            .clamp(min_side.max(0.0), max_side.max(0.0));
        if (side as i32) % 2 == 0 {
            side -= 1.0;
        }
        if side < min_side.max(0.0) || side <= 0.0 {
            return None;
        }
        let min_x = self.min.x + ((self.width() - side) * 0.5).floor();
        let min_y = self.min.y + ((self.height() - side) * 0.5).floor();
        Some(Self::from_min_max(
            Point::new(min_x, min_y),
            Point::new(min_x + side, min_y + side),
        ))
    }

    /// Return a square anchored to the rectangle's top-right corner.
    ///
    /// Non-positive sides or empty rectangles return an empty rectangle at the
    /// top-right anchor.
    pub fn top_right_square(self, side: f32, inset: f32) -> Self {
        let inset = inset.max(0.0);
        let max = Point::new(
            (self.max.x - inset).max(self.min.x),
            (self.min.y + inset).min(self.max.y),
        );
        if self.width() <= 0.0 || self.height() <= 0.0 || side <= 0.0 {
            return Self::from_min_max(max, max);
        }
        let side = side.min(self.width()).min(self.height());
        Self::from_min_max(
            Point::new(max.x - side, max.y),
            Point::new(max.x, max.y + side),
        )
        .clamp_to(self)
    }
}
