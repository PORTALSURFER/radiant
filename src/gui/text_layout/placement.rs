use crate::gui::{
    text_layout::{TextLineInsets, TextLineMode},
    types::{Point, Rect},
};

pub(super) fn compute_text_line(
    bounds: Rect,
    font_size: f32,
    insets: TextLineInsets,
    min_top_inset: f32,
    mode: TextLineMode,
) -> Rect {
    let inner = inset_rect(bounds, insets);
    if inner.width() <= 0.0 || inner.height() <= 0.0 {
        return bounds.empty_at_min();
    }
    let line_height = font_size.max(1.0);
    let line = match mode {
        TextLineMode::Center => {
            let min_y = inner.min.y + ((inner.height() - line_height) * 0.5).max(0.0);
            Rect::from_min_max(
                Point::new(inner.min.x, min_y),
                Point::new(inner.max.x, min_y + line_height),
            )
        }
        TextLineMode::Top => Rect::from_min_max(
            inner.min,
            Point::new(inner.max.x, inner.min.y + line_height),
        ),
    };
    let line = clamp_min_top(line, inner, min_top_inset.max(0.0));
    line.clamp_to(inner)
}

/// Snap a text-line rectangle so its bottom edge lands on a full pixel.
///
/// This keeps repeated text rows from alternating between adjacent raster rows
/// when compact density tokens produce fractional line positions.
pub fn snap_text_baseline_to_pixel(line: Rect) -> Rect {
    let height = line.height().max(0.0);
    if height <= 0.0 {
        return line;
    }
    let baseline = (line.min.y + height).round();
    let min_y = baseline - height;
    Rect::from_min_max(
        Point::new(line.min.x, min_y),
        Point::new(line.max.x, baseline),
    )
}

fn clamp_min_top(line: Rect, bounds: Rect, min_top_inset: f32) -> Rect {
    let min_y = (bounds.min.y + min_top_inset).min(bounds.max.y);
    if line.min.y >= min_y {
        return line;
    }
    Rect::from_min_max(
        Point::new(line.min.x, min_y),
        Point::new(
            line.max.x,
            (min_y + line.height().max(0.0)).min(bounds.max.y),
        ),
    )
}

fn inset_rect(rect: Rect, insets: TextLineInsets) -> Rect {
    let min_x = (rect.min.x + insets.left.max(0.0)).min(rect.max.x);
    let max_x = (rect.max.x - insets.right.max(0.0)).max(min_x);
    let min_y = (rect.min.y + insets.top.max(0.0)).min(rect.max.y);
    let max_y = (rect.max.y - insets.bottom.max(0.0)).max(min_y);
    Rect::from_min_max(Point::new(min_x, min_y), Point::new(max_x, max_y))
}
