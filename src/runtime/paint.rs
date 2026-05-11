//! Backend-neutral paint plans emitted from generic Radiant surfaces.

mod primitives;
mod scroll;
mod text;

pub use primitives::*;
pub(super) use scroll::{
    push_scroll_affordance, resolve_scroll_affordance, scroll_content_clip_rect,
};
pub(crate) use text::{
    button_font_size, input_font_size, optical_centered_baseline, push_text_run, text_font_size,
};

use crate::{
    gui::types::{Point, Rect, Rgba8},
    layout::{LayoutOutput, NodeId},
    theme::ThemeTokens,
    widgets::{TextWrap, WidgetId, WidgetState, WidgetStyle, resolve_widget_visual_tokens},
};

pub(super) fn push_clip_start(primitives: &mut Vec<PaintPrimitive>, node_id: NodeId, rect: Rect) {
    primitives.push(PaintPrimitive::ClipStart(PaintClipStart { node_id, rect }));
}

pub(super) fn push_clip_end(primitives: &mut Vec<PaintPrimitive>, node_id: NodeId) {
    primitives.push(PaintPrimitive::ClipEnd(PaintClipEnd { node_id }));
}

pub(super) fn push_container_chrome(
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

pub(super) fn push_overlay_panel(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    rect: Rect,
    label: Option<PaintText>,
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
            color: Rgba8 {
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
                    baseline: optical_centered_baseline(
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

pub(crate) fn blend_color(from: Rgba8, to: Rgba8, amount: f32) -> Rgba8 {
    let amount = amount.clamp(0.0, 1.0);
    Rgba8 {
        r: blend_channel(from.r, to.r, amount),
        g: blend_channel(from.g, to.g, amount),
        b: blend_channel(from.b, to.b, amount),
        a: blend_channel(from.a, to.a, amount),
    }
}

fn blend_channel(from: u8, to: u8, amount: f32) -> u8 {
    ((from as f32) + (((to as f32) - (from as f32)) * amount)).round() as u8
}

pub(crate) fn diagonal_cut_rect_points(rect: Rect) -> PaintPointList {
    let cut = (rect.height().min(rect.width()) * 0.18).clamp(4.0, 8.0);
    [
        Point::new(rect.min.x, rect.min.y),
        Point::new(rect.max.x, rect.min.y),
        Point::new(rect.max.x, rect.max.y - cut),
        Point::new(rect.max.x - cut, rect.max.y),
        Point::new(rect.min.x, rect.max.y),
    ]
    .into()
}

pub(crate) fn push_axis_stroke(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    color: Rgba8,
    horizontal: bool,
) {
    let rect = if horizontal {
        Rect::from_min_max(
            Point::new(bounds.min.x, bounds.min.y + bounds.height() * 0.5),
            Point::new(bounds.max.x, bounds.min.y + bounds.height() * 0.5),
        )
    } else {
        Rect::from_min_max(
            Point::new(bounds.min.x + bounds.width() * 0.5, bounds.min.y),
            Point::new(bounds.min.x + bounds.width() * 0.5, bounds.max.y),
        )
    };
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect,
        color,
        width: 1.0,
    }));
}

pub(crate) fn inset_rect(rect: Rect, x: f32, y: f32) -> Rect {
    Rect::from_min_max(
        Point::new(rect.min.x + x, rect.min.y + y),
        Point::new(rect.max.x - x, rect.max.y - y),
    )
}
