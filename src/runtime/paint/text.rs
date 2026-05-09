//! Text paint helpers for backend-neutral paint plans.

use crate::gui::types::{Rect, Rgba8};
use crate::widgets::{TextWrap, WidgetId};

use super::{PaintPrimitive, PaintTextAlign, PaintTextRun};

pub(crate) fn push_text_run(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    text: String,
    rect: Rect,
    baseline: Option<f32>,
    color: Rgba8,
    align: PaintTextAlign,
    wrap: TextWrap,
    font_size: f32,
) {
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text,
        rect,
        font_size,
        baseline,
        color,
        align,
        wrap,
    }));
}

pub(crate) fn text_font_size(rect: Rect) -> f32 {
    if rect.height() >= 38.0 {
        18.0
    } else if rect.height() >= 28.0 {
        14.0
    } else {
        13.0
    }
}

pub(crate) fn button_font_size(rect: Rect) -> f32 {
    if rect.height() >= 48.0 {
        16.0
    } else if rect.height() >= 36.0 {
        14.0
    } else {
        13.0
    }
}

pub(crate) fn input_font_size(rect: Rect) -> f32 {
    if rect.height() >= 42.0 { 15.0 } else { 13.0 }
}

pub(crate) fn optical_centered_baseline(rect: Rect, font_size: f32) -> Option<f32> {
    Some((rect.height() * 0.5 + font_size * 0.35).max(0.0))
}
