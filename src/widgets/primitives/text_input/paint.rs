use crate::gui::types::Rect;
use crate::runtime::{
    PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintTextInput, blend_color, input_font_size,
    inset_rect, optical_centered_baseline,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::{
    WidgetCommon,
    text_input::{TextInputChrome, TextInputWidget},
};

const COMPACT_INPUT_HEIGHT: f32 = 28.0;

fn push_text_input_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    common: &WidgetCommon,
    chrome: TextInputChrome,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = crate::widgets::resolve_widget_visual_tokens(theme, common.style, common.state);
    if chrome == TextInputChrome::Underline {
        let y = bounds.max.y - 1.0;
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: common.id,
            rect: Rect::from_min_max(
                crate::gui::types::Point::new(bounds.min.x, y),
                crate::gui::types::Point::new(bounds.max.x, bounds.max.y),
            ),
            color: if common.state.focused {
                tokens.emphasis
            } else {
                theme.text_muted
            },
            width: 1.0,
        }));
        return;
    }
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
    push_text_input_chrome(primitives, &input.common, input.props.chrome, bounds, theme);
    let rect = text_input_content_rect(bounds);
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
        selection_color: text_input_selection_color(theme),
        caret_color: theme.accent_danger,
        focused: input.common.state.focused,
    }));
}

fn text_input_content_rect(bounds: Rect) -> Rect {
    if bounds.height() <= COMPACT_INPUT_HEIGHT {
        return inset_rect(bounds, 8.0, 2.0);
    }
    inset_rect(bounds, 16.0, 4.0)
}

fn text_input_selection_color(theme: &ThemeTokens) -> crate::gui::types::Rgba8 {
    blend_color(theme.bg_primary, theme.accent_danger, 0.34)
}
