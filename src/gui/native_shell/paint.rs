//! Backend-neutral paint primitives emitted by the native shell.

use crate::gui::types::{ImageRgba, Point, Rect, Rgba8};
use std::sync::Arc;

/// Filled rectangle draw primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FillRect {
    pub rect: Rect,
    pub color: Rgba8,
}

/// Filled circle draw primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FillCircle {
    pub center: Point,
    pub radius: f32,
    pub color: Rgba8,
}

/// Filled rectangle draw primitive using a linear gradient in shell coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FillLinearGradient {
    pub rect: Rect,
    pub start: Point,
    pub end: Point,
    pub start_color: Rgba8,
    pub end_color: Rgba8,
}

/// Textured RGBA image draw primitive stretched into one destination rect.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DrawImage {
    /// Destination rectangle in logical shell coordinates.
    pub rect: Rect,
    /// Source RGBA image payload.
    pub image: Arc<ImageRgba>,
}

/// Horizontal alignment strategy for text runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextAlign {
    Left,
    Center,
    Right,
}

/// Single-line text primitive emitted by the shell paint pass.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextRun {
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
pub(crate) enum Primitive {
    Rect(FillRect),
    Circle(FillCircle),
    LinearGradient(FillLinearGradient),
    Image(DrawImage),
}

/// Full frame emitted by the retained shell render pipeline.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct NativeViewFrame {
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
    fn native_view_frame_equality_includes_gradient_primitives() {
        let first = NativeViewFrame {
            primitives: vec![gradient_with_end_alpha(80)],
            ..NativeViewFrame::default()
        };
        let same = NativeViewFrame {
            primitives: vec![gradient_with_end_alpha(80)],
            ..NativeViewFrame::default()
        };
        let changed = NativeViewFrame {
            primitives: vec![gradient_with_end_alpha(81)],
            ..NativeViewFrame::default()
        };

        assert_eq!(first, same);
        assert_ne!(first, changed);
    }
}
