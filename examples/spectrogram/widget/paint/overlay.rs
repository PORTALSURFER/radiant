use super::color::translucent;
use super::primitives::push_rect;
use radiant::prelude::*;

pub(super) fn append_hover(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    plot: Rect,
    x: f32,
    theme: &ThemeTokens,
) {
    if let Some(line) = vertical_line_rect(plot, x, 2.0) {
        push_rect(
            primitives,
            widget_id,
            line,
            translucent(theme.accent_mint, 180),
        );
    }
}
