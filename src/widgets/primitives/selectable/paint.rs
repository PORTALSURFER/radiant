//! Selectable paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextAlign, PaintTextRun, inset_rect, optical_centered_baseline,
    push_text_run, text_font_size,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::{
    selectable::SelectableWidget, support::push_control_chrome, text::TextWrap,
};

pub(super) fn push_selectable_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    selectable: &SelectableWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &selectable.common, bounds, theme);
    let font_size = text_font_size(bounds);
    let rect = inset_rect(bounds, 8.0, 3.0);
    push_text_run(
        primitives,
        PaintTextRun {
            widget_id: selectable.common.id,
            text: selectable.props.label.clone(),
            rect,
            baseline: optical_centered_baseline(rect, font_size),
            color: crate::widgets::resolve_widget_visual_tokens(
                theme,
                selectable.common.style,
                selectable.common.state,
            )
            .foreground,
            align: PaintTextAlign::Left,
            wrap: TextWrap::None,
            font_size,
        },
    );
}
