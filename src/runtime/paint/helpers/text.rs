use crate::{
    gui::types::{Rect, Rgba8},
    runtime::{PaintPrimitive, PaintText, PaintTextAlign, PaintTextRun},
    widgets::{TextWrap, WidgetId},
};

/// Text metrics used by paint-helper text runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintTextMetrics {
    /// Font size in logical pixels.
    pub font_size: f32,
    /// Optional baseline offset in logical pixels.
    pub baseline: Option<f32>,
}

impl PaintTextMetrics {
    /// Create explicit text metrics.
    pub const fn new(font_size: f32, baseline: Option<f32>) -> Self {
        Self {
            font_size,
            baseline,
        }
    }
}

impl Default for PaintTextMetrics {
    fn default() -> Self {
        Self {
            font_size: 12.0,
            baseline: Some(16.0),
        }
    }
}

/// Push a single-line text run with explicit metrics into a runtime paint primitive buffer.
pub fn push_text_run_with_metrics(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    text: impl Into<String>,
    rect: Rect,
    color: Rgba8,
    align: PaintTextAlign,
    metrics: PaintTextMetrics,
) {
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text: PaintText::from(text.into()),
        rect,
        font_size: metrics.font_size,
        baseline: metrics.baseline,
        color,
        align,
        wrap: TextWrap::None,
    }));
}

/// Push a compact default single-line text run into a runtime paint primitive buffer.
pub fn push_text(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    text: impl Into<String>,
    rect: Rect,
    color: Rgba8,
    align: PaintTextAlign,
) {
    push_text_run_with_metrics(
        primitives,
        widget_id,
        text,
        rect,
        color,
        align,
        PaintTextMetrics::default(),
    );
}
