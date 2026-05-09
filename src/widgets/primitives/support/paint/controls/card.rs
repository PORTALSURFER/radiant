//! Card paint command generation.

use super::super::chrome::push_control_chrome;
use crate::gui::types::Rect;
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;
use crate::widgets::primitives::card::CardWidget;

pub(in crate::widgets::primitives) fn push_card_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    card: &CardWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &card.common, bounds, theme);
}
