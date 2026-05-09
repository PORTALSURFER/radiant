//! Button paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextAlign, button_font_size, inset_rect, optical_centered_baseline,
    push_text_run,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::{
    button::ButtonWidget, support::push_button_chrome, text::TextWrap,
};

pub(super) fn push_button_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    button: &ButtonWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_button_chrome(primitives, &button.common, bounds, theme);
    push_text_run(
        primitives,
        button.common.id,
        button.props.label.clone(),
        inset_rect(bounds, 8.0, 4.0),
        optical_centered_baseline(inset_rect(bounds, 8.0, 4.0), button_font_size(bounds)),
        crate::widgets::resolve_widget_visual_tokens(
            theme,
            button.common.style,
            button.common.state,
        )
        .foreground,
        PaintTextAlign::Center,
        TextWrap::None,
        button_font_size(bounds),
    );
}
