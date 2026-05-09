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
