//! List-item paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextAlign, PaintTextRun, inset_rect, optical_centered_baseline,
    push_text_run, text_font_size,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::{
    list_item::ListItemWidget, support::push_control_chrome, text::TextWrap,
};

pub(super) fn push_list_item_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    item: &ListItemWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &item.common, bounds, theme);
    let font_size = text_font_size(bounds);
    let label_rect = inset_rect(bounds, 8.0, 3.0);
    push_text_run(
        primitives,
        PaintTextRun {
            widget_id: item.common.id,
            text: item.label.clone(),
            rect: label_rect,
            baseline: optical_centered_baseline(label_rect, font_size),
            color: crate::widgets::resolve_widget_visual_tokens(
                theme,
                item.common.style,
                item.common.state,
            )
            .foreground,
            align: PaintTextAlign::Left,
            wrap: TextWrap::None,
            font_size,
        },
    );
    if let Some(detail) = &item.detail {
        let detail_rect = inset_rect(bounds, bounds.width() * 0.5, 3.0);
        push_text_run(
            primitives,
            PaintTextRun {
                widget_id: item.common.id,
                text: detail.clone(),
                rect: detail_rect,
                baseline: optical_centered_baseline(detail_rect, font_size),
                color: theme.text_muted,
                align: PaintTextAlign::Right,
                wrap: TextWrap::None,
                font_size,
            },
        );
    }
}
