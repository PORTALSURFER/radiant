//! Text paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextAlign, PaintTextRun, optical_centered_baseline, push_text_run,
    text_font_size,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::text::{TextAlign, TextWidget};

pub(super) fn push_text_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    text: &TextWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let font_size = text_font_size(bounds);
    push_text_run(
        primitives,
        PaintTextRun {
            widget_id: text.common.id,
            text: text.text.clone(),
            rect: bounds,
            baseline: optical_centered_baseline(bounds, font_size),
            color: theme.text_primary,
            align: match text.align {
                TextAlign::Left => PaintTextAlign::Left,
                TextAlign::Center => PaintTextAlign::Center,
                TextAlign::Right => PaintTextAlign::Right,
            },
            wrap: text.wrap,
            font_size,
        },
    );
}
