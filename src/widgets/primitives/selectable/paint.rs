//! Selectable paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{
    PaintFillRect, PaintPrimitive, PaintTextAlign, PaintTextRun, ResolvedEnvironment, inset_rect,
    optical_centered_baseline, push_text_run,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::{
    ColorMarkerAlign, selectable::SelectableWidget, support::push_control_chrome, text::TextWrap,
};
use crate::widgets::{Widget, WidgetPaintContext};

const SELECTABLE_MARKER_TEXT_GAP: f32 = 4.0;

pub(super) fn push_selectable_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    selectable: &SelectableWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_selectable_widget_paint_resolved(
        primitives,
        selectable,
        bounds,
        theme,
        &ResolvedEnvironment::default(),
    );
}

pub(super) fn push_selectable_widget_paint_with_context(
    context: &mut WidgetPaintContext<'_>,
    selectable: &SelectableWidget,
) {
    let bounds = context.bounds();
    let theme = context.theme();
    let environment = context.environment().clone();
    let primitives = context.primitives();
    push_selectable_widget_paint_resolved(primitives, selectable, bounds, theme, &environment);
}

fn push_selectable_widget_paint_resolved(
    primitives: &mut Vec<PaintPrimitive>,
    selectable: &SelectableWidget,
    bounds: Rect,
    theme: &ThemeTokens,
    environment: &ResolvedEnvironment,
) {
    push_control_chrome(primitives, &selectable.common, bounds, theme);
    let marker = push_color_marker(primitives, selectable, bounds);
    let metrics = selectable
        .declared_text_metrics()
        .resolve(environment, selectable.text_scale_participation());
    let font_size = metrics.font_size;
    let rect = text_rect_for_marker(
        inset_rect(bounds, metrics.insets.x, metrics.insets.y),
        marker,
    );
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

fn push_color_marker(
    primitives: &mut Vec<PaintPrimitive>,
    selectable: &SelectableWidget,
    bounds: Rect,
) -> Option<(Rect, ColorMarkerAlign)> {
    let props = selectable.props.color_marker?;
    let color = props.color?;
    let rect = props.rect_in(bounds)?;
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: selectable.common.id,
        rect,
        color,
    }));
    Some((rect, props.align))
}

fn text_rect_for_marker(mut text_rect: Rect, marker: Option<(Rect, ColorMarkerAlign)>) -> Rect {
    let Some((marker_rect, align)) = marker else {
        return text_rect;
    };
    match align {
        ColorMarkerAlign::Left => {
            text_rect.min.x = text_rect
                .min
                .x
                .max(marker_rect.max.x + SELECTABLE_MARKER_TEXT_GAP);
        }
        ColorMarkerAlign::Center => {}
        ColorMarkerAlign::Right => {
            text_rect.max.x = text_rect
                .max
                .x
                .min(marker_rect.min.x - SELECTABLE_MARKER_TEXT_GAP);
        }
    }
    if text_rect.max.x < text_rect.min.x {
        text_rect.max.x = text_rect.min.x;
    }
    text_rect
}
