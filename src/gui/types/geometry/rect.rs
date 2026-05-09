use super::{Point, Vector2};

/// Axis-aligned rectangle in logical UI coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// Minimum corner of the rectangle.
    pub min: Point,
    /// Maximum corner of the rectangle.
    pub max: Point,
}

impl Rect {
    /// Construct a rectangle from minimum and maximum corners.
    pub fn from_min_max(min: Point, max: Point) -> Self {
        Self { min, max }
    }

    /// Construct a rectangle from a minimum corner and size.
    pub fn from_min_size(min: Point, size: Vector2) -> Self {
        Self {
            min,
            max: Point::new(min.x + size.x, min.y + size.y),
        }
    }

    /// Rectangle width in logical coordinates.
    pub fn width(self) -> f32 {
        self.max.x - self.min.x
    }

    /// Rectangle height in logical coordinates.
    pub fn height(self) -> f32 {
        self.max.y - self.min.y
    }

    /// Return the geometric center point.
    pub fn center(self) -> Point {
        Point::new(
            self.min.x + (self.width() * 0.5),
            self.min.y + (self.height() * 0.5),
        )
    }

    /// Return whether the point lies inside the rectangle bounds.
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// Return an empty rectangle at this rectangle's minimum corner.
    pub fn empty_at_min(self) -> Self {
        Self::from_min_max(self.min, self.min)
    }

    /// Return an empty rectangle at this rectangle's maximum corner.
    pub fn empty_at_max(self) -> Self {
        Self::from_min_max(self.max, self.max)
    }

    /// Clamp this rectangle inside `bounds`.
    ///
    /// If the rectangle does not overlap `bounds`, this returns an empty
    /// rectangle at `bounds.min`.
    pub fn clamp_to(self, bounds: Rect) -> Self {
        let min = Point::new(self.min.x.max(bounds.min.x), self.min.y.max(bounds.min.y));
        let max = Point::new(self.max.x.min(bounds.max.x), self.max.y.min(bounds.max.y));
        if max.x < min.x || max.y < min.y {
            return bounds.empty_at_min();
        }
        Self::from_min_max(min, max)
    }

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

    /// Snap rectangle bounds to a stroke-width grid for even retained borders.
    ///
    /// Tiny rectangles keep their original bounds when snapping would leave too
    /// little room for both stroke edges.
    pub fn stroke_aligned_rect(self, stroke: f32) -> Self {
        let stroke = stroke.max(1.0);
        let snap = |value: f32| (value / stroke).round() * stroke;
        let snapped = Self::from_min_max(
            Point::new(snap(self.min.x), snap(self.min.y)),
            Point::new(snap(self.max.x), snap(self.max.y)),
        );
        if snapped.width() <= stroke * 2.0 || snapped.height() <= stroke * 2.0 {
            self
        } else {
            snapped
        }
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

    /// Return the smallest rectangle that contains both input rectangles.
    pub fn union(self, other: Self) -> Self {
        Self::from_min_max(
            Point::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y)),
            Point::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y)),
        )
    }
}
