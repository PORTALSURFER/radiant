//! Badge paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextAlign, button_font_size, inset_rect, optical_centered_baseline,
    push_text_run,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::{
    badge::BadgeWidget, support::push_control_chrome, text::TextWrap,
};

pub(super) fn push_badge_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    badge: &BadgeWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &badge.common, bounds, theme);
    push_text_run(
        primitives,
        badge.common.id,
        badge.props.label.clone(),
        inset_rect(bounds, 8.0, 3.0),
        optical_centered_baseline(inset_rect(bounds, 8.0, 3.0), button_font_size(bounds)),
        crate::widgets::resolve_widget_visual_tokens(theme, badge.common.style, badge.common.state)
            .foreground,
        PaintTextAlign::Center,
        TextWrap::None,
        button_font_size(bounds),
    );
}
