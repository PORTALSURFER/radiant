use super::chrome::{push_button_chrome, push_checkbox_chrome, push_control_chrome};
use crate::gui::types::{Point, Rect};
use crate::runtime::{
    PaintFillRect, PaintPrimitive, PaintStrokePolyline, PaintStrokeRect, PaintTextAlign,
    button_font_size, inset_rect, optical_centered_baseline, push_axis_stroke, push_text_run,
    text_font_size,
};
use crate::theme::ThemeTokens;

use super::super::super::{
    badge::BadgeWidget, button::ButtonWidget, card::CardWidget, drag_handle::DragHandleWidget,
    list_item::ListItemWidget, scrollbar::ScrollbarAxis, scrollbar::ScrollbarWidget,
    selectable::SelectableWidget, text::TextWidget, text::TextWrap, toggle::ToggleWidget,
};

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
