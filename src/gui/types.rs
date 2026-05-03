//! Backend-neutral geometry and image buffer types.
use std::sync::Arc;

/// 2D point in logical UI coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// Horizontal coordinate in logical pixels.
    pub x: f32,
    /// Vertical coordinate in logical pixels.
    pub y: f32,
}

impl Point {
    /// Construct a point from x/y coordinates.
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// 2D vector in logical UI coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector2 {
    /// Horizontal component in logical pixels.
    pub x: f32,
    /// Vertical component in logical pixels.
    pub y: f32,
}

impl Vector2 {
    /// Construct a vector from x/y components.
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

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
}

/// RGBA color in 8-bit per channel sRGB space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgba8 {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel.
    pub a: u8,
}

#[cfg(test)]
mod tests {
    use super::{Point, Rect};

    #[test]
    fn rect_clamp_to_limits_rect_to_bounds() {
        let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 120.0));
        let rect = Rect::from_min_max(Point::new(0.0, 40.0), Point::new(50.0, 140.0));

        assert_eq!(
            rect.clamp_to(bounds),
            Rect::from_min_max(Point::new(10.0, 40.0), Point::new(50.0, 120.0))
        );
    }

    #[test]
    fn rect_clamp_to_returns_empty_bounds_origin_for_disjoint_rect() {
        let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 120.0));
        let rect = Rect::from_min_max(Point::new(200.0, 40.0), Point::new(250.0, 80.0));

        assert_eq!(rect.clamp_to(bounds), bounds.empty_at_min());
    }

    #[test]
    fn rect_centered_square_clamps_side_and_centers() {
        let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 70.0));

        assert_eq!(
            rect.centered_square(80.0),
            Rect::from_min_max(Point::new(35.0, 20.0), Point::new(85.0, 70.0))
        );
    }

    #[test]
    fn rect_centered_pixel_square_clamps_and_snaps_origin() {
        let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(49.0, 61.0));

        assert_eq!(
            rect.centered_pixel_square(14.7, 8.0, 20.0),
            Some(Rect::from_min_max(
                Point::new(22.0, 33.0),
                Point::new(36.0, 47.0)
            ))
        );
        assert_eq!(rect.centered_pixel_square(0.0, 0.0, 20.0), None);
    }

    #[test]
    fn rect_centered_odd_pixel_square_forces_odd_side() {
        let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(22.0, 32.0));

        assert_eq!(
            rect.centered_odd_pixel_square(5.0, 9.0),
            Some(Rect::from_min_max(
                Point::new(11.0, 21.0),
                Point::new(20.0, 30.0)
            ))
        );
        assert_eq!(
            Rect::from_min_max(Point::new(0.0, 0.0), Point::new(1.0, 10.0))
                .centered_odd_pixel_square(5.0, 9.0),
            None
        );
    }

    #[test]
    fn rect_stroke_aligned_rect_snaps_to_stroke_grid() {
        let rect = Rect::from_min_max(Point::new(10.4, 20.6), Point::new(111.2, 119.1));

        assert_eq!(
            rect.stroke_aligned_rect(2.0),
            Rect::from_min_max(Point::new(10.0, 20.0), Point::new(112.0, 120.0))
        );
    }

    #[test]
    fn rect_stroke_aligned_rect_keeps_tiny_rects() {
        let rect = Rect::from_min_max(Point::new(10.4, 20.6), Point::new(12.1, 22.2));

        assert_eq!(rect.stroke_aligned_rect(0.25), rect);
    }
}

/// Owned RGBA image buffer used by the GUI layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageRgba {
    /// Image width in pixels.
    pub width: usize,
    /// Image height in pixels.
    pub height: usize,
    /// Shared packed RGBA8 pixels in row-major order.
    ///
    /// Cloning `ImageRgba` reuses this backing storage to avoid deep payload copies.
    pub pixels: Arc<[u8]>,
}

impl ImageRgba {
    /// Create an image buffer from width/height and RGBA8 bytes.
    pub fn new(width: usize, height: usize, pixels: Vec<u8>) -> Option<Self> {
        if pixels.len() != width.saturating_mul(height).saturating_mul(4) {
            return None;
        }
        Some(Self {
            width,
            height,
            pixels: pixels.into(),
        })
    }
}
