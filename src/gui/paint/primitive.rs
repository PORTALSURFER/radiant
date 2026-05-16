use super::{DrawImage, DrawSvg, FillCircle, FillLinearGradient, FillRect};

/// Backend-neutral scene primitive.
#[derive(Clone, Debug, PartialEq)]
pub enum Primitive {
    /// Filled rectangle primitive.
    Rect(FillRect),
    /// Filled circle primitive.
    Circle(FillCircle),
    /// Filled linear gradient primitive.
    LinearGradient(FillLinearGradient),
    /// Textured image primitive.
    Image(DrawImage),
    /// Parsed SVG document primitive.
    Svg(DrawSvg),
}
