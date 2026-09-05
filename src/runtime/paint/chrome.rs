use super::{
    PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintTextAlign, PaintTextRun,
    geometry::{blend_color, inset_rect},
    text::{optical_centered_baseline, push_text_run, text_font_size_for_height},
};
use crate::{
    gui::types::{Point, Rect, Vector2},
    layout::{LayoutOutput, NodeId},
    theme::ThemeTokens,
    widgets::{
        DeclaredTextMetrics, TextScaleParticipation, TextWrap, WidgetId, WidgetSizing, WidgetState,
        WidgetStyle, resolve_widget_visual_tokens,
    },
};

pub(in crate::runtime) fn push_container_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    node_id: NodeId,
    layout: &LayoutOutput,
    theme: &ThemeTokens,
    style: WidgetStyle,
    state: WidgetState,
) {
    let Some(bounds) = layout.rects.get(&node_id).copied() else {
        return;
    };
    let base_tokens = resolve_widget_visual_tokens(theme, style, WidgetState::default());
    let tokens = if state.hovered {
        base_tokens
    } else {
        resolve_widget_visual_tokens(theme, style, state)
    };
    let fill = if state.hovered {
        blend_color(
            base_tokens.fill,
            theme.surface_overlay,
            theme.state_hover_strong,
        )
    } else {
        tokens.fill
    };
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: node_id,
        rect: bounds,
        color: fill,
    }));
    if state.hovered {
        let marker_height = (bounds.height() - 16.0).max(8.0).min(bounds.height());
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: node_id,
            rect: Rect::from_min_size(
                Point::new(
                    bounds.min.x + 1.0,
                    bounds.min.y + (bounds.height() - marker_height) * 0.5,
                ),
                crate::gui::types::Vector2::new(3.0, marker_height),
            ),
            color: theme.accent_danger,
        }));
    }
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id: node_id,
        rect: bounds,
        color: tokens.border,
        width: 1.0,
    }));
}

pub(crate) fn push_overlay_panel_with_environment(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    rect: Rect,
    label: Option<super::PaintText>,
    theme: &ThemeTokens,
    style: WidgetStyle,
    environment: &crate::runtime::ResolvedEnvironment,
) {
    let metrics = DeclaredTextMetrics::new(
        WidgetSizing::fixed(Vector2::new(0.0, 0.0)),
        text_font_size_for_height(24.0),
        Vector2::new(10.0, 3.0),
    )
    .resolve(environment, TextScaleParticipation::Scaled);
    let mut state = WidgetState {
        active: true,
        ..WidgetState::default()
    };
    if label.is_none() {
        state.selected = true;
    }
    let tokens = resolve_widget_visual_tokens(theme, style, state);
    if label.is_some() {
        let shadow = Rect::from_min_max(
            Point::new(rect.min.x + 4.0, rect.min.y + 6.0),
            Point::new(rect.max.x + 4.0, rect.max.y + 6.0),
        );
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id,
            rect: shadow,
            color: crate::gui::types::Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 96,
            },
        }));
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id,
            rect,
            color: blend_color(tokens.fill, theme.surface_overlay, 0.30),
        }));
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id,
            rect,
            color: tokens.emphasis,
            width: 1.0,
        }));
        if let Some(label) = label {
            let text_rect = inset_rect(rect, metrics.insets.x, metrics.insets.y);
            push_text_run(
                primitives,
                PaintTextRun {
                    widget_id,
                    text: label,
                    rect: text_rect,
                    baseline: super::text::optical_centered_baseline(text_rect, metrics.font_size),
                    color: theme.text_primary,
                    align: PaintTextAlign::Left,
                    wrap: TextWrap::None,
                    font_size: metrics.font_size,
                },
            );
        }
    } else {
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id,
            rect,
            color: tokens.emphasis,
        }));
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn push_tooltip_panel_with_environment(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    rect: Rect,
    lines: &[String],
    theme: &ThemeTokens,
    font_size: f32,
    line_height: f32,
    environment: &crate::runtime::ResolvedEnvironment,
) {
    let metrics = DeclaredTextMetrics::new(
        WidgetSizing::fixed(Vector2::new(0.0, 0.0)),
        font_size,
        Vector2::new(8.0, 4.0),
    )
    .resolve(environment, TextScaleParticipation::Scaled);
    let scale = metrics.font_size / font_size.max(f32::MIN_POSITIVE);
    let line_height = line_height * scale;
    let shadow = Rect::from_min_max(
        Point::new(rect.min.x + 2.0, rect.min.y + 3.0),
        Point::new(rect.max.x + 2.0, rect.max.y + 3.0),
    );
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect: shadow,
        color: crate::gui::types::Rgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 88,
        },
    }));
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color: theme.surface_overlay.blend_toward(theme.bg_primary, 0.18),
    }));
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect,
        color: theme.border_emphasis,
        width: 1.0,
    }));

    let text_rect = inset_rect(rect, metrics.insets.x, metrics.insets.y);
    for (index, line) in lines.iter().enumerate() {
        let line_rect = Rect::from_min_size(
            Point::new(
                text_rect.min.x,
                text_rect.min.y + index as f32 * line_height,
            ),
            Vector2::new(text_rect.width(), line_height),
        );
        push_text_run(
            primitives,
            PaintTextRun {
                widget_id,
                text: super::PaintText::from(line.as_str()),
                rect: line_rect,
                baseline: optical_centered_baseline(line_rect, metrics.font_size),
                color: theme.text_primary,
                align: PaintTextAlign::Left,
                wrap: TextWrap::None,
                font_size: metrics.font_size,
            },
        );
    }
}
