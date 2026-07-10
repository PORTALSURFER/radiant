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
    let (label_rect, trailing_rect) = match button.props.trailing_label.as_ref() {
        Some(_) => {
            let trailing_width = font_size.max(12.0);
            let split = (rect.max.x - trailing_width).max(rect.min.x);
            let mut label_rect = rect;
            label_rect.max.x = split;
            let mut trailing_rect = rect;
            trailing_rect.min.x = split;
            (label_rect, Some(trailing_rect))
        }
        None => (rect, None),
    };
    let foreground = crate::widgets::resolve_widget_visual_tokens(
        theme,
        button.common.style,
        button.common.state,
    )
    .foreground;
    push_text_run(
        primitives,
        PaintTextRun {
            widget_id: button.common.id,
            text: button.props.label.clone(),
            rect: label_rect,
            baseline: optical_centered_baseline(label_rect, font_size),
            color: foreground,
            align: match button.props.text_align {
                TextAlign::Left => PaintTextAlign::Left,
                TextAlign::Center => PaintTextAlign::Center,
                TextAlign::Right => PaintTextAlign::Right,
            },
            wrap: TextWrap::None,
            font_size,
        },
    );
    if let (Some(trailing), Some(trailing_rect)) =
        (button.props.trailing_label.as_ref(), trailing_rect)
    {
        push_text_run(
            primitives,
            PaintTextRun {
                widget_id: button.common.id,
                text: trailing.clone(),
                rect: trailing_rect,
                baseline: optical_centered_baseline(trailing_rect, font_size),
                color: foreground,
                align: PaintTextAlign::Right,
                wrap: TextWrap::None,
                font_size,
            },
        );
    }
}
