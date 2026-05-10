//! Toggle paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextAlign, PaintTextRun, inset_rect, optical_centered_baseline,
    push_text_run, text_font_size,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::{
    support::{push_checkbox_chrome, push_control_chrome},
    text::TextWrap,
    toggle::ToggleWidget,
};

pub(super) fn push_toggle_widget_paint(
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
        let font_size = text_font_size(bounds);
        let rect = inset_rect(bounds, 8.0, 4.0);
        push_text_run(
            primitives,
            PaintTextRun {
                widget_id: toggle.common.id,
                text: toggle.props.label.clone(),
                rect,
                baseline: optical_centered_baseline(rect, font_size),
                color: tokens.foreground,
                align: PaintTextAlign::Left,
                wrap: TextWrap::None,
                font_size,
            },
        );
    }
}
