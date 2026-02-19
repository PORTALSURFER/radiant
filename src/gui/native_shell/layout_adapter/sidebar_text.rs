//! Slotized sidebar row and recovery-badge text-line geometry helpers.

use super::super::style::SizingTokens;
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};

const SIDEBAR_TEXT_ROOT_ID: u64 = 1700;
const SIDEBAR_TEXT_ALIGN_ID: u64 = 1701;
const SIDEBAR_TEXT_LINE_ID: u64 = 1702;
const SIDEBAR_BADGE_TEXT_ID: u64 = 1710;

/// Compute source-row label bounds through strict slotized text layout.
pub(crate) fn compute_sidebar_source_row_text_rect(row_rect: Rect, sizing: SizingTokens) -> Rect {
    let inset = sizing.text_inset_x + sizing.row_corner_inset;
    let bounds = inset_rect_horizontal(row_rect, inset, inset);
    compute_sidebar_text_line(
        bounds,
        sizing.font_body,
        sizing.text_inset_y,
        SIDEBAR_TEXT_LINE_ID,
    )
}

/// Compute folder-row label bounds through strict slotized text layout.
pub(crate) fn compute_sidebar_folder_row_text_rect(
    row_rect: Rect,
    sizing: SizingTokens,
    depth_indent: f32,
) -> Rect {
    let base_inset = sizing.text_inset_x + sizing.row_corner_inset;
    let left_inset = base_inset + depth_indent.max(0.0);
    let bounds = inset_rect_horizontal(row_rect, left_inset, base_inset);
    compute_sidebar_text_line(
        bounds,
        sizing.font_body,
        sizing.text_inset_y,
        SIDEBAR_TEXT_LINE_ID + 1,
    )
}

/// Compute recovery-badge label bounds through strict slotized text layout.
pub(crate) fn compute_sidebar_recovery_badge_text_rect(
    badge_rect: Rect,
    sizing: SizingTokens,
) -> Rect {
    let inset = sizing.text_inset_x.max(0.0);
    let bounds = inset_rect_horizontal(badge_rect, inset, inset);
    compute_sidebar_text_line(
        bounds,
        sizing.font_meta,
        sizing.text_inset_y,
        SIDEBAR_BADGE_TEXT_ID,
    )
}

fn compute_sidebar_text_line(rect: Rect, font_size: f32, inset_y: f32, node_seed: u64) -> Rect {
    let empty = empty_rect(rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    let tree = build_sidebar_text_tree(font_size, node_seed);
    let output = layout_tree(&tree, rect);
    let line = rect_for(&output.rects, SIDEBAR_TEXT_LINE_ID + node_seed, empty);
    clamp_top_inset(line, rect, inset_y.max(0.0))
}

fn build_sidebar_text_tree(font_size: f32, node_seed: u64) -> LayoutNode {
    let line_height = font_size.max(1.0);
    LayoutNode::container(
        SIDEBAR_TEXT_ROOT_ID + node_seed,
        align_box_policy(),
        vec![SlotChild {
            slot: text_line_slot(line_height),
            child: LayoutNode::container(
                SIDEBAR_TEXT_ALIGN_ID + node_seed,
                padding_box_policy(),
                vec![SlotChild {
                    slot: SlotParams::fill(),
                    child: LayoutNode::widget(
                        SIDEBAR_TEXT_LINE_ID + node_seed,
                        Vector2::new(1.0, line_height),
                    ),
                }],
            ),
        }],
    )
}

fn align_box_policy() -> ContainerPolicy {
    ContainerPolicy {
        kind: ContainerKind::AlignBox,
        align_main: MainAlign::Center,
        align_cross: CrossAlign::Stretch,
        overflow: OverflowPolicy::Clip,
        ..ContainerPolicy::default()
    }
}

fn padding_box_policy() -> ContainerPolicy {
    ContainerPolicy {
        kind: ContainerKind::PaddingBox,
        align_cross: CrossAlign::Stretch,
        overflow: OverflowPolicy::Clip,
        ..ContainerPolicy::default()
    }
}

fn text_line_slot(line_height: f32) -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Fixed(line_height),
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::new(0.0, f32::INFINITY, line_height, line_height),
        margin: Insets::default(),
        align_cross_override: Some(CrossAlign::Stretch),
        allow_fixed_compress: false,
    }
}

fn clamp_top_inset(line: Rect, bounds: Rect, inset_y: f32) -> Rect {
    let min_y = (bounds.min.y + inset_y).min(bounds.max.y);
    if line.min.y >= min_y {
        return clamp_rect_to_bounds(line, bounds);
    }
    let height = line.height().max(0.0);
    let shifted = Rect::from_min_max(
        Point::new(line.min.x, min_y),
        Point::new(line.max.x, (min_y + height).min(bounds.max.y)),
    );
    clamp_rect_to_bounds(shifted, bounds)
}

fn inset_rect_horizontal(rect: Rect, left: f32, right: f32) -> Rect {
    let min_x = (rect.min.x + left.max(0.0)).min(rect.max.x);
    let max_x = (rect.max.x - right.max(0.0)).max(min_x);
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
    fn source_row_text_rect_stays_inside_row() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let row = Rect::from_min_max(Point::new(8.0, 64.0), Point::new(198.0, 80.0));
        let text_rect = compute_sidebar_source_row_text_rect(row, style.sizing);
        assert_inside(row, text_rect);
    }

    #[test]
    fn folder_row_text_rect_respects_depth_indent() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let row = Rect::from_min_max(Point::new(8.0, 296.0), Point::new(198.0, 312.0));
        let depth_indent = 18.0;
        let text_rect = compute_sidebar_folder_row_text_rect(row, style.sizing, depth_indent);
        assert_inside(row, text_rect);
        let base_left = row.min.x + style.sizing.text_inset_x + style.sizing.row_corner_inset;
        assert!(text_rect.min.x >= base_left + depth_indent);
    }

    #[test]
    fn recovery_badge_text_rect_stays_inside_badge() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let badge = Rect::from_min_max(Point::new(152.0, 276.0), Point::new(196.0, 292.0));
        let text_rect = compute_sidebar_recovery_badge_text_rect(badge, style.sizing);
        assert_inside(badge, text_rect);
    }
}
