use super::*;
use crate::gui::feedback::{
    InlineIndicatorAnchor, InlineIndicatorMetrics, inline_indicator_layout,
    inline_indicator_reserved_width,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui::native_shell::state) struct RowRatingIndicatorLayout {
    pub(in crate::gui::native_shell::state) rects: [Rect; 3],
    pub(in crate::gui::native_shell::state) count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui::native_shell::state) struct RowRatingIndicatorAnchor {
    pub(in crate::gui::native_shell::state) item_label: Rect,
    pub(in crate::gui::native_shell::state) label_origin_x: f32,
    pub(in crate::gui::native_shell::state) label_rendered_width: f32,
    pub(in crate::gui::native_shell::state) right_limit_x: f32,
}

pub(in crate::gui::native_shell::state) fn row_rating_indicator_reserved_width(
    rating_level: i8,
    locked: bool,
    sizing: SizingTokens,
) -> f32 {
    let count = row_rating_indicator_count(rating_level, locked);
    inline_indicator_reserved_width(
        count,
        row_rating_indicator_metrics(rating_level, locked, sizing),
    )
}

pub(in crate::gui::native_shell::state) fn row_rating_indicator_layout(
    anchor: RowRatingIndicatorAnchor,
    rating_level: i8,
    locked: bool,
    sizing: SizingTokens,
) -> Option<RowRatingIndicatorLayout> {
    let count = row_rating_indicator_count(rating_level, locked);
    let item_label = anchor.item_label;
    let layout = inline_indicator_layout(
        InlineIndicatorAnchor {
            content_rect: item_label,
            text_origin_x: anchor.label_origin_x,
            text_width: anchor.label_rendered_width,
            right_limit_x: anchor.right_limit_x,
        },
        count,
        row_rating_indicator_metrics(rating_level, locked, sizing),
    )?;
    let mut rects = [item_label.empty_at_min(); 3];
    for (target, source) in rects.iter_mut().zip(layout.rects).take(layout.count.min(3)) {
        *target = source;
    }
    Some(RowRatingIndicatorLayout {
        rects,
        count: layout.count.min(3),
    })
}

pub(in crate::gui::native_shell::state) fn row_rating_indicator_color(
    style: &StyleTokens,
    rating_level: i8,
) -> Rgba8 {
    if rating_level < 0 {
        style.accent_danger
    } else {
        style.accent_mint
    }
}

pub(in crate::gui::native_shell::state) fn row_rating_indicator_side(
    sizing: SizingTokens,
) -> f32 {
    (sizing.font_meta * 0.68).round().clamp(5.0, 8.0)
}

pub(in crate::gui::native_shell::state) fn row_rating_indicator_count(
    rating_level: i8,
    locked: bool,
) -> usize {
    if locked && rating_level > 0 {
        1
    } else {
        rating_level.unsigned_abs().min(3) as usize
    }
}

pub(in crate::gui::native_shell::state) fn row_rating_indicator_unit_width(
    rating_level: i8,
    locked: bool,
    sizing: SizingTokens,
) -> f32 {
    let side = row_rating_indicator_side(sizing);
    if locked && rating_level > 0 {
        (side * 2.4).round().max(side + 2.0)
    } else {
        side
    }
}

pub(in crate::gui::native_shell::state) fn row_rating_indicator_gap(
    sizing: SizingTokens,
) -> f32 {
    sizing.border_width.max(1.0) + 1.0
}

pub(in crate::gui::native_shell::state) fn row_rating_indicator_text_gap(
    sizing: SizingTokens,
) -> f32 {
    sizing.text_inset_x.min(5.0).max(2.0)
}

fn row_rating_indicator_metrics(
    rating_level: i8,
    locked: bool,
    sizing: SizingTokens,
) -> InlineIndicatorMetrics {
    InlineIndicatorMetrics {
        unit_width: row_rating_indicator_unit_width(rating_level, locked, sizing),
        unit_height: row_rating_indicator_side(sizing),
        unit_gap: row_rating_indicator_gap(sizing),
        text_gap: row_rating_indicator_text_gap(sizing),
        max_count: 3,
    }
}
