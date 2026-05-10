//! Badge paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextAlign, PaintTextRun, button_font_size, inset_rect,
    optical_centered_baseline, push_text_run,
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
    let font_size = button_font_size(bounds);
    let rect = inset_rect(bounds, 8.0, 3.0);
    push_text_run(
        primitives,
        PaintTextRun {
            widget_id: badge.common.id,
            text: badge.props.label.clone(),
            rect,
            baseline: optical_centered_baseline(rect, font_size),
            color: crate::widgets::resolve_widget_visual_tokens(
                theme,
                badge.common.style,
                badge.common.state,
            )
            .foreground,
            align: PaintTextAlign::Center,
            wrap: TextWrap::None,
            font_size,
        },
    );
}
