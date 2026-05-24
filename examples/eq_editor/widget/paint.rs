use super::*;
use radiant::runtime::{PaintFillPolygon, PaintStrokePolyline};
use std::sync::Arc;

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

fn append_grid(
    widget: &EqEditorWidget,
    primitives: &mut Vec<PaintPrimitive>,
    plot: Rect,
    theme: &ThemeTokens,
) {
    for freq in [
        20.0, 50.0, 100.0, 200.0, 500.0, 1_000.0, 2_000.0, 5_000.0, 10_000.0, 20_000.0,
    ] {
        let x = x_for_freq(plot, freq);
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(Point::new(x, plot.min.y), Point::new(x + 1.0, plot.max.y)),
            if freq == 1_000.0 {
                theme.grid_strong
            } else {
                theme.grid_soft
            },
        );
    }
    append_gain_grid(widget, primitives, plot, theme);
    append_frequency_labels(widget, primitives, plot, theme);
}

fn append_gain_grid(
    widget: &EqEditorWidget,
    primitives: &mut Vec<PaintPrimitive>,
    plot: Rect,
    theme: &ThemeTokens,
) {
    for gain in [-24.0, -12.0, 0.0, 12.0, 24.0] {
        let y = y_for_gain(plot, gain);
        push_rect(
            primitives,
            widget.common.id,
            Rect::from_min_max(Point::new(plot.min.x, y), Point::new(plot.max.x, y + 1.0)),
            if gain == 0.0 {
                theme.grid_strong
            } else {
                theme.grid_soft
            },
        );
        push_text(
            primitives,
            widget.common.id,
            format!("{gain:+.0}"),
            Rect::from_min_max(
                Point::new(plot.min.x - 42.0, y - 9.0),
                Point::new(plot.min.x - 6.0, y + 12.0),
            ),
            theme.text_muted,
            PaintTextAlign::Right,
        );
    }
}

fn append_frequency_labels(
    widget: &EqEditorWidget,
    primitives: &mut Vec<PaintPrimitive>,
    plot: Rect,
    theme: &ThemeTokens,
) {
    for (label, freq) in [
        ("20", 20.0),
        ("100", 100.0),
        ("1k", 1_000.0),
        ("10k", 10_000.0),
        ("20k", 20_000.0),
    ] {
        let x = x_for_freq(plot, freq);
        push_text(
            primitives,
            widget.common.id,
            label,
            Rect::from_min_max(
                Point::new(x - 22.0, plot.max.y + 8.0),
                Point::new(x + 22.0, plot.max.y + 28.0),
            ),
            theme.text_muted,
            PaintTextAlign::Center,
        );
    }
}

fn append_analyzer(
    widget: &EqEditorWidget,
    primitives: &mut Vec<PaintPrimitive>,
    plot: Rect,
    theme: &ThemeTokens,
) {
    let floor = plot.max.y;
    let points = (0..96)
        .map(|index| {
            let ratio = index as f32 / 95.0;
            let x = plot.min.x + plot.width() * ratio;
            let wave = ((ratio * 5.8).sin() * 0.45 + (ratio * 18.0).sin() * 0.12).max(-0.9);
            let height = plot.height() * (0.18 + (1.0 - ratio).powf(0.7) * 0.32 + wave * 0.08);
            Point::new(x, floor - height)
        })
        .chain([Point::new(plot.max.x, floor), Point::new(plot.min.x, floor)])
        .collect::<Vec<_>>();
    primitives.push(PaintPrimitive::FillPolygon(PaintFillPolygon {
        widget_id: widget.common.id,
        points: Arc::from(points),
        color: translucent(theme.highlight_blue, 46),
    }));
}

fn append_curve(
    widget: &EqEditorWidget,
    primitives: &mut Vec<PaintPrimitive>,
    plot: Rect,
    theme: &ThemeTokens,
) {
    let points = (0..160)
        .map(|index| {
            let ratio = index as f32 / 159.0;
            let freq = freq_for_ratio(ratio);
            Point::new(
                plot.min.x + plot.width() * ratio,
                y_for_gain(plot, response_gain_db(&widget.bands, freq)),
            )
        })
        .collect::<Vec<_>>();
    primitives.push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
        widget_id: widget.common.id,
        points: Arc::from(points),
        color: theme.accent_mint,
        width: 3.0,
    }));
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

pub(super) fn push_rect(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    rect: Rect,
    color: Rgba8,
) {
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color,
    }));
}

fn push_stroke(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    rect: Rect,
    color: Rgba8,
    width: f32,
) {
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect,
        color,
        width,
    }));
}

fn push_text(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    text: impl Into<String>,
    rect: Rect,
    color: Rgba8,
    align: PaintTextAlign,
) {
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text: text.into().into(),
        rect,
        font_size: 12.0,
        baseline: Some(16.0),
        color,
        align,
        wrap: TextWrap::None,
    }));
}
