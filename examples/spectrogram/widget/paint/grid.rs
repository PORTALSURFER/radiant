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
    let axis = VerticalValueAxis::new(plot, 0.0, 1.0);
    for fraction in [0.25, 0.5, 0.75] {
        let y = axis.y_for_value(fraction);
        if let Some(line) = horizontal_line_rect(plot, y, 1.0) {
            push_rect(primitives, widget_id, line, theme.grid_soft.with_alpha(150));
        }
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
        if let Some(line) = vertical_line_rect(plot, x, 1.0) {
            push_rect(primitives, widget_id, line, theme.grid_soft.with_alpha(120));
        }
    }
}
