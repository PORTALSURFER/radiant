/// Insets applied before resolving a single text-line placement rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextLineInsets {
    /// Left inset applied before line placement.
    pub left: f32,
    /// Right inset applied before line placement.
    pub right: f32,
    /// Top inset applied before line placement.
    pub top: f32,
    /// Bottom inset applied before line placement.
    pub bottom: f32,
}

impl TextLineInsets {
    /// Build equal horizontal and vertical insets.
    pub fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical,
        }
    }

    /// Build horizontal-only insets.
    pub fn horizontal(horizontal: f32) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: 0.0,
            bottom: 0.0,
        }
    }
}
