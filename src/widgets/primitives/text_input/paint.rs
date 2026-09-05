use crate::gui::types::Rect;
use crate::runtime::{
    PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintTextInput, ResolvedEnvironment,
    blend_color, inset_rect, optical_centered_baseline,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::{
    WidgetCommon,
    support::push_automation_active_marker,
    text_input::{TextInputChrome, TextInputWidget},
};
use crate::widgets::{ResolvedTextMetrics, Widget, WidgetPaintContext};

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
        push_automation_active_marker(primitives, common.id, bounds, common.state, tokens.emphasis);
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
    push_automation_active_marker(primitives, common.id, bounds, common.state, tokens.emphasis);
}

pub(super) fn push_text_input_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    input: &TextInputWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_text_input_widget_paint_resolved(
        primitives,
        input,
        bounds,
        theme,
        &ResolvedEnvironment::default(),
        input.composition_hides_native_adornments(),
    );
}

pub(super) fn push_text_input_widget_paint_with_hidden_composition(
    primitives: &mut Vec<PaintPrimitive>,
    input: &TextInputWidget,
    bounds: Rect,
    theme: &ThemeTokens,
    hidden_composition: bool,
) {
    push_text_input_widget_paint_resolved(
        primitives,
        input,
        bounds,
        theme,
        &ResolvedEnvironment::default(),
        hidden_composition,
    );
}

pub(super) fn push_text_input_widget_paint_with_context_hidden_composition(
    context: &mut WidgetPaintContext<'_>,
    input: &TextInputWidget,
    hidden_composition: bool,
) {
    let environment = context.environment().clone();
    let bounds = context.bounds();
    let theme = context.theme();
    let primitives = context.primitives();
    push_text_input_widget_paint_resolved(
        primitives,
        input,
        bounds,
        theme,
        &environment,
        hidden_composition,
    );
}

fn push_text_input_widget_paint_resolved(
    primitives: &mut Vec<PaintPrimitive>,
    input: &TextInputWidget,
    bounds: Rect,
    theme: &ThemeTokens,
    environment: &ResolvedEnvironment,
    hidden_composition: bool,
) {
    let tokens =
        crate::widgets::resolve_widget_visual_tokens(theme, input.common.style, input.common.state);
    push_text_input_chrome(primitives, &input.common, input.props.chrome, bounds, theme);
    let declared = input.declared_text_metrics();
    let metrics: ResolvedTextMetrics =
        declared.resolve(environment, input.text_scale_participation());
    let rect = inset_rect(bounds, metrics.insets.x, metrics.insets.y);
    let font_size = metrics.font_size;
    let mut selection_color = text_input_selection_color(theme);
    let mut caret_color = theme.accent_danger;
    if hidden_composition {
        selection_color.a = 0;
        caret_color.a = 0;
    }
    primitives.push(PaintPrimitive::TextInput(PaintTextInput {
        widget_id: input.common.id,
        rect,
        placeholder: input.props.placeholder.clone(),
        completion_suffix: input.props.completion_suffix.clone(),
        state: input.state.clone(),
        font_size,
        align: input.align.resolve(environment.writing_direction()),
        baseline: optical_centered_baseline(rect, font_size),
        color: tokens.foreground,
        placeholder_color: theme.text_muted,
        completion_color: theme.text_muted,
        selection_color,
        caret_color,
        focused: input.common.state.focused,
    }));
}

fn text_input_selection_color(theme: &ThemeTokens) -> crate::gui::types::Rgba8 {
    blend_color(theme.bg_primary, theme.accent_danger, 0.34)
}
