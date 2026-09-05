//! Text paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextRun, ResolvedEnvironment, inset_rect, optical_centered_baseline,
    push_fill_rect, push_text_run, text_font_size,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::text::{TextBackgroundRole, TextWidget};
use crate::widgets::{DeclaredTextMetrics, ResolvedTextMetrics, Widget, WidgetPaintContext};

pub(super) fn push_text_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    text: &TextWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_text_widget_paint_resolved(
        primitives,
        text,
        bounds,
        theme,
        &ResolvedEnvironment::default(),
    );
}

pub(super) fn push_text_widget_paint_with_context(
    context: &mut WidgetPaintContext<'_>,
    text: &TextWidget,
) {
    let bounds = context.bounds();
    let theme = context.theme();
    let environment = context.environment().clone();
    let primitives = context.primitives();
    push_text_widget_paint_resolved(primitives, text, bounds, theme, &environment);
}

fn push_text_widget_paint_resolved(
    primitives: &mut Vec<PaintPrimitive>,
    text: &TextWidget,
    bounds: crate::gui::types::Rect,
    theme: &ThemeTokens,
    environment: &ResolvedEnvironment,
) {
    if let Some(background) = text.background {
        push_fill_rect(
            primitives,
            text.common.id,
            bounds,
            text_background_color(background, theme),
        );
    }
    let declared = DeclaredTextMetrics::new(
        text.common.sizing,
        text_font_size(crate::gui::types::Rect::from_min_size(
            crate::gui::types::Point::default(),
            text.common.sizing.preferred,
        )),
        text.inset,
    );
    let metrics: ResolvedTextMetrics =
        declared.resolve(environment, text.text_scale_participation());
    let font_size = metrics.font_size;
    let text_rect = inset_rect(bounds, metrics.insets.x, metrics.insets.y);
    push_text_run(
        primitives,
        PaintTextRun {
            widget_id: text.common.id,
            text: text.text.clone(),
            rect: text_rect,
            baseline: optical_centered_baseline(text_rect, font_size),
            color: text_color(text.color, theme),
            align: text.align.resolve(environment.writing_direction()),
            wrap: text.wrap,
            font_size,
        },
    );
}

fn text_color(
    color: crate::widgets::TextColorRole,
    theme: &ThemeTokens,
) -> crate::gui::types::Rgba8 {
    match color {
        crate::widgets::TextColorRole::Primary => theme.text_primary,
        crate::widgets::TextColorRole::Muted => theme.text_muted,
        crate::widgets::TextColorRole::OnAccent => theme.bg_primary,
        crate::widgets::TextColorRole::Custom(color) => color,
    }
}

fn text_background_color(
    background: TextBackgroundRole,
    theme: &ThemeTokens,
) -> crate::gui::types::Rgba8 {
    match background {
        TextBackgroundRole::Accent => theme.accent_mint.blend_toward(theme.bg_primary, 0.12),
        TextBackgroundRole::Custom(color) => color,
    }
}
