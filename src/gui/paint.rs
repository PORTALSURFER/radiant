//! Backend-neutral paint primitives and frame payloads for renderer adapters.

use crate::gui::types::{ImageRgba, Point, Rect, Rgba8};
use std::sync::Arc;

/// Filled rectangle draw primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FillRect {
    /// Destination rectangle in logical surface coordinates.
    pub rect: Rect,
    /// Fill color.
    pub color: Rgba8,
}

/// Filled circle draw primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FillCircle {
    /// Circle center in logical surface coordinates.
    pub center: Point,
    /// Circle radius in logical pixels.
    pub radius: f32,
    /// Fill color.
    pub color: Rgba8,
}

/// Filled rectangle draw primitive using a linear gradient in logical surface coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FillLinearGradient {
    /// Destination rectangle for the gradient fill.
    pub rect: Rect,
    /// Gradient start point.
    pub start: Point,
    /// Gradient end point.
    pub end: Point,
    /// Gradient color at `start`.
    pub start_color: Rgba8,
    /// Gradient color at `end`.
    pub end_color: Rgba8,
}

/// Textured RGBA image draw primitive stretched into one destination rect.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawImage {
    /// Destination rectangle in logical surface coordinates.
    pub rect: Rect,
    /// RGBA image payload.
    pub image: Arc<ImageRgba>,
}

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
}

/// Full frame emitted by a retained render pipeline.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintFrame {
    /// Root clear color.
    pub clear_color: Rgba8,
    /// Shape primitives.
    pub primitives: Vec<Primitive>,
    /// Text primitives.
    pub text_runs: Vec<TextRun>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient_with_end_alpha(end_alpha: u8) -> Primitive {
        Primitive::LinearGradient(FillLinearGradient {
            rect: Rect::from_min_max(Point::new(1.0, 2.0), Point::new(11.0, 22.0)),
            start: Point::new(1.0, 2.0),
            end: Point::new(11.0, 2.0),
            start_color: Rgba8 {
                r: 10,
                g: 20,
                b: 30,
                a: 40,
            },
            end_color: Rgba8 {
                r: 50,
                g: 60,
                b: 70,
                a: end_alpha,
            },
        })
    }

    #[test]
    fn linear_gradient_primitive_has_stable_equality() {
        let first = gradient_with_end_alpha(80);
        let same = gradient_with_end_alpha(80);
        let changed = gradient_with_end_alpha(81);

        assert_eq!(first, same);
        assert_ne!(first, changed);
    }

    #[test]
    fn paint_frame_equality_includes_gradient_primitives() {
        let first = PaintFrame {
            primitives: vec![gradient_with_end_alpha(80)],
            ..PaintFrame::default()
        };
        let same = PaintFrame {
            primitives: vec![gradient_with_end_alpha(80)],
            ..PaintFrame::default()
        };
        let changed = PaintFrame {
            primitives: vec![gradient_with_end_alpha(81)],
            ..PaintFrame::default()
        };

        assert_eq!(first, same);
        assert_ne!(first, changed);
    }
}
