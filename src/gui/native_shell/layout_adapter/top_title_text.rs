//! Slotized helper for top-bar title text-line geometry.

use super::super::style::SizingTokens;
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};

const TOP_TITLE_TEXT_ROOT_ID: u64 = 1720;
const TOP_TITLE_TEXT_ALIGN_ID: u64 = 1721;
const TOP_TITLE_TEXT_LINE_ID: u64 = 1722;

/// Compute top-bar title label bounds through strict slotized text layout.
pub(crate) fn compute_top_bar_title_text_rect(
    title_cluster: Rect,
    title_row: Rect,
    sizing: SizingTokens,
) -> Rect {
    let empty = empty_rect(title_row);
    let bounds = title_text_bounds(title_cluster, title_row);
    if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return empty;
    }
    let inset = sizing.text_inset_x + sizing.header_label_gutter;
    let text_bounds = inset_rect_horizontal(bounds, inset.max(0.0), inset.max(0.0));
    if text_bounds.width() <= 0.0 || text_bounds.height() <= 0.0 {
        return empty_rect(text_bounds);
    }
    let output = layout_tree(&build_top_title_tree(sizing.font_title), text_bounds);
    let line = rect_for(
        &output.rects,
        TOP_TITLE_TEXT_LINE_ID,
        empty_rect(text_bounds),
    );
    clamp_top_inset(line, text_bounds, sizing.text_inset_y.max(0.0))
}

fn build_top_title_tree(font_size: f32) -> LayoutNode {
    let line_height = font_size.max(1.0);
    LayoutNode::container(
        TOP_TITLE_TEXT_ROOT_ID,
        align_box_policy(),
        vec![SlotChild {
            slot: title_slot(line_height),
            child: LayoutNode::container(
                TOP_TITLE_TEXT_ALIGN_ID,
                padding_box_policy(),
                vec![SlotChild {
                    slot: SlotParams::fill(),
                    child: LayoutNode::widget(
                        TOP_TITLE_TEXT_LINE_ID,
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

fn title_slot(line_height: f32) -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Fixed(line_height),
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::new(0.0, f32::INFINITY, line_height, line_height),
        margin: Insets::default(),
        align_cross_override: Some(CrossAlign::Stretch),
        allow_fixed_compress: false,
    }
}

fn title_text_bounds(title_cluster: Rect, title_row: Rect) -> Rect {
    if title_cluster.width() <= 0.0 || title_row.height() <= 0.0 {
        return empty_rect(title_row);
    }
    Rect::from_min_max(
        Point::new(title_cluster.min.x, title_row.min.y),
        Point::new(title_cluster.max.x, title_row.max.y.max(title_row.min.y)),
    )
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
    fn top_bar_title_text_rect_stays_inside_title_bounds() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let cluster = Rect::from_min_max(Point::new(20.0, 12.0), Point::new(880.0, 36.0));
        let row = Rect::from_min_max(Point::new(20.0, 12.0), Point::new(1260.0, 36.0));
        let text_rect = compute_top_bar_title_text_rect(cluster, row, style.sizing);
        let bounds = title_text_bounds(cluster, row);
        assert_inside(bounds, text_rect);
    }

    #[test]
    fn top_bar_title_text_rect_respects_left_inset() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let cluster = Rect::from_min_max(Point::new(20.0, 12.0), Point::new(420.0, 36.0));
        let row = Rect::from_min_max(Point::new(20.0, 12.0), Point::new(1260.0, 36.0));
        let text_rect = compute_top_bar_title_text_rect(cluster, row, style.sizing);
        let inset = style.sizing.text_inset_x + style.sizing.header_label_gutter;
        assert!(text_rect.min.x >= cluster.min.x + inset);
    }

    #[test]
    fn top_bar_title_text_rect_collapses_for_empty_cluster() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let cluster = Rect::from_min_max(Point::new(20.0, 12.0), Point::new(20.0, 36.0));
        let row = Rect::from_min_max(Point::new(20.0, 12.0), Point::new(1260.0, 36.0));
        let text_rect = compute_top_bar_title_text_rect(cluster, row, style.sizing);
        assert_eq!(text_rect, Rect::from_min_max(row.min, row.min));
    }
}
