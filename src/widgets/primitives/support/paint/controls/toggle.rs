//! Toggle paint command generation.

use super::super::chrome::{push_checkbox_chrome, push_control_chrome};
use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextAlign, inset_rect, optical_centered_baseline, push_text_run,
    text_font_size,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::{text::TextWrap, toggle::ToggleWidget};

pub(in crate::widgets::primitives) fn push_toggle_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    toggle: &ToggleWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = crate::widgets::resolve_widget_visual_tokens(
        theme,
        toggle.common.style,
        toggle.common.state,
    );
    if toggle.props.label.is_empty() {
        push_checkbox_chrome(
            primitives,
            toggle.common.id,
            bounds,
            theme,
            toggle.common.state,
            toggle.state.checked,
        );
    } else {
        push_control_chrome(primitives, &toggle.common, bounds, theme);
        push_text_run(
            primitives,
            toggle.common.id,
            toggle.props.label.clone(),
            inset_rect(bounds, 8.0, 4.0),
            optical_centered_baseline(inset_rect(bounds, 8.0, 4.0), text_font_size(bounds)),
            tokens.foreground,
            PaintTextAlign::Left,
            TextWrap::None,
            text_font_size(bounds),
        );
    }
}
