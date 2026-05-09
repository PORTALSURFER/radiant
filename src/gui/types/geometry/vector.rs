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
