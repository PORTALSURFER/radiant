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
    push_rect(
        primitives,
        widget_id,
        Rect::from_min_max(Point::new(x, plot.min.y), Point::new(x + 2.0, plot.max.y)),
        translucent(theme.accent_mint, 180),
    );
}
