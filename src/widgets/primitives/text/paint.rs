//! Text paint command generation.

use crate::gui::types::Rect;
use crate::runtime::{
    PaintPrimitive, PaintTextAlign, PaintTextRun, inset_rect, optical_centered_baseline,
    push_fill_rect, push_text_run, text_font_size,
};
use crate::theme::ThemeTokens;
use crate::widgets::primitives::text::{TextAlign, TextBackgroundRole, TextWidget};

pub(super) fn push_text_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    text: &TextWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    if let Some(background) = text.background {
        push_fill_rect(
            primitives,
            text.common.id,
            bounds,
            text_background_color(background, theme),
        );
    }
    let font_size = text_font_size(bounds);
    let text_rect = inset_rect(bounds, text.inset.x, text.inset.y);
    push_text_run(
        primitives,
        PaintTextRun {
            widget_id: text.common.id,
            text: text.text.clone(),
            rect: text_rect,
            baseline: optical_centered_baseline(text_rect, font_size),
            color: text_color(text.color, theme),
            align: match text.align {
                TextAlign::Left => PaintTextAlign::Left,
                TextAlign::Center => PaintTextAlign::Center,
                TextAlign::Right => PaintTextAlign::Right,
            },
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
