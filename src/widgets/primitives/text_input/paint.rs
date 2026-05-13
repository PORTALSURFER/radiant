use crate::gui::types::Rect;
use crate::runtime::{
    PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintTextInput, blend_color, input_font_size,
    inset_rect, optical_centered_baseline,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::{WidgetCommon, text_input::TextInputWidget};

fn push_text_input_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    common: &WidgetCommon,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = crate::widgets::resolve_widget_visual_tokens(theme, common.style, common.state);
    let fill = if common.state.disabled {
        tokens.fill
    } else if common.state.hovered {
        blend_color(
            theme.bg_primary,
            theme.surface_raised,
            theme.state_hover_strong,
        )
    } else {
        theme.bg_primary
    };
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: common.id,
        rect: bounds,
        color: fill,
    }));
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id: common.id,
        rect: bounds,
        color: tokens.border,
        width: 1.0,
    }));
    if common.state.focused && common.paint.paints_focus {
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: common.id,
            rect: inset_rect(bounds, -1.0, -1.0),
            color: tokens.emphasis,
            width: 1.0,
        }));
    }
}

pub(super) fn push_text_input_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    input: &TextInputWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens =
        crate::widgets::resolve_widget_visual_tokens(theme, input.common.style, input.common.state);
    push_text_input_chrome(primitives, &input.common, bounds, theme);
    let rect = inset_rect(bounds, 16.0, 4.0);
    let font_size = input_font_size(bounds);
    primitives.push(PaintPrimitive::TextInput(PaintTextInput {
        widget_id: input.common.id,
        rect,
        placeholder: input.props.placeholder.clone(),
        state: input.state.clone(),
        font_size,
        baseline: optical_centered_baseline(rect, font_size),
        color: tokens.foreground,
        placeholder_color: theme.text_muted,
        selection_color: blend_color(theme.bg_primary, tokens.emphasis, 0.58),
        caret_color: theme.accent_danger,
        focused: input.common.state.focused,
    }));
}
