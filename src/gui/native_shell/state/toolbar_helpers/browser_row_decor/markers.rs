use super::*;

/// Return the stroke width used for browser row borders at the current UI scale.
///
/// At `ui_scale == 1.0` this resolves to `1.0` logical px so row borders stay
/// visually consistent at 100% scale.
pub(in crate::gui::native_shell::state) fn browser_row_border_stroke(layout: &ShellLayout) -> f32 {
    layout.ui_scale.max(1.0)
}

/// Return x-advance reserved for the missing-file marker before a sample label.
pub(in crate::gui::native_shell::state) fn browser_missing_marker_advance(font_size: f32) -> f32 {
    (font_size * 1.05).max(7.0)
}

/// Return the inset left-edge marker rect used to flag locked browser rows.
///
/// The marker stays inside the row gutter before the numbering column. When a
/// focused row also renders a left focus border, `focused_left_border_width`
/// shifts the marker to the right so both accents remain visible.
pub(in crate::gui::native_shell::state) fn browser_locked_marker_rect(
    row_rect: Rect,
    sizing: SizingTokens,
    focused_left_border_width: f32,
) -> Option<Rect> {
    if row_rect.width() <= 0.0 || row_rect.height() <= 0.0 {
        return None;
    }
    let inset = sizing.row_corner_inset.max(1.0);
    let marker_width = (row_rect.height() * 0.22).clamp(4.0, 6.0);
    let min_x = row_rect.min.x + inset + focused_left_border_width.max(0.0);
    let max_x = (min_x + marker_width).min(row_rect.max.x - inset);
    let min_y = row_rect.min.y + inset;
    let max_y = row_rect.max.y - inset;
    if max_x <= min_x || max_y <= min_y {
        return None;
    }
    Some(Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(max_x, max_y),
    ))
}

/// Snap browser-row border bounds to the border stroke grid to avoid uneven AA
/// widths between top/bottom edges.
pub(in crate::gui::native_shell::state) fn browser_row_border_rect(
    rect: Rect,
    stroke: f32,
) -> Rect {
    let stroke = stroke.max(1.0);
    let snap = |value: f32| (value / stroke).round() * stroke;
    let min_x = snap(rect.min.x);
    let min_y = snap(rect.min.y);
    let max_x = snap(rect.max.x);
    let max_y = snap(rect.max.y);
    let snapped = Rect::from_min_max(Point::new(min_x, min_y), Point::new(max_x, max_y));
    if snapped.width() <= stroke * 2.0 || snapped.height() <= stroke * 2.0 {
        rect
    } else {
        snapped
    }
}
