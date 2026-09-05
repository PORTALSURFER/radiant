//! List-item paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextAlign, PaintTextRun, ResolvedEnvironment, inset_rect,
    optical_centered_baseline, push_text_run,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::{
    list_item::ListItemWidget, support::push_control_chrome, text::TextWrap,
};
use crate::widgets::{Widget, WidgetPaintContext};

pub(super) fn push_list_item_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    item: &ListItemWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_list_item_widget_paint_resolved(
        primitives,
        item,
        bounds,
        theme,
        &ResolvedEnvironment::default(),
    );
}

pub(super) fn push_list_item_widget_paint_with_context(
    context: &mut WidgetPaintContext<'_>,
    item: &ListItemWidget,
) {
    let bounds = context.bounds();
    let theme = context.theme();
    let environment = context.environment().clone();
    let primitives = context.primitives();
    push_list_item_widget_paint_resolved(primitives, item, bounds, theme, &environment);
}

fn push_list_item_widget_paint_resolved(
    primitives: &mut Vec<PaintPrimitive>,
    item: &ListItemWidget,
    bounds: Rect,
    theme: &ThemeTokens,
    environment: &ResolvedEnvironment,
) {
    push_control_chrome(primitives, &item.common, bounds, theme);
    let metrics = item
        .declared_text_metrics()
        .resolve(environment, item.text_scale_participation());
    let font_size = metrics.font_size;
    let label_rect = inset_rect(bounds, metrics.insets.x, metrics.insets.y);
    push_text_run(
        primitives,
        PaintTextRun {
            widget_id: item.common.id,
            text: item.label.clone(),
            rect: label_rect,
            baseline: optical_centered_baseline(label_rect, font_size),
            color: crate::widgets::resolve_widget_visual_tokens(
                theme,
                item.common.style,
                item.common.state,
            )
            .foreground,
            align: PaintTextAlign::Left,
            wrap: TextWrap::None,
            font_size,
        },
    );
    if let Some(detail) = &item.detail {
        let detail_rect = inset_rect(bounds, bounds.width() * 0.5, metrics.insets.y);
        push_text_run(
            primitives,
            PaintTextRun {
                widget_id: item.common.id,
                text: detail.clone(),
                rect: detail_rect,
                baseline: optical_centered_baseline(detail_rect, font_size),
                color: theme.text_muted,
                align: PaintTextAlign::Right,
                wrap: TextWrap::None,
                font_size,
            },
        );
    }
}
