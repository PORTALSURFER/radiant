//! Browser-row indicator, label, and border layout helpers.

use super::super::*;

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui::native_shell::state) struct BrowserRatingIndicatorLayout {
    pub(in crate::gui::native_shell::state) rects: [Rect; 3],
    pub(in crate::gui::native_shell::state) count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui::native_shell::state) struct BrowserRatingIndicatorAnchor {
    pub(in crate::gui::native_shell::state) sample_label: Rect,
    pub(in crate::gui::native_shell::state) label_origin_x: f32,
    pub(in crate::gui::native_shell::state) label_rendered_width: f32,
    pub(in crate::gui::native_shell::state) right_limit_x: f32,
}

pub(in crate::gui::native_shell::state) fn browser_rating_indicator_reserved_width(
    rating_level: i8,
    locked: bool,
    sizing: SizingTokens,
) -> f32 {
    let count = browser_rating_indicator_count(rating_level, locked);
    if count == 0 {
        return 0.0;
    }
    let width = browser_rating_indicator_unit_width(rating_level, locked, sizing);
    let gap = browser_rating_indicator_gap(sizing);
    let text_gap = browser_rating_indicator_text_gap(sizing);
    (count as f32 * width) + ((count.saturating_sub(1)) as f32 * gap) + text_gap
}

pub(in crate::gui::native_shell::state) fn browser_rating_indicator_layout(
    anchor: BrowserRatingIndicatorAnchor,
    rating_level: i8,
    locked: bool,
    sizing: SizingTokens,
) -> Option<BrowserRatingIndicatorLayout> {
    let count = browser_rating_indicator_count(rating_level, locked);
    let sample_label = anchor.sample_label;
    if count == 0 || sample_label.width() <= 0.0 || sample_label.height() <= 0.0 {
        return None;
    }
    let side = browser_rating_indicator_side(sizing).min(sample_label.height().max(1.0));
    let width = browser_rating_indicator_unit_width(rating_level, locked, sizing)
        .min(sample_label.width().max(1.0));
    let gap = browser_rating_indicator_gap(sizing);
    let total_width = (count as f32 * width) + ((count.saturating_sub(1)) as f32 * gap);
    let ideal_start_x = anchor.label_origin_x
        + anchor.label_rendered_width.max(0.0)
        + browser_rating_indicator_text_gap(sizing);
    let right_limit_x = anchor
        .right_limit_x
        .clamp(sample_label.min.x, sample_label.max.x);
    let max_start_x = (right_limit_x - total_width).max(sample_label.min.x);
    let start_x = ideal_start_x.clamp(sample_label.min.x, max_start_x);
    let min_y = sample_label.min.y + ((sample_label.height() - side) * 0.5).floor();
    let max_y = (min_y + side).min(sample_label.max.y);
    let mut rects = [Rect::from_min_max(sample_label.min, sample_label.min); 3];
    for (index, rect) in rects.iter_mut().take(count).enumerate() {
        let min_x = start_x + index as f32 * (width + gap);
        *rect = Rect::from_min_max(
            Point::new(min_x, min_y),
            Point::new((min_x + width).min(sample_label.max.x), max_y),
        );
    }
    Some(BrowserRatingIndicatorLayout { rects, count })
}

pub(in crate::gui::native_shell::state) fn browser_rating_indicator_color(
    style: &StyleTokens,
    rating_level: i8,
) -> Rgba8 {
    if rating_level < 0 {
        style.accent_trash
    } else {
        style.accent_mint
    }
}

pub(in crate::gui::native_shell::state) fn browser_rating_indicator_side(
    sizing: SizingTokens,
) -> f32 {
    (sizing.font_meta * 0.68).round().clamp(5.0, 8.0)
}

pub(in crate::gui::native_shell::state) fn browser_rating_indicator_count(
    rating_level: i8,
    locked: bool,
) -> usize {
    if locked && rating_level > 0 {
        1
    } else {
        rating_level.unsigned_abs().min(3) as usize
    }
}

pub(in crate::gui::native_shell::state) fn browser_rating_indicator_unit_width(
    rating_level: i8,
    locked: bool,
    sizing: SizingTokens,
) -> f32 {
    let side = browser_rating_indicator_side(sizing);
    if locked && rating_level > 0 {
        (side * 2.4).round().max(side + 2.0)
    } else {
        side
    }
}

pub(in crate::gui::native_shell::state) fn browser_rating_indicator_gap(
    sizing: SizingTokens,
) -> f32 {
    sizing.border_width.max(1.0) + 1.0
}

pub(in crate::gui::native_shell::state) fn browser_rating_indicator_text_gap(
    sizing: SizingTokens,
) -> f32 {
    sizing.text_inset_x.min(5.0).max(2.0)
}

/// Return width reserved for the inline browser metadata chip cluster plus its left gutter.
pub(in crate::gui::native_shell::state) fn browser_inline_tag_reserved_width(
    text: &str,
    sizing: SizingTokens,
) -> f32 {
    let labels: Vec<&str> = browser_inline_tag_labels(text).collect();
    if labels.is_empty() {
        return 0.0;
    }
    let chips_width: f32 = labels
        .iter()
        .map(|label| browser_inline_tag_chip_width(label, sizing))
        .sum();
    let chip_gap_count = labels.len().saturating_sub(1) as f32;
    chips_width
        + (chip_gap_count * browser_inline_tag_chip_gap(sizing))
        + browser_inline_tag_gap(sizing)
}

/// Approximate the rendered width of one inline browser metadata label.
pub(in crate::gui::native_shell::state) fn browser_inline_tag_text_width(
    text: &str,
    sizing: SizingTokens,
) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    ((text.chars().count() as f32) * (sizing.font_meta * 0.56).max(1.0)).ceil()
}

