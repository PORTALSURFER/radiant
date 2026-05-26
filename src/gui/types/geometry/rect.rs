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

    /// Construct a normalized rectangle from any two opposite corners.
    pub fn from_points(a: Point, b: Point) -> Self {
        Self::from_min_max(
            Point::new(a.x.min(b.x), a.y.min(b.y)),
            Point::new(a.x.max(b.x), a.y.max(b.y)),
        )
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

    /// Return whether both corners and derived extents are finite.
    pub fn is_finite(self) -> bool {
        self.min.is_finite()
            && self.max.is_finite()
            && self.width().is_finite()
            && self.height().is_finite()
    }

    /// Return whether this rectangle has finite coordinates and positive area.
    pub fn has_finite_positive_area(self) -> bool {
        self.is_finite() && self.width() > 0.0 && self.height() > 0.0
    }

    /// Return the geometric center point.
    pub fn center(self) -> Point {
        Point::new(
            self.min.x + (self.width() * 0.5),
            self.min.y + (self.height() * 0.5),
        )
    }

    /// Project a normalized horizontal ratio into this rectangle.
    pub fn x_for_ratio(self, ratio: f32) -> f32 {
        self.x_for_ratio_unclamped(ratio.clamp(0.0, 1.0))
    }

    /// Project a normalized vertical ratio into this rectangle from top to bottom.
    pub fn y_for_ratio(self, ratio: f32) -> f32 {
        self.y_for_ratio_unclamped(ratio.clamp(0.0, 1.0))
    }

    /// Project a normalized vertical ratio into this rectangle from bottom to top.
    pub fn y_for_ratio_from_bottom(self, ratio: f32) -> f32 {
        self.y_for_ratio_from_bottom_unclamped(ratio.clamp(0.0, 1.0))
    }

    /// Project a horizontal ratio into this rectangle without clamping.
    pub fn x_for_ratio_unclamped(self, ratio: f32) -> f32 {
        self.min.x + self.width() * ratio
    }

    /// Return a full-height sub-rectangle bounded by two horizontal ratios.
    pub fn horizontal_ratio_span(self, start_ratio: f32, end_ratio: f32) -> Self {
        let start = self.x_for_ratio(start_ratio);
        let end = self.x_for_ratio(end_ratio);
        Self::from_min_max(
            Point::new(start.min(end), self.min.y),
            Point::new(start.max(end), self.max.y),
        )
    }

    /// Project a vertical ratio into this rectangle without clamping.
    pub fn y_for_ratio_unclamped(self, ratio: f32) -> f32 {
        self.min.y + self.height() * ratio
    }

    /// Project a bottom-up vertical ratio into this rectangle without clamping.
    pub fn y_for_ratio_from_bottom_unclamped(self, ratio: f32) -> f32 {
        self.max.y - self.height() * ratio
    }

    /// Convert an x coordinate into a normalized horizontal ratio inside this rectangle.
    pub fn ratio_for_x(self, x: f32) -> f32 {
        let width = self.width();
        if !x.is_finite() || !width.is_finite() || width <= f32::EPSILON {
            return 0.0;
        }
        ((x - self.min.x) / width).clamp(0.0, 1.0)
    }

    /// Convert a y coordinate into a normalized vertical ratio inside this rectangle.
    pub fn ratio_for_y(self, y: f32) -> f32 {
        let height = self.height();
        if !y.is_finite() || !height.is_finite() || height <= f32::EPSILON {
            return 0.0;
        }
        ((y - self.min.y) / height).clamp(0.0, 1.0)
    }

    /// Convert a y coordinate into a normalized bottom-up vertical ratio inside this rectangle.
    pub fn ratio_for_y_from_bottom(self, y: f32) -> f32 {
        let height = self.height();
        if !y.is_finite() || !height.is_finite() || height <= f32::EPSILON {
            return 0.0;
        }
        ((self.max.y - y) / height).clamp(0.0, 1.0)
    }

    /// Return whether the point lies inside the rectangle bounds.
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// Return whether this rectangle intersects `other`, including shared edges.
    pub fn intersects(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    /// Return whether this rectangle overlaps `other` with positive area.
    pub fn overlaps(self, other: Self) -> bool {
        self.width() > 0.0
            && self.height() > 0.0
            && other.width() > 0.0
            && other.height() > 0.0
            && self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
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
