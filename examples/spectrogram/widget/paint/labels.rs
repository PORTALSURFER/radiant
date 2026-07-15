use super::primitives::push_text;
use radiant::gui::visualization::VerticalValueAxis;
use radiant::prelude::*;
use radiant::runtime::PaintTextAlign;

pub(super) fn append_labels(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    plot: Rect,
    frame: u64,
    theme: &ThemeTokens,
) {
    for (label, ratio) in [
        ("18k", 1.0),
        ("8k", 0.78),
        ("2k", 0.55),
        ("500", 0.32),
        ("40", 0.0),
    ] {
        append_frequency_label(primitives, widget_id, plot, label, ratio, theme);
    }
    push_text(
        primitives,
        widget_id,
        format!("frame {frame}"),
        Rect::from_min_max(
            Point::new(plot.max.x - 118.0, plot.max.y + 8.0),
            Point::new(plot.max.x, plot.max.y + 28.0),
        ),
        theme.text_muted,
        PaintTextAlign::Right,
    );
    push_text(
        primitives,
        widget_id,
        "synthetic realtime spectrum",
        Rect::from_min_max(
            Point::new(plot.min.x, plot.max.y + 8.0),
            Point::new(plot.min.x + 180.0, plot.max.y + 28.0),
        ),
        theme.text_muted,
        PaintTextAlign::Left,
    );
}

fn append_frequency_label(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    plot: Rect,
    label: &'static str,
    ratio: f32,
    theme: &ThemeTokens,
) {
    let y = VerticalValueAxis::new(plot, 0.0, 1.0).y_for_value(ratio);
    push_text(
        primitives,
        widget_id,
        label,
        Rect::from_min_max(
            Point::new(plot.min.x - 44.0, y - 9.0),
            Point::new(plot.min.x - 6.0, y + 11.0),
        ),
        theme.text_muted,
        PaintTextAlign::Right,
    );
}