/// Return the horizontal gap between a sample label and its inline metadata label.
pub(in crate::gui::native_shell::state) fn browser_inline_tag_gap(sizing: SizingTokens) -> f32 {
    sizing.text_inset_x.min(6.0).max(3.0)
}

/// Split one inline browser metadata payload into stable per-chip labels.
pub(in crate::gui::native_shell::state) fn browser_inline_tag_labels(
    text: &str,
) -> impl Iterator<Item = &str> + '_ {
    text.split(" · ")
        .map(str::trim)
        .filter(|label| !label.is_empty())
}

/// Return the filled chip width needed for one inline browser metadata label.
pub(in crate::gui::native_shell::state) fn browser_inline_tag_chip_width(
    text: &str,
    sizing: SizingTokens,
) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    browser_inline_tag_text_width(text, sizing) + (browser_inline_tag_chip_padding_x(sizing) * 2.0)
}

/// Compute chip rects for one inline browser metadata cluster.
pub(in crate::gui::native_shell::state) fn browser_inline_tag_chip_rects(
    sample_label: Rect,
    text: &str,
    trailing_reserved_width: f32,
    sizing: SizingTokens,
) -> Vec<Rect> {
    if text.is_empty() || sample_label.width() <= 0.0 || sample_label.height() <= 0.0 {
        return Vec::new();
    }
    let labels: Vec<&str> = browser_inline_tag_labels(text).collect();
    if labels.is_empty() {
        return Vec::new();
    }
    let chip_gap = browser_inline_tag_chip_gap(sizing);
    let total_width: f32 = labels
        .iter()
        .map(|label| browser_inline_tag_chip_width(label, sizing))
        .sum::<f32>()
        + (labels.len().saturating_sub(1) as f32 * chip_gap);
    let right_edge = (sample_label.max.x - trailing_reserved_width).max(sample_label.min.x);
    let start_x = (right_edge - total_width).max(sample_label.min.x);
    let chip_height = browser_inline_tag_chip_height(sample_label, sizing);
    let min_y = sample_label.min.y + ((sample_label.height() - chip_height) * 0.5).floor();
    let max_y = (min_y + chip_height).min(sample_label.max.y);
    let mut x = start_x;
    labels
        .into_iter()
        .map(|label| {
            let width = browser_inline_tag_chip_width(label, sizing);
            let rect = Rect::from_min_max(
                Point::new(x, min_y),
                Point::new((x + width).min(right_edge), max_y),
            );
            x = (rect.max.x + chip_gap).min(right_edge);
            rect
        })
        .collect()
}

/// Return the inset text origin for one inline browser metadata chip.
pub(in crate::gui::native_shell::state) fn browser_inline_tag_text_origin(
    chip_rect: Rect,
    sizing: SizingTokens,
) -> Point {
    Point::new(
        chip_rect.min.x + browser_inline_tag_chip_padding_x(sizing),
        chip_rect.min.y + ((chip_rect.height() - sizing.font_meta) * 0.5).floor(),
    )
}

pub(in crate::gui::native_shell::state) fn browser_inline_tag_chip_height(
    sample_label: Rect,
    sizing: SizingTokens,
) -> f32 {
    (sizing.font_meta + (browser_inline_tag_chip_padding_y(sizing) * 2.0))
        .round()
        .clamp(10.0, sample_label.height().max(1.0))
}

pub(in crate::gui::native_shell::state) fn browser_inline_tag_chip_padding_x(
    sizing: SizingTokens,
) -> f32 {
    sizing.text_inset_x.min(5.0).max(3.0)
}

pub(in crate::gui::native_shell::state) fn browser_inline_tag_chip_padding_y(
    sizing: SizingTokens,
) -> f32 {
    sizing.text_inset_y.min(3.0).max(1.0)
}

pub(in crate::gui::native_shell::state) fn browser_inline_tag_chip_gap(
    sizing: SizingTokens,
) -> f32 {
    sizing.border_width.max(1.0) + 2.0
}

/// Return the width reserved for the focused-row similarity trigger.
pub(in crate::gui::native_shell::state) fn browser_similarity_button_reserved_width(
    visible: bool,
    sizing: SizingTokens,
) -> f32 {
    if !visible {
        return 0.0;
    }
    browser_similarity_button_width(sizing) + browser_similarity_button_gap(sizing)
}

/// Return the far-right button rect used to trigger row similarity mode.
pub(in crate::gui::native_shell::state) fn browser_similarity_button_rect(
    row_rect: Rect,
    sizing: SizingTokens,
) -> Option<Rect> {
    if row_rect.width() <= 0.0 || row_rect.height() <= 0.0 {
        return None;
    }
    let inset = sizing.row_corner_inset.max(2.0);
    let width =
        browser_similarity_button_width(sizing).min((row_rect.width() - (inset * 2.0)).max(0.0));
    let height = browser_similarity_button_height(row_rect, sizing);
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let max_x = row_rect.max.x - inset;
    let min_x = (max_x - width).max(row_rect.min.x + inset);
    let min_y = row_rect.min.y + ((row_rect.height() - height) * 0.5).floor();
    Some(Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(max_x, (min_y + height).min(row_rect.max.y - inset)),
    ))
}

fn browser_similarity_button_width(sizing: SizingTokens) -> f32 {
    (sizing.font_meta * 4.4).round().clamp(28.0, 40.0)
}

fn browser_similarity_button_height(row_rect: Rect, sizing: SizingTokens) -> f32 {
    let inset = sizing.row_corner_inset.max(2.0);
    (row_rect.height() - (inset * 2.0))
        .round()
        .clamp(12.0, 20.0)
}

fn browser_similarity_button_gap(sizing: SizingTokens) -> f32 {
    sizing.text_inset_x.min(6.0).max(4.0)
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
