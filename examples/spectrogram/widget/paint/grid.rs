use super::color::translucent;
use super::primitives::push_rect;
use radiant::prelude::*;

pub(super) fn append_grid(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    plot: Rect,
    theme: &ThemeTokens,
) {
    append_horizontal_lines(primitives, widget_id, plot, theme);
    append_vertical_lines(primitives, widget_id, plot, theme);
}

fn append_horizontal_lines(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    plot: Rect,
    theme: &ThemeTokens,
) {
    for fraction in [0.25, 0.5, 0.75] {
        let y = plot.y_for_ratio_from_bottom(fraction);
        push_rect(
            primitives,
            widget_id,
            Rect::from_min_max(Point::new(plot.min.x, y), Point::new(plot.max.x, y + 1.0)),
            translucent(theme.grid_soft, 150),
        );
    }
}

fn append_vertical_lines(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    plot: Rect,
    theme: &ThemeTokens,
) {
    for fraction in [0.25, 0.5, 0.75] {
        let x = plot.x_for_ratio(fraction);
        push_rect(
            primitives,
            widget_id,
            Rect::from_min_max(Point::new(x, plot.min.y), Point::new(x + 1.0, plot.max.y)),
            translucent(theme.grid_soft, 120),
        );
    }
}
