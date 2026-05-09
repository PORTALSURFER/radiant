//! Text paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextAlign, optical_centered_baseline, push_text_run, text_font_size,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::text::TextWidget;

pub(in crate::widgets::primitives) fn push_text_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    text: &TextWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_text_run(
        primitives,
        text.common.id,
        text.text.clone(),
        bounds,
        optical_centered_baseline(bounds, text_font_size(bounds)),
        theme.text_primary,
        PaintTextAlign::Left,
        text.wrap,
        text_font_size(bounds),
    );
}
