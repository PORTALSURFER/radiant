//! Shared geometry helpers for layout placement.

use crate::gui::layout_core::model::{CrossAlign, Insets, SlotParams};
use crate::gui::types::{Point, Rect, Vector2};

pub(in crate::gui::layout_core::engine) fn place_child_rect(
    content: Rect,
    horizontal: bool,
    cursor_main: f32,
    child_main: f32,
    child_cross: f32,
    slot: SlotParams,
    align: CrossAlign,
) -> Rect {
    if horizontal {
        let x = content.min.x + cursor_main;
        let avail_cross = content.height() - slot.margin.top - slot.margin.bottom;
        let y = match align {
            CrossAlign::Start | CrossAlign::Stretch => content.min.y + slot.margin.top,
            CrossAlign::Center => content.min.y + ((content.height() - child_cross) * 0.5),
            CrossAlign::End => content.max.y - child_cross - slot.margin.bottom,
        };
        let h = if matches!(align, CrossAlign::Stretch) {
            avail_cross.max(0.0)
        } else {
            child_cross
        };
        return Rect::from_min_size(
            Point::new(x, y),
            Vector2::new(child_main.max(0.0), h.max(0.0)),
        );
    }

    let y = content.min.y + cursor_main;
    let avail_cross = content.width() - slot.margin.left - slot.margin.right;
    let x = match align {
        CrossAlign::Start | CrossAlign::Stretch => content.min.x + slot.margin.left,
        CrossAlign::Center => content.min.x + ((content.width() - child_cross) * 0.5),
        CrossAlign::End => content.max.x - child_cross - slot.margin.right,
    };
    let w = if matches!(align, CrossAlign::Stretch) {
        avail_cross.max(0.0)
    } else {
        child_cross
    };
    Rect::from_min_size(
        Point::new(x, y),
        Vector2::new(w.max(0.0), child_main.max(0.0)),
    )
}

pub(in crate::gui::layout_core::engine) fn content_rect(rect: Rect, padding: Insets) -> Rect {
    let min_x = rect.min.x + padding.left;
    let max_x = (rect.max.x - padding.right).max(min_x);
    let min_y = rect.min.y + padding.top;
    let max_y = (rect.max.y - padding.bottom).max(min_y);
    Rect::from_min_max(Point::new(min_x, min_y), Point::new(max_x, max_y))
}
