use super::{
    PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintTextAlign, PaintTextRun,
    geometry::{blend_color, inset_rect},
    text::{push_text_run, text_font_size},
};
use crate::{
    gui::types::{Point, Rect},
    layout::{LayoutOutput, NodeId},
    theme::ThemeTokens,
    widgets::{TextWrap, WidgetId, WidgetState, WidgetStyle, resolve_widget_visual_tokens},
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

pub(in crate::runtime) fn push_overlay_panel(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    rect: Rect,
    label: Option<super::PaintText>,
    theme: &ThemeTokens,
    style: WidgetStyle,
) {
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
            push_text_run(
                primitives,
                PaintTextRun {
                    widget_id,
                    text: label,
                    rect: inset_rect(rect, 48.0, 4.0),
                    baseline: super::text::optical_centered_baseline(
                        inset_rect(rect, 48.0, 4.0),
                        text_font_size(rect),
                    ),
                    color: theme.text_primary,
                    align: PaintTextAlign::Left,
                    wrap: TextWrap::None,
                    font_size: text_font_size(rect),
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
