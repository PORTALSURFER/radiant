//! Text paint helpers for backend-neutral paint plans.

use crate::gui::types::Rect;

use super::{PaintPrimitive, PaintTextRun};

pub(crate) fn push_text_run(primitives: &mut Vec<PaintPrimitive>, run: PaintTextRun) {
    primitives.push(PaintPrimitive::Text(run));
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
    crate::gui::text_layout::centered_text_baseline(rect, font_size)
}
