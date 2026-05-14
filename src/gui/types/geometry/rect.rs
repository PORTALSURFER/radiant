use super::{Point, Vector2};

mod edges;
mod insets;
mod squares;
mod stroke;

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

    /// Return the smallest rectangle that contains both input rectangles.
    pub fn union(self, other: Self) -> Self {
        Self::from_min_max(
            Point::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y)),
            Point::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y)),
        )
    }
}
