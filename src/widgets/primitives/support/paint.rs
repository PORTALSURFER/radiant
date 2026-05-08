//! Paint projection helpers for primitive widget implementations.

use crate::gui::types::{Point, Rect, Vector2};
use crate::runtime::{
    PaintCustomSurface, PaintFillPolygon, PaintFillRect, PaintImage, PaintPrimitive,
    PaintStrokePolygon, PaintStrokePolyline, PaintStrokeRect, PaintTextAlign, PaintTextInput,
    blend_color, button_font_size, diagonal_cut_rect_points, input_font_size, inset_rect,
    optical_centered_baseline, push_axis_stroke, push_text_run, text_font_size,
};
use crate::theme::ThemeTokens;
use std::sync::Arc;

use super::super::{
    badge::BadgeWidget, button::ButtonWidget, canvas::CanvasWidget, card::CardWidget,
    drag_handle::DragHandleWidget, image::ImageWidget, list_item::ListItemWidget,
    scrollbar::ScrollbarAxis, scrollbar::ScrollbarWidget, selectable::SelectableWidget,
    text::TextWidget, text::TextWrap, text_input::TextInputWidget, toggle::ToggleWidget,
};
use super::common::WidgetCommon;
use crate::widgets::contract::{WidgetId, WidgetState};

pub(in crate::widgets::primitives) fn push_text_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    text: &TextWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_text_run(
        primitives,
        text.common.id,
        text.text.clone(),
        bounds,
        optical_centered_baseline(bounds, text_font_size(bounds)),
        theme.text_primary,
        PaintTextAlign::Left,
        text.wrap,
        text_font_size(bounds),
    );
}

fn push_button_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    common: &WidgetCommon,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = crate::widgets::resolve_widget_visual_tokens(theme, common.style, common.state);
    let points = diagonal_cut_rect_points(bounds);
    primitives.push(PaintPrimitive::FillPolygon(PaintFillPolygon {
        widget_id: common.id,
        points: points.clone(),
        color: tokens.fill,
    }));
    primitives.push(PaintPrimitive::StrokePolygon(PaintStrokePolygon {
        widget_id: common.id,
        points,
        color: tokens.border,
        width: 1.0,
    }));
    if common.state.focused && common.paint.paints_focus {
        primitives.push(PaintPrimitive::StrokePolygon(PaintStrokePolygon {
            widget_id: common.id,
            points: diagonal_cut_rect_points(inset_rect(bounds, -1.0, -1.0)),
            color: tokens.emphasis,
            width: 1.0,
        }));
    }
}

fn push_control_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    common: &WidgetCommon,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = crate::widgets::resolve_widget_visual_tokens(theme, common.style, common.state);
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: common.id,
        rect: bounds,
        color: tokens.fill,
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
}

pub(in crate::widgets::primitives) fn push_button_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    button: &ButtonWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_button_chrome(primitives, &button.common, bounds, theme);
    push_text_run(
        primitives,
        button.common.id,
        button.props.label.clone(),
        inset_rect(bounds, 8.0, 4.0),
        optical_centered_baseline(inset_rect(bounds, 8.0, 4.0), button_font_size(bounds)),
        crate::widgets::resolve_widget_visual_tokens(
            theme,
            button.common.style,
            button.common.state,
        )
        .foreground,
        PaintTextAlign::Center,
        TextWrap::None,
        button_font_size(bounds),
    );
}

fn push_checkbox_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: WidgetId,
    bounds: Rect,
    theme: &ThemeTokens,
    state: WidgetState,
    checked: bool,
) {
    let side = bounds.width().min(bounds.height()).max(0.0);
    let bounds = Rect::from_min_size(
        Point::new(
            bounds.min.x + (bounds.width() - side) * 0.5,
            bounds.min.y + (bounds.height() - side) * 0.5,
        ),
        Vector2::new(side, side),
    );
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect: bounds,
        color: if state.pressed {
            theme.bg_tertiary
        } else if state.hovered {
            theme.surface_raised
        } else {
            theme.surface_base
        },
    }));
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect: bounds,
        color: if checked || state.pressed || state.hovered {
            theme.accent_danger
        } else {
            theme.border_emphasis
        },
        width: 1.0,
    }));
    if checked {
        primitives.push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
            widget_id,
            points: vec![
                Point::new(bounds.min.x + side * 0.25, bounds.min.y + side * 0.55),
                Point::new(bounds.min.x + side * 0.43, bounds.min.y + side * 0.72),
                Point::new(bounds.min.x + side * 0.76, bounds.min.y + side * 0.30),
            ],
            color: theme.accent_danger,
            width: 2.0,
        }));
    }
}

pub(in crate::widgets::primitives) fn push_toggle_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    toggle: &ToggleWidget,
    bounds: Rect,
    theme: &ThemeTokens,
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
        push_text_run(
            primitives,
            toggle.common.id,
            toggle.props.label.clone(),
            inset_rect(bounds, 8.0, 4.0),
            optical_centered_baseline(inset_rect(bounds, 8.0, 4.0), text_font_size(bounds)),
            tokens.foreground,
            PaintTextAlign::Left,
            TextWrap::None,
            text_font_size(bounds),
        );
    }
}

fn push_text_input_chrome(
    primitives: &mut Vec<PaintPrimitive>,
    common: &WidgetCommon,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = crate::widgets::resolve_widget_visual_tokens(theme, common.style, common.state);
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
}

