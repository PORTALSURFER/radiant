use super::*;

#[path = "paint/grid.rs"]
mod grid;
#[path = "paint/primitives.rs"]
mod primitives;

use grid::append_grid;
pub(super) use primitives::push_rect;
use primitives::{push_fill_polygon, push_stroke, push_text};

pub(super) fn append_eq_paint(
    widget: &EqEditorWidget,
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let plot = widget.plot_rect(bounds);
    push_rect(primitives, widget.common.id, bounds, theme.bg_secondary);
    push_rect(primitives, widget.common.id, plot, theme.surface_base);
    push_stroke(
        primitives,
        widget.common.id,
        plot,
        theme.border_emphasis,
        1.0,
    );
    append_grid(widget, primitives, plot, theme);
    if widget.analyzer {
        append_analyzer(widget, primitives, plot, theme);
    }
    append_curve(widget, primitives, plot, theme);
    append_band_handles(widget, primitives, plot, theme);
}

fn append_analyzer(
    widget: &EqEditorWidget,
    primitives: &mut Vec<PaintPrimitive>,
    plot: Rect,
    theme: &ThemeTokens,
) {
    let floor = plot.max.y;
    let axis = HorizontalValueAxis::normalized(plot);
    let points = (0..96)
        .map(|index| {
            let ratio = index as f32 / 95.0;
            let wave = ((ratio * 5.8).sin() * 0.45 + (ratio * 18.0).sin() * 0.12).max(-0.9);
            let height = plot.height() * (0.18 + (1.0 - ratio).powf(0.7) * 0.32 + wave * 0.08);
            Point::new(axis.x_for_value(ratio), floor - height)
        })
        .chain([Point::new(plot.max.x, floor), Point::new(plot.min.x, floor)]);
    push_fill_polygon(
        primitives,
        widget.common.id,
        points,
        theme.highlight_blue.with_alpha(46),
    );
}

fn append_curve(
    widget: &EqEditorWidget,
    primitives: &mut Vec<PaintPrimitive>,
    plot: Rect,
    theme: &ThemeTokens,
) {
    let axis = HorizontalValueAxis::normalized(plot);
    push_sampled_curve_stroke(
        primitives,
        SampledCurveStrokeParts::new(widget.common.id, plot, 159, theme.accent_mint, 3.0),
        |ratio| {
            let freq = geometry::freq_for_ratio(ratio);
            Some(Point::new(
                axis.x_for_value(ratio),
                y_for_gain(plot, response_gain_db(&widget.bands, freq)),
            ))
        },
    );
}

fn append_band_handles(
    widget: &EqEditorWidget,
    primitives: &mut Vec<PaintPrimitive>,
    plot: Rect,
    theme: &ThemeTokens,
) {
    for band in &widget.bands {
        let center = widget.handle_center(plot, *band);
        let active = band.id == widget.selected_band || Some(band.id) == widget.hover_band;
        let fill = if !band.enabled {
            theme.text_muted
        } else if active {
            theme.accent_mint
        } else {
            theme.surface_raised
        };
        let rect = Rect::from_min_size(
            Point::new(center.x - HANDLE_SIZE * 0.5, center.y - HANDLE_SIZE * 0.5),
            Vector2::new(HANDLE_SIZE, HANDLE_SIZE),
        );
        push_rect(primitives, widget.common.id, rect, fill);
        push_stroke(primitives, widget.common.id, rect, theme.text_primary, 1.0);
        push_text(
            primitives,
            widget.common.id,
            band.id.to_string(),
            Rect::from_min_max(
                Point::new(center.x - 14.0, center.y - 25.0),
                Point::new(center.x + 14.0, center.y - 7.0),
            ),
            theme.text_primary,
            PaintTextAlign::Center,
        );
    }
}
