use super::ScrollbarAxis;

/// Immutable public properties for a reusable scrollbar widget.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarProps {
    /// Scroll direction represented by the scrollbar.
    pub axis: ScrollbarAxis,
    /// Fraction of the full content currently visible inside the viewport.
    pub viewport_fraction: f32,
    /// Fraction moved by one keyboard arrow press.
    pub step_fraction: f32,
}

/// Mutable interaction state for a reusable scrollbar widget.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollbarState {
    /// Normalized viewport start position.
    pub offset_fraction: f32,
    /// Drag grip inside the thumb measured as `0.0..=1.0` of the thumb length.
    pub drag_grip_fraction: Option<f32>,
}
