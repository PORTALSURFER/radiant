//! Button paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextAlign, PaintTextRun, button_font_size, inset_rect,
    optical_centered_baseline, push_text_run,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::{
    button::ButtonWidget,
    support::push_button_chrome,
    text::{TextAlign, TextWrap},
};

pub(super) fn push_button_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    button: &ButtonWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    if !button.common.paint.paints_state_layers {
        return;
    }
    if button.props.hover_chrome_only
        && !button.common.state.hovered
        && !button.common.state.pressed
        && !button.common.state.focused
    {
        return;
    }
    push_button_chrome(primitives, &button.common, bounds, theme);
    let font_size = button_font_size(bounds);
    let rect = inset_rect(bounds, 8.0, 4.0);
    push_text_run(
        primitives,
        PaintTextRun {
            widget_id: button.common.id,
            text: button.props.label.clone(),
            rect,
            baseline: optical_centered_baseline(rect, font_size),
            color: crate::widgets::resolve_widget_visual_tokens(
                theme,
                button.common.style,
                button.common.state,
            )
            .foreground,
            align: match button.props.text_align {
                TextAlign::Left => PaintTextAlign::Left,
                TextAlign::Center => PaintTextAlign::Center,
                TextAlign::Right => PaintTextAlign::Right,
            },
            wrap: TextWrap::None,
            font_size,
        },
    );
}
