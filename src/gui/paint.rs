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

/// Per-edge border ownership for rectangle border emission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BorderSides {
    /// Draw the top edge.
    pub top: bool,
    /// Draw the bottom edge.
    pub bottom: bool,
    /// Draw the left edge.
    pub left: bool,
    /// Draw the right edge.
    pub right: bool,
}

impl BorderSides {
    /// Draw all four edges.
    pub const ALL: Self = Self {
        top: true,
        bottom: true,
        left: true,
        right: true,
    };
}

/// Return filled rectangles that draw the requested border edges.
pub fn border_fill_rects(
    rect: Rect,
    color: Rgba8,
    stroke: f32,
    sides: BorderSides,
) -> Vec<FillRect> {
    let stroke = stroke.max(1.0);
    if rect.width() <= stroke * 2.0 || rect.height() <= stroke * 2.0 {
        return Vec::new();
    }

    let mut fills = Vec::with_capacity(4);
    if sides.top {
        fills.push(FillRect {
            rect: Rect::from_min_max(rect.min, Point::new(rect.max.x, rect.min.y + stroke)),
            color,
        });
    }
    if sides.bottom {
        fills.push(FillRect {
            rect: Rect::from_min_max(Point::new(rect.min.x, rect.max.y - stroke), rect.max),
            color,
        });
    }
    if sides.left {
        fills.push(FillRect {
            rect: Rect::from_min_max(rect.min, Point::new(rect.min.x + stroke, rect.max.y)),
            color,
        });
    }
    if sides.right {
        fills.push(FillRect {
            rect: Rect::from_min_max(Point::new(rect.max.x - stroke, rect.min.y), rect.max),
            color,
        });
    }
    fills
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

/// Inputs for painting one active single-line text field.
#[derive(Clone, Debug, PartialEq)]
pub struct TextFieldPaint {
    /// Full text-field chrome rect.
    pub field_rect: Rect,
    /// Text content rect inside the field.
    pub text_rect: Rect,
    /// Visible text content.
    pub text: String,
    /// Caret x-offset inside `text_rect`.
    pub caret_offset: f32,
    /// Optional selected x-span inside `text_rect`.
    pub selection_offsets: Option<(f32, f32)>,
    /// Font size for the visible text.
    pub font_size: f32,
    /// Field fill color.
    pub fill_color: Rgba8,
    /// Field border color.
    pub border_color: Rgba8,
    /// Selection fill color.
    pub selection_color: Rgba8,
    /// Caret color.
    pub caret_color: Rgba8,
    /// Text color.
    pub text_color: Rgba8,
    /// Border and caret width.
    pub stroke_width: f32,
}

/// Paint output for one active single-line text field.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextFieldPaintOutput {
    /// Shape primitives for fill, border, selection, and caret.
    pub primitives: Vec<Primitive>,
    /// Optional visible text run.
    pub text_run: Option<TextRun>,
}

/// Build backend-neutral paint primitives for an active single-line text field.
pub fn text_field_paint(input: TextFieldPaint) -> TextFieldPaintOutput {
    let mut primitives = Vec::new();
    primitives.push(Primitive::Rect(FillRect {
        rect: input.field_rect,
        color: input.fill_color,
    }));
    primitives.extend(
        border_fill_rects(
            input.field_rect,
            input.border_color,
            input.stroke_width,
            BorderSides::ALL,
        )
        .into_iter()
        .map(Primitive::Rect),
    );

    if let Some((start, end)) = input.selection_offsets
        && end > start
    {
        primitives.push(Primitive::Rect(FillRect {
            rect: Rect::from_min_max(
                Point::new(input.text_rect.min.x + start, input.text_rect.min.y),
                Point::new(input.text_rect.min.x + end, input.text_rect.max.y),
            ),
            color: input.selection_color,
        }));
    }

    let text_run = (!input.text.is_empty()).then(|| TextRun {
        text: input.text,
        position: input.text_rect.min,
        font_size: input.font_size,
        color: input.text_color,
        max_width: Some(input.text_rect.width().max(24.0)),
        align: TextAlign::Left,
    });

    let stroke = input.stroke_width.max(1.0);
    primitives.push(Primitive::Rect(FillRect {
        rect: Rect::from_min_max(
            Point::new(
                input.text_rect.min.x + input.caret_offset,
                input.text_rect.min.y,
            ),
            Point::new(
                input.text_rect.min.x + input.caret_offset + stroke,
                input.text_rect.max.y,
            ),
        ),
        color: input.caret_color,
    }));

    TextFieldPaintOutput {
        primitives,
        text_run,
    }
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

    #[test]
    fn border_fill_rects_returns_requested_edges() {
        let color = Rgba8 {
            r: 1,
            g: 2,
            b: 3,
            a: 4,
        };
        let rect = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 80.0));

        let fills = border_fill_rects(
            rect,
            color,
            2.0,
            BorderSides {
                top: true,
                bottom: false,
                left: false,
                right: true,
            },
        );

        assert_eq!(fills.len(), 2);
        assert_eq!(
            fills[0].rect,
            Rect::from_min_max(Point::new(10.0, 20.0), Point::new(50.0, 22.0))
        );
        assert_eq!(
            fills[1].rect,
            Rect::from_min_max(Point::new(48.0, 20.0), Point::new(50.0, 80.0))
        );
        assert!(fills.iter().all(|fill| fill.color == color));
    }

    #[test]
    fn border_fill_rects_omits_degenerate_rectangles() {
        let color = Rgba8 {
            r: 1,
            g: 2,
            b: 3,
            a: 4,
        };
        let rect = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(3.0, 12.0));

        assert!(border_fill_rects(rect, color, 2.0, BorderSides::ALL).is_empty());
    }

    #[test]
    fn text_field_paint_emits_chrome_selection_text_and_caret() {
        let color = Rgba8 {
            r: 1,
            g: 2,
            b: 3,
            a: 4,
        };
        let output = text_field_paint(TextFieldPaint {
            field_rect: Rect::from_min_max(Point::new(0.0, 0.0), Point::new(120.0, 24.0)),
            text_rect: Rect::from_min_max(Point::new(8.0, 4.0), Point::new(112.0, 20.0)),
            text: "query".to_string(),
            caret_offset: 36.0,
            selection_offsets: Some((8.0, 24.0)),
            font_size: 12.0,
            fill_color: color,
            border_color: color,
            selection_color: color,
            caret_color: color,
            text_color: color,
            stroke_width: 2.0,
        });

        assert_eq!(output.primitives.len(), 7);
        assert_eq!(
            output.text_run.as_ref().map(|run| run.text.as_str()),
            Some("query")
        );
    }
}