pub(in crate::widgets::primitives) fn push_text_input_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    input: &TextInputWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_text_input_chrome(primitives, &input.common, bounds, theme);
    let rect = inset_rect(bounds, 16.0, 4.0);
    let font_size = input_font_size(bounds);
    primitives.push(PaintPrimitive::TextInput(PaintTextInput {
        widget_id: input.common.id,
        rect,
        placeholder: input.props.placeholder.clone(),
        state: input.state.clone(),
        font_size,
        baseline: optical_centered_baseline(rect, font_size),
        color: crate::widgets::resolve_widget_visual_tokens(
            theme,
            input.common.style,
            input.common.state,
        )
        .foreground,
        placeholder_color: theme.text_muted,
        selection_color: theme.grid_strong,
        caret_color: theme.accent_danger,
        focused: input.common.state.focused,
    }));
}

pub(in crate::widgets::primitives) fn push_scrollbar_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    scrollbar: &ScrollbarWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let tokens = crate::widgets::resolve_widget_visual_tokens(
        theme,
        scrollbar.common.style,
        scrollbar.common.state,
    );
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: scrollbar.common.id,
        rect: bounds,
        color: theme.surface_base,
    }));
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: scrollbar.common.id,
        rect: scrollbar.thumb_rect(bounds),
        color: tokens.emphasis,
    }));
    push_axis_stroke(
        primitives,
        scrollbar.common.id,
        bounds,
        theme.grid_soft,
        scrollbar.props.axis == ScrollbarAxis::Horizontal,
    );
}

pub(in crate::widgets::primitives) fn push_drag_handle_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    handle: &DragHandleWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    if !handle.common.paint.paints_state_layers {
        return;
    }
    let tokens = crate::widgets::resolve_widget_visual_tokens(
        theme,
        handle.common.style,
        handle.common.state,
    );
    let color = if handle.common.state.pressed {
        theme.accent_danger
    } else if handle.common.state.hovered {
        tokens.emphasis
    } else {
        theme.text_muted
    };
    let center_y = bounds.min.y + bounds.height() * 0.5;
    for y in [center_y - 5.0, center_y, center_y + 5.0] {
        primitives.push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
            widget_id: handle.common.id,
            points: vec![
                Point::new(bounds.min.x + bounds.width() * 0.25, y),
                Point::new(bounds.max.x - bounds.width() * 0.25, y),
            ],
            color,
            width: if handle.common.state.pressed {
                2.0
            } else {
                1.25
            },
        }));
    }
    if handle.common.state.hovered || handle.common.state.pressed {
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: handle.common.id,
            rect: inset_rect(bounds, 2.0, 2.0),
            color,
            width: 1.0,
        }));
    }
}

pub(in crate::widgets::primitives) fn push_list_item_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    item: &ListItemWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &item.common, bounds, theme);
    push_text_run(
        primitives,
        item.common.id,
        item.label.clone(),
        inset_rect(bounds, 8.0, 3.0),
        optical_centered_baseline(inset_rect(bounds, 8.0, 3.0), text_font_size(bounds)),
        crate::widgets::resolve_widget_visual_tokens(theme, item.common.style, item.common.state)
            .foreground,
        PaintTextAlign::Left,
        TextWrap::None,
        text_font_size(bounds),
    );
    if let Some(detail) = &item.detail {
        push_text_run(
            primitives,
            item.common.id,
            detail.clone(),
            inset_rect(bounds, bounds.width() * 0.5, 3.0),
            optical_centered_baseline(
                inset_rect(bounds, bounds.width() * 0.5, 3.0),
                text_font_size(bounds),
            ),
            theme.text_muted,
            PaintTextAlign::Right,
            TextWrap::None,
            text_font_size(bounds),
        );
    }
}

pub(in crate::widgets::primitives) fn push_selectable_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    selectable: &SelectableWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &selectable.common, bounds, theme);
    push_text_run(
        primitives,
        selectable.common.id,
        selectable.props.label.clone(),
        inset_rect(bounds, 8.0, 3.0),
        optical_centered_baseline(inset_rect(bounds, 8.0, 3.0), text_font_size(bounds)),
        crate::widgets::resolve_widget_visual_tokens(
            theme,
            selectable.common.style,
            selectable.common.state,
        )
        .foreground,
        PaintTextAlign::Left,
        TextWrap::None,
        text_font_size(bounds),
    );
}

pub(in crate::widgets::primitives) fn push_badge_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    badge: &BadgeWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &badge.common, bounds, theme);
    push_text_run(
        primitives,
        badge.common.id,
        badge.props.label.clone(),
        inset_rect(bounds, 8.0, 3.0),
        optical_centered_baseline(inset_rect(bounds, 8.0, 3.0), button_font_size(bounds)),
        crate::widgets::resolve_widget_visual_tokens(theme, badge.common.style, badge.common.state)
            .foreground,
        PaintTextAlign::Center,
        TextWrap::None,
        button_font_size(bounds),
    );
}

pub(in crate::widgets::primitives) fn push_card_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    card: &CardWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &card.common, bounds, theme);
}

pub(in crate::widgets::primitives) fn push_image_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    image: &ImageWidget,
    bounds: Rect,
) {
    primitives.push(PaintPrimitive::Image(PaintImage {
        widget_id: image.common.id,
        source_rect: None,
        rect: bounds,
        image: Arc::clone(&image.props.image),
    }));
}

pub(in crate::widgets::primitives) fn push_canvas_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    canvas: &CanvasWidget,
    bounds: Rect,
) {
    primitives.push(PaintPrimitive::CustomSurface(PaintCustomSurface {
        widget_id: canvas.common.id,
        rect: bounds,
        bounds: canvas.common.paint.bounds,
        retained: canvas.retained,
    }));
}
