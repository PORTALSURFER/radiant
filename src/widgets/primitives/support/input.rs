//! Input utility helpers shared by primitive widgets.

use super::super::scrollbar::ScrollbarAxis;
use crate::widgets::interaction::WidgetKey;

pub(in crate::widgets::primitives) fn activate_on_keyboard(key: WidgetKey) -> bool {
    matches!(key, WidgetKey::Enter | WidgetKey::Space)
}

pub(in crate::widgets::primitives) fn clamp_fraction(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

pub(in crate::widgets::primitives) fn leading_arrow_for_axis(axis: ScrollbarAxis) -> WidgetKey {
    match axis {
        ScrollbarAxis::Horizontal => WidgetKey::ArrowLeft,
        ScrollbarAxis::Vertical => WidgetKey::ArrowUp,
    }
}

pub(in crate::widgets::primitives) fn trailing_arrow_for_axis(axis: ScrollbarAxis) -> WidgetKey {
    match axis {
        ScrollbarAxis::Horizontal => WidgetKey::ArrowRight,
        ScrollbarAxis::Vertical => WidgetKey::ArrowDown,
    }
}
