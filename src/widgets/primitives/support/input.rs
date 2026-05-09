//! Input utility helpers shared by primitive widgets.

use crate::widgets::interaction::WidgetKey;

pub(in crate::widgets::primitives) fn activate_on_keyboard(key: WidgetKey) -> bool {
    matches!(key, WidgetKey::Enter | WidgetKey::Space)
}

pub(in crate::widgets::primitives) fn clamp_fraction(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}
