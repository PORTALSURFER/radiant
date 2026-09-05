//! Toggle paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextAlign, PaintTextRun, ResolvedEnvironment, inset_rect,
    optical_centered_baseline, push_text_run,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::{
    support::{push_checkbox_chrome, push_control_chrome},
    text::TextWrap,
    toggle::ToggleWidget,
};
use crate::widgets::{Widget, WidgetPaintContext};

pub(super) fn push_toggle_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    toggle: &ToggleWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_toggle_widget_paint_resolved(
        primitives,
        toggle,
        bounds,
        theme,
        &ResolvedEnvironment::default(),
    );
}

pub(super) fn push_toggle_widget_paint_with_context(
    context: &mut WidgetPaintContext<'_>,
    toggle: &ToggleWidget,
) {
    let bounds = context.bounds();
    let theme = context.theme();
    let environment = context.environment().clone();
    let primitives = context.primitives();
    push_toggle_widget_paint_resolved(primitives, toggle, bounds, theme, &environment);
}

fn push_toggle_widget_paint_resolved(
    primitives: &mut Vec<PaintPrimitive>,
    toggle: &ToggleWidget,
    bounds: crate::gui::types::Rect,
    theme: &ThemeTokens,
    environment: &ResolvedEnvironment,
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
        let metrics = toggle
            .declared_text_metrics()
            .resolve(environment, toggle.text_scale_participation());
        let font_size = metrics.font_size;
        let rect = inset_rect(bounds, metrics.insets.x, metrics.insets.y);
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
