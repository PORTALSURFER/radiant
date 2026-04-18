//! Slotized text-line geometry helpers for control rows and action buttons.

use super::super::style::SizingTokens;
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};

const ACTION_BUTTON_TEXT_BASE_ID: u64 = 1610;
const TEXT_ROOT_ID: u64 = 1620;
const TEXT_ALIGN_ID: u64 = 1621;
const TEXT_LINE_ID: u64 = 1622;

/// Compute an action-button label line rect with horizontal inset.
pub(crate) fn compute_action_button_text_rect(rect: Rect, sizing: SizingTokens) -> Rect {
    compute_text_line_rect(
        rect,
        sizing,
        sizing.font_meta,
        sizing.text_inset_x.max(0.0),
        ACTION_BUTTON_TEXT_BASE_ID,
    )
}

fn compute_text_line_rect(
    rect: Rect,
    sizing: SizingTokens,
    font_size: f32,
    horizontal_inset: f32,
    node_id: u64,
) -> Rect {
    let empty = empty_rect(rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    let text_bounds = inset_horizontal(rect, horizontal_inset);
    if text_bounds.width() <= 0.0 || text_bounds.height() <= 0.0 {
        return empty;
    }
    let output = layout_tree(&centered_line_tree(font_size, node_id), text_bounds);
    let centered = clamp_rect_to_bounds(rect_for(&output.rects, TEXT_LINE_ID, empty), text_bounds);
    clamp_line_top_inset(centered, text_bounds, sizing.text_inset_y.max(0.0))
}

fn centered_line_tree(font_size: f32, node_id: u64) -> LayoutNode {
    LayoutNode::container(
        TEXT_ROOT_ID + node_id,
        ContainerPolicy {
            kind: ContainerKind::AlignBox,
            align_main: MainAlign::Center,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fixed(font_size.max(1.0)),
                size_cross: SizeModeCross::Fill,
                constraints: Constraints::new(
                    0.0,
                    f32::INFINITY,
                    font_size.max(1.0),
                    font_size.max(1.0),
                ),
                margin: Insets::default(),
                align_cross_override: Some(CrossAlign::Stretch),
                allow_fixed_compress: false,
            },
            child: LayoutNode::container(
                TEXT_ALIGN_ID + node_id,
                ContainerPolicy {
                    kind: ContainerKind::PaddingBox,
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![SlotChild {
                    slot: SlotParams::fill(),
                    child: LayoutNode::widget(TEXT_LINE_ID, Vector2::new(1.0, font_size.max(1.0))),
                }],
            ),
        }],
    )
}

fn clamp_line_top_inset(line: Rect, bounds: Rect, inset_y: f32) -> Rect {
    let min_y = (bounds.min.y + inset_y).min(bounds.max.y);
    if line.min.y >= min_y {
        return line;
    }
    let height = line.height().max(0.0);
    let shifted_min_y = min_y;
    let shifted_max_y = (shifted_min_y + height).min(bounds.max.y);
    clamp_rect_to_bounds(
        Rect::from_min_max(
            Point::new(line.min.x, shifted_min_y),
            Point::new(line.max.x, shifted_max_y),
        ),
        bounds,
    )
}

fn inset_horizontal(rect: Rect, inset: f32) -> Rect {
    let min_x = (rect.min.x + inset).min(rect.max.x);
    let max_x = (rect.max.x - inset).max(min_x);
    Rect::from_min_max(
        Point::new(min_x, rect.min.y),
        Point::new(max_x, rect.max.y.max(rect.min.y)),
    )
}

fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(min, max)
}

fn rect_for(rects: &std::collections::BTreeMap<u64, Rect>, id: u64, fallback: Rect) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

fn empty_rect(bounds: Rect) -> Rect {
    Rect::from_min_max(bounds.min, bounds.min)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::native_shell::style::StyleTokens;

    fn assert_inside(outer: Rect, inner: Rect) {
        assert!(inner.min.x >= outer.min.x);
        assert!(inner.min.y >= outer.min.y);
        assert!(inner.max.x <= outer.max.x);
        assert!(inner.max.y <= outer.max.y);
    }

    #[test]
    fn action_button_text_rect_respects_horizontal_inset() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let button = Rect::from_min_max(Point::new(920.0, 16.0), Point::new(1020.0, 34.0));
        let text_rect = compute_action_button_text_rect(button, style.sizing);
        assert_inside(button, text_rect);
        assert!(text_rect.min.x >= button.min.x + style.sizing.text_inset_x);
        assert!(text_rect.max.x <= button.max.x - style.sizing.text_inset_x);
    }

    #[test]
    fn action_button_text_rect_collapses_for_empty_button() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let button = Rect::from_min_max(Point::new(920.0, 16.0), Point::new(920.0, 16.0));
        let text_rect = compute_action_button_text_rect(button, style.sizing);
        assert_eq!(text_rect, button);
    }
}
