//! Shared geometry helpers for layout placement.

use crate::gui::layout_core::model::{CrossAlign, Insets, SlotParams};
use crate::gui::types::{Point, Rect, Vector2};

#[allow(clippy::too_many_arguments)]
pub(in crate::gui::layout_core::engine) fn place_child_rect_with_direction(
    content: Rect,
    horizontal: bool,
    cursor_main: f32,
    child_main: f32,
    child_cross: f32,
    slot: SlotParams,
    align: CrossAlign,
    direction: crate::gui::layout_core::WritingDirection,
) -> Rect {
    if horizontal {
        let x = if direction == crate::gui::layout_core::WritingDirection::Rtl {
            content.max.x - cursor_main - child_main
        } else {
            content.min.x + cursor_main
        };
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
        CrossAlign::Start | CrossAlign::Stretch => {
            if direction == crate::gui::layout_core::WritingDirection::Rtl {
                content.max.x - child_cross - slot.margin.right
            } else {
                content.min.x + slot.margin.left
            }
        }
        CrossAlign::Center => content.min.x + ((content.width() - child_cross) * 0.5),
        CrossAlign::End => {
            if direction == crate::gui::layout_core::WritingDirection::Rtl {
                content.min.x + slot.margin.left
            } else {
                content.max.x - child_cross - slot.margin.right
            }
        }
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

pub(in crate::gui::layout_core::engine) fn content_rect(
    rect: Rect,
    padding: Insets,
) -> Option<Rect> {
    crate::gui::layout_core::validated_geometry::checked_inset_rect(rect, padding)
}
