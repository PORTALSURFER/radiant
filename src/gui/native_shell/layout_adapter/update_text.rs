//! Slotized helpers for top-bar update status and controls text geometry.

use super::super::style::SizingTokens;
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};

const UPDATE_TEXT_ROW_ID: u64 = 1400;
const UPDATE_TEXT_LINE_REGION_ID: u64 = 1401;
const UPDATE_TEXT_RESERVED_ID: u64 = 1402;
const UPDATE_TEXT_ROOT_ID: u64 = 1410;
const UPDATE_TEXT_ALIGN_ID: u64 = 1411;
const UPDATE_TEXT_LINE_ID: u64 = 1412;

/// Slot-resolved top-bar update text bounds for title and controls rows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TopBarUpdateTextLayout {
    pub status_line: Rect,
    pub controls_line: Rect,
}

/// Compute top-bar update status/controls text line bounds via strict slot layout.
pub(crate) fn compute_top_bar_update_text_layout(
    action_cluster: Rect,
    title_row: Rect,
    controls_row: Rect,
    sizing: SizingTokens,
    button_rects: &[Rect],
) -> TopBarUpdateTextLayout {
    let status_bounds = row_with_action_cluster_x(action_cluster, title_row);
    let controls_bounds = row_with_action_cluster_x(action_cluster, controls_row);
    let reserved_width = reserved_button_width(action_cluster, button_rects);
    TopBarUpdateTextLayout {
        status_line: compute_update_text_line(
            status_bounds,
            sizing,
            sizing.font_meta,
            reserved_width,
        ),
        controls_line: compute_update_text_line(controls_bounds, sizing, sizing.font_meta, 0.0),
    }
}

fn compute_update_text_line(
    rect: Rect,
    sizing: SizingTokens,
    font_size: f32,
    reserve_right: f32,
) -> Rect {
    let empty = empty_rect(rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    let row_tree = build_text_row_tree(sizing, reserve_right);
    let output = layout_tree(&row_tree, rect);
    let line_region = clamp_rect_to_bounds(
        rect_for(&output.rects, UPDATE_TEXT_LINE_REGION_ID, empty),
        rect,
    );
    compute_text_line_in_region(line_region, sizing, font_size)
}

fn build_text_row_tree(sizing: SizingTokens, reserve_right: f32) -> LayoutNode {
    let mut children = vec![SlotChild {
        slot: SlotParams::fill(),
        child: LayoutNode::widget(UPDATE_TEXT_LINE_REGION_ID, Vector2::new(1.0, 1.0)),
    }];
    let reserved = reserve_right.max(0.0);
    if reserved > 0.0 {
        children.push(fixed_width_child(UPDATE_TEXT_RESERVED_ID, reserved));
    }
    LayoutNode::container(
        UPDATE_TEXT_ROW_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: sizing.text_inset_x.max(0.0),
                right: sizing.text_inset_x.max(0.0),
                ..Insets::default()
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                UPDATE_TEXT_ROW_ID + 20,
                ContainerPolicy {
                    kind: ContainerKind::Row,
                    align_main: MainAlign::Start,
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                children,
            ),
        }],
    )
}

fn compute_text_line_in_region(rect: Rect, sizing: SizingTokens, font_size: f32) -> Rect {
    let empty = empty_rect(rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    let tree = LayoutNode::container(
        UPDATE_TEXT_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                top: sizing.text_inset_y.max(0.0),
                bottom: sizing.text_inset_y.max(0.0),
                ..Insets::default()
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                UPDATE_TEXT_ALIGN_ID,
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
                    child: LayoutNode::widget(
                        UPDATE_TEXT_LINE_ID,
                        Vector2::new(1.0, font_size.max(1.0)),
                    ),
                }],
            ),
        }],
    );
    let output = layout_tree(&tree, rect);
    clamp_rect_to_bounds(rect_for(&output.rects, UPDATE_TEXT_LINE_ID, empty), rect)
}

fn fixed_width_child(node_id: u64, width: f32) -> SlotChild {
    let width = width.max(0.0);
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(width),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(width, width, 0.0, f32::INFINITY),
            margin: Insets::default(),
            align_cross_override: Some(CrossAlign::Stretch),
            allow_fixed_compress: false,
        },
        child: LayoutNode::widget(node_id, Vector2::new(width.max(1.0), 1.0)),
    }
}

fn reserved_button_width(action_cluster: Rect, button_rects: &[Rect]) -> f32 {
    let left = button_rects
        .iter()
        .map(|rect| rect.min.x)
        .min_by(f32::total_cmp)
        .unwrap_or(action_cluster.max.x);
    let clamped_left = left.clamp(action_cluster.min.x, action_cluster.max.x);
    (action_cluster.max.x - clamped_left).clamp(0.0, action_cluster.width())
}

fn row_with_action_cluster_x(action_cluster: Rect, row: Rect) -> Rect {
    let min = Point::new(action_cluster.min.x.max(row.min.x), row.min.y);
    let max = Point::new(action_cluster.max.x.min(row.max.x), row.max.y);
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(row.min, row.min);
    }
    Rect::from_min_max(min, max)
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
    fn update_text_lines_stay_inside_cluster_rows() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let cluster = Rect::from_min_max(Point::new(880.0, 16.0), Point::new(1260.0, 72.0));
        let title_row = Rect::from_min_max(Point::new(20.0, 16.0), Point::new(1260.0, 36.0));
        let controls_row = Rect::from_min_max(Point::new(20.0, 44.0), Point::new(1260.0, 72.0));
        let layout =
            compute_top_bar_update_text_layout(cluster, title_row, controls_row, style.sizing, &[]);
        assert_inside(cluster, layout.status_line);
        assert_inside(cluster, layout.controls_line);
        assert!(layout.status_line.min.y >= title_row.min.y);
        assert!(layout.status_line.max.y <= title_row.max.y);
        assert!(layout.controls_line.min.y >= controls_row.min.y);
        assert!(layout.controls_line.max.y <= controls_row.max.y);
    }

    #[test]
    fn update_status_line_reserves_button_region() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let cluster = Rect::from_min_max(Point::new(880.0, 16.0), Point::new(1260.0, 72.0));
        let title_row = Rect::from_min_max(Point::new(20.0, 16.0), Point::new(1260.0, 36.0));
        let controls_row = Rect::from_min_max(Point::new(20.0, 44.0), Point::new(1260.0, 72.0));
        let button = Rect::from_min_max(Point::new(1120.0, 18.0), Point::new(1240.0, 34.0));
        let layout = compute_top_bar_update_text_layout(
            cluster,
            title_row,
            controls_row,
            style.sizing,
            &[button],
        );
        assert!(layout.status_line.max.x <= button.min.x);
        assert!(layout.controls_line.max.x > layout.status_line.max.x);
    }

    #[test]
    fn update_text_layout_collapses_for_invalid_cluster() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let cluster = Rect::from_min_max(Point::new(500.0, 20.0), Point::new(500.0, 20.0));
        let row = Rect::from_min_max(Point::new(20.0, 16.0), Point::new(1260.0, 36.0));
        let layout = compute_top_bar_update_text_layout(cluster, row, row, style.sizing, &[]);
        assert_eq!(layout.status_line.width(), 0.0);
        assert_eq!(layout.status_line.height(), 0.0);
        assert_eq!(layout.controls_line.width(), 0.0);
        assert_eq!(layout.controls_line.height(), 0.0);
    }
}
