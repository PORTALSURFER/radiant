use super::{push_rect, push_text};
use crate::widget::{EqEditorWidget, x_for_freq, y_for_gain};
use radiant::prelude::*;

pub(super) fn append_grid(
    widget: &EqEditorWidget,
    primitives: &mut Vec<PaintPrimitive>,
    plot: Rect,
    theme: &ThemeTokens,
) {
    for freq in [
        20.0, 50.0, 100.0, 200.0, 500.0, 1_000.0, 2_000.0, 5_000.0, 10_000.0, 20_000.0,
    ] {
        append_frequency_line(widget, primitives, plot, theme, freq);
    }
    append_gain_grid(widget, primitives, plot, theme);
    append_frequency_labels(widget, primitives, plot, theme);
}

fn append_frequency_line(
    widget: &EqEditorWidget,
    primitives: &mut Vec<PaintPrimitive>,
    plot: Rect,
    theme: &ThemeTokens,
    freq: f32,
) {
    let x = x_for_freq(plot, freq);
    let color = if freq == 1_000.0 {
        theme.grid_strong
    } else {
        theme.grid_soft
    };
    push_rect(
        primitives,
        widget.common.id,
        Rect::from_min_max(Point::new(x, plot.min.y), Point::new(x + 1.0, plot.max.y)),
        color,
    );
}

fn append_gain_grid(
    widget: &EqEditorWidget,
    primitives: &mut Vec<PaintPrimitive>,
    plot: Rect,
    theme: &ThemeTokens,
) {
    for gain in [-24.0, -12.0, 0.0, 12.0, 24.0] {
        append_gain_line(widget, primitives, plot, theme, gain);
    }
}

fn append_gain_line(
    widget: &EqEditorWidget,
    primitives: &mut Vec<PaintPrimitive>,
    plot: Rect,
    theme: &ThemeTokens,
    gain: f32,
) {
    let y = y_for_gain(plot, gain);
    let color = if gain == 0.0 {
        theme.grid_strong
    } else {
        theme.grid_soft
    };
    push_rect(
        primitives,
        widget.common.id,
        Rect::from_min_max(Point::new(plot.min.x, y), Point::new(plot.max.x, y + 1.0)),
        color,
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
        append_frequency_label(widget, primitives, plot, theme, label, freq);
    }
}

fn append_frequency_label(
    widget: &EqEditorWidget,
    primitives: &mut Vec<PaintPrimitive>,
    plot: Rect,
    theme: &ThemeTokens,
    label: &'static str,
    freq: f32,
) {
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
