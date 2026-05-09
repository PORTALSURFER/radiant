//! Selectable paint command generation.

use super::super::chrome::push_control_chrome;
use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextAlign, inset_rect, optical_centered_baseline, push_text_run,
    text_font_size,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::{selectable::SelectableWidget, text::TextWrap};

pub(in crate::widgets::primitives) fn push_selectable_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    selectable: &SelectableWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &selectable.common, bounds, theme);
    push_text_run(
        primitives,
        selectable.common.id,
        selectable.props.label.clone(),
        inset_rect(bounds, 8.0, 3.0),
        optical_centered_baseline(inset_rect(bounds, 8.0, 3.0), text_font_size(bounds)),
        crate::widgets::resolve_widget_visual_tokens(
            theme,
            selectable.common.style,
            selectable.common.state,
        )
        .foreground,
        PaintTextAlign::Left,
        TextWrap::None,
        text_font_size(bounds),
    );
}
