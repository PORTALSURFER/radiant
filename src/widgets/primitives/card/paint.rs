//! Card paint command generation.

use crate::gui::types::Rect;
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;
use crate::widgets::primitives::{card::CardWidget, support::push_control_chrome};

pub(super) fn push_card_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    card: &CardWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    push_control_chrome(primitives, &card.common, bounds, theme);
}
