//! Button paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextAlign, PaintTextRun, ResolvedEnvironment, inset_rect,
    optical_centered_baseline, push_text_run,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::{
    button::ButtonWidget, support::push_button_chrome, text::TextWrap,
};
use crate::widgets::{Widget, WidgetPaintContext};

pub(super) fn push_button_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    button: &ButtonWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_button_widget_paint_resolved(
        primitives,
        button,
        bounds,
        theme,
        &ResolvedEnvironment::default(),
    );
}

pub(super) fn push_button_widget_paint_with_context(
    context: &mut WidgetPaintContext<'_>,
    button: &ButtonWidget,
) {
    let bounds = context.bounds();
    let theme = context.theme();
    let environment = context.environment().clone();
    let primitives = context.primitives();
    push_button_widget_paint_resolved(primitives, button, bounds, theme, &environment);
}

fn push_button_widget_paint_resolved(
    primitives: &mut Vec<PaintPrimitive>,
    button: &ButtonWidget,
    bounds: Rect,
    theme: &ThemeTokens,
    environment: &ResolvedEnvironment,
) {
    if !button.common.paint.paints_state_layers {
        return;
    }
    if button.props.hover_chrome_only
        && !button.common.state.hovered
        && !button.common.state.pressed
        && !button.common.state.focused
        && !button.common.state.selected
        && !button.common.state.active
    {
        return;
    }
    push_button_chrome(primitives, &button.common, bounds, theme);
    let metrics = button
        .declared_text_metrics()
        .resolve(environment, button.text_scale_participation());
    let font_size = metrics.font_size;
    let rect = inset_rect(bounds, metrics.insets.x, metrics.insets.y);
    let trailing_width =
        if button.trailing_icon.is_some() || button.trailing_icon_tint_cache.is_some() {
            font_size.max(16.0)
        } else {
            0.0
        };
    let (label_rect, trailing_rect) = match button.props.trailing_label.as_ref() {
        Some(_) => {
            let split = (rect.max.x - font_size.max(12.0)).max(rect.min.x);
            let mut label_rect = rect;
            label_rect.max.x = split;
            let mut trailing_rect = rect;
            trailing_rect.min.x = split;
            (label_rect, Some(trailing_rect))
        }
        None if trailing_width > 0.0 => {
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
            align: button
                .props
                .text_align
                .resolve(environment.writing_direction()),
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
    if let (Some(cache), Some(trailing_rect)) = (button.trailing_icon_tint_cache, trailing_rect) {
        cache
            .icon(foreground)
            .append_paint(primitives, button.common.id, trailing_rect);
    } else if let (Some(icon), Some(trailing_rect)) = (button.trailing_icon.as_ref(), trailing_rect)
    {
        icon.append_paint(
            primitives,
            button.common.id,
            crate::gui::types::Rect::from_min_max(
                crate::gui::types::Point::new(trailing_rect.min.x, trailing_rect.min.y),
                crate::gui::types::Point::new(trailing_rect.max.x, trailing_rect.max.y),
            ),
        );
    }
}
