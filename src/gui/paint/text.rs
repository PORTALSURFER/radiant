use crate::gui::types::{Point, Rgba8};

/// Horizontal alignment strategy for text runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    /// Align text to the left edge of its layout width.
    Left,
    /// Align text to the center of its layout width.
    Center,
    /// Align text to the right edge of its layout width.
    Right,
}

/// Single-line text primitive emitted by a paint pass.
#[derive(Clone, Debug, PartialEq)]
pub struct TextRun {
    /// Text content.
    pub text: String,
    /// Top-left anchor point for the run.
    pub position: Point,
    /// Font size in logical pixels-per-em.
    pub font_size: f32,
    /// Text color.
    pub color: Rgba8,
    /// Optional clipping width.
    pub max_width: Option<f32>,
    /// Horizontal alignment inside `max_width`.
    pub align: TextAlign,
}
