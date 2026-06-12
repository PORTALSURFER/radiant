use crate::{
    gui::text_layout::{TextLineInsets, centered_text_baseline, centered_text_line},
    gui::types::{Rect, Rgba8},
    runtime::{PaintPrimitive, PaintText, PaintTextAlign, PaintTextRun},
    widgets::{TextWrap, WidgetId},
};

/// Named fields for painting a dense-row label.
#[derive(Clone, Debug, PartialEq)]
pub struct DenseRowLabelParts {
    /// Text to paint.
    pub text: PaintText,
    /// Text color.
    pub color: Rgba8,
    /// Horizontal text inset.
    pub inset_x: f32,
    /// Additional vertical offset after centering.
    pub offset_y: f32,
    /// Horizontal alignment inside the label rectangle.
    pub align: PaintTextAlign,
    /// Text wrapping policy.
    pub wrap: TextWrap,
}

impl DenseRowLabelParts {
    /// Build dense-row label parts with left alignment and no wrapping.
    pub fn new(text: impl Into<PaintText>, color: Rgba8) -> Self {
        Self {
            text: text.into(),
            color,
            inset_x: 4.0,
            offset_y: 0.0,
            align: PaintTextAlign::Left,
            wrap: TextWrap::None,
        }
    }

    /// Set horizontal text inset.
    pub fn inset_x(mut self, inset: f32) -> Self {
        self.inset_x = inset.max(0.0);
        self
    }

    /// Set additional vertical text offset after centering.
    pub fn offset_y(mut self, offset: f32) -> Self {
        self.offset_y = offset;
        self
    }

    /// Set horizontal alignment.
    pub fn align(mut self, align: PaintTextAlign) -> Self {
        self.align = align;
        self
    }

    /// Set wrapping policy.
    pub fn wrap(mut self, wrap: TextWrap) -> Self {
        self.wrap = wrap;
        self
    }
}

/// Return a readable label font size for a dense row height.
///
/// Custom tree and list row painters can use this with Radiant text-line
/// helpers to keep label sizing consistent across compact, medium, and taller
/// dense rows.
pub fn dense_row_label_font_size(row_height: f32) -> f32 {
    if row_height >= 38.0 {
        18.0
    } else if row_height >= 28.0 {
        14.0
    } else {
        13.0
    }
}

/// Push a vertically centered dense-row label.
///
/// Returns `true` when a text primitive was appended.
pub fn push_dense_row_label(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    parts: DenseRowLabelParts,
) -> bool {
    if parts.text.is_empty() || !bounds.has_finite_positive_area() {
        return false;
    }
    let font_size = dense_row_label_font_size(bounds.height());
    let rect = centered_text_line(
        bounds,
        font_size,
        TextLineInsets::horizontal(parts.inset_x),
        parts.offset_y,
    );
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text: parts.text,
        rect,
        font_size,
        baseline: centered_text_baseline(rect, font_size),
        color: parts.color,
        align: parts.align,
        wrap: parts.wrap,
    }));
    true
}
