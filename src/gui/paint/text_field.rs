use crate::gui::types::{Point, Rect, Rgba8};

use super::{
    BorderSides, FillRect, Primitive, TextAlign, TextRun, shapes::for_each_border_fill_rect,
};

/// Inputs for painting one active single-line text field.
#[derive(Clone, Debug, PartialEq)]
pub struct TextFieldPaintGeometry {
    /// Full text-field chrome rect.
    pub field_rect: Rect,
    /// Text content rect inside the field.
    pub text_rect: Rect,
}

/// Text, caret, and selection state for one active single-line text field.
#[derive(Clone, Debug, PartialEq)]
pub struct TextFieldPaintContent {
    /// Visible text content.
    pub text: String,
    /// Caret x-offset inside `text_rect`.
    pub caret_offset: f32,
    /// Optional selected x-span inside `text_rect`.
    pub selection_offsets: Option<(f32, f32)>,
    /// Font size for the visible text.
    pub font_size: f32,
}

/// Colors used to paint one active single-line text field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextFieldPaintColors {
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
}

/// Stroke metrics for one active single-line text field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextFieldPaintStroke {
    /// Border and caret width.
    pub stroke_width: f32,
}

/// Inputs for painting one active single-line text field.
#[derive(Clone, Debug, PartialEq)]
pub struct TextFieldPaint {
    /// Text-field chrome and content rectangles.
    pub geometry: TextFieldPaintGeometry,
    /// Text, caret, and selection state.
    pub content: TextFieldPaintContent,
    /// Field, text, selection, and caret colors.
    pub colors: TextFieldPaintColors,
    /// Border and caret stroke metrics.
    pub stroke: TextFieldPaintStroke,
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
    let mut primitives = Vec::with_capacity(6);
    primitives.push(Primitive::Rect(FillRect {
        rect: input.geometry.field_rect,
        color: input.colors.fill_color,
    }));
    for_each_border_fill_rect(
        input.geometry.field_rect,
        input.colors.border_color,
        input.stroke.stroke_width,
        BorderSides::ALL,
        |rect| primitives.push(Primitive::Rect(rect)),
    );

    if let Some((start, end)) = input.content.selection_offsets
        && end > start
    {
        primitives.push(Primitive::Rect(FillRect {
            rect: Rect::from_min_max(
                Point::new(
                    input.geometry.text_rect.min.x + start,
                    input.geometry.text_rect.min.y,
                ),
                Point::new(
                    input.geometry.text_rect.min.x + end,
                    input.geometry.text_rect.max.y,
                ),
            ),
            color: input.colors.selection_color,
        }));
    }

    let text_run = (!input.content.text.is_empty()).then(|| TextRun {
        text: input.content.text,
        position: input.geometry.text_rect.min,
        font_size: input.content.font_size,
        color: input.colors.text_color,
        max_width: Some(input.geometry.text_rect.width().max(24.0)),
        align: TextAlign::Left,
    });

    let stroke = input.stroke.stroke_width.max(1.0);
    primitives.push(Primitive::Rect(FillRect {
        rect: Rect::from_min_max(
            Point::new(
                input.geometry.text_rect.min.x + input.content.caret_offset,
                input.geometry.text_rect.min.y,
            ),
            Point::new(
                input.geometry.text_rect.min.x + input.content.caret_offset + stroke,
                input.geometry.text_rect.max.y,
            ),
        ),
        color: input.colors.caret_color,
    }));

    TextFieldPaintOutput {
        primitives,
        text_run,
    }
}
