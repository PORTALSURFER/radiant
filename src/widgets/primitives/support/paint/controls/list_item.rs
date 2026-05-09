//! List-item paint command generation.

use super::super::chrome::push_control_chrome;
use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextAlign, inset_rect, optical_centered_baseline, push_text_run,
    text_font_size,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::{list_item::ListItemWidget, text::TextWrap};

pub(in crate::widgets::primitives) fn push_list_item_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    item: &ListItemWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &item.common, bounds, theme);
    push_text_run(
        primitives,
        item.common.id,
        item.label.clone(),
        inset_rect(bounds, 8.0, 3.0),
        optical_centered_baseline(inset_rect(bounds, 8.0, 3.0), text_font_size(bounds)),
        crate::widgets::resolve_widget_visual_tokens(theme, item.common.style, item.common.state)
            .foreground,
        PaintTextAlign::Left,
        TextWrap::None,
        text_font_size(bounds),
    );
    if let Some(detail) = &item.detail {
        push_text_run(
            primitives,
            item.common.id,
            detail.clone(),
            inset_rect(bounds, bounds.width() * 0.5, 3.0),
            optical_centered_baseline(
                inset_rect(bounds, bounds.width() * 0.5, 3.0),
                text_font_size(bounds),
            ),
            theme.text_muted,
            PaintTextAlign::Right,
            TextWrap::None,
            text_font_size(bounds),
        );
    }
}
