//! Slotized helpers for status-bar segment and text-line geometry.

use super::super::style::SizingTokens;
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};

const STATUS_ROOT_ID: u64 = 900;
const STATUS_ROW_ID: u64 = 901;
const STATUS_LEFT_ID: u64 = 902;
const STATUS_GAP_LEFT_ID: u64 = 903;
const STATUS_CENTER_ID: u64 = 904;
const STATUS_GAP_RIGHT_ID: u64 = 905;
const STATUS_RIGHT_ID: u64 = 906;
const STATUS_TEXT_ROOT_ID: u64 = 920;
const STATUS_TEXT_ALIGN_ID: u64 = 921;
const STATUS_TEXT_LINE_ID: u64 = 922;
const STATUS_LEFT_RATIO: f32 = 0.30;
const STATUS_RIGHT_RATIO: f32 = 0.22;

/// Slot-resolved left/center/right status-bar segment geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StatusBarSegments {
    pub left: Rect,
    pub center: Rect,
    pub right: Rect,
}

/// Compute left/center/right status-bar segments through slotized layout.
pub(crate) fn compute_status_bar_segments(
    status_bar: Rect,
    sizing: SizingTokens,
) -> StatusBarSegments {
    let empty = empty_rect(status_bar);
    if status_bar.width() <= 0.0 || status_bar.height() <= 0.0 {
        return StatusBarSegments {
            left: empty,
            center: empty,
            right: empty,
        };
    }
    let row = LayoutNode::container(
        STATUS_ROW_ID,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 0.0,
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![
            percent_child(STATUS_LEFT_ID, STATUS_LEFT_RATIO),
            fixed_gap_child(STATUS_GAP_LEFT_ID, sizing.status_segment_gap),
            SlotChild {
                slot: SlotParams::fill(),
                child: LayoutNode::widget(STATUS_CENTER_ID, Vector2::new(1.0, 1.0)),
            },
            fixed_gap_child(STATUS_GAP_RIGHT_ID, sizing.status_segment_gap),
            percent_child(STATUS_RIGHT_ID, STATUS_RIGHT_RATIO),
        ],
    );
    let tree = LayoutNode::container(
        STATUS_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: sizing.panel_inset.max(0.0),
                right: sizing.panel_inset.max(0.0),
                ..Insets::default()
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: row,
        }],
    );
    let output = layout_tree(&tree, status_bar);
    let left = clamp_rect_to_bounds(rect_for(&output.rects, STATUS_LEFT_ID, empty), status_bar);
    let center = clamp_rect_to_bounds(rect_for(&output.rects, STATUS_CENTER_ID, empty), status_bar);
    let right = clamp_rect_to_bounds(rect_for(&output.rects, STATUS_RIGHT_ID, empty), status_bar);
    StatusBarSegments {
        left,
        center,
        right,
    }
}

/// Compute a status text line bounds rect inside a status segment.
pub(crate) fn compute_status_text_line_rect(
    segment: Rect,
    sizing: SizingTokens,
    font_size: f32,
) -> Rect {
    let empty = empty_rect(segment);
    if segment.width() <= 0.0 || segment.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    let segment_tree = LayoutNode::container(
        STATUS_TEXT_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: (sizing.text_inset_x + sizing.header_label_gutter).max(0.0),
                right: sizing.text_inset_x.max(0.0),
                top: sizing.text_inset_y.max(0.0),
                bottom: sizing.text_inset_y.max(0.0),
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                STATUS_TEXT_ALIGN_ID,
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
                        STATUS_TEXT_LINE_ID,
                        Vector2::new(1.0, font_size.max(1.0)),
                    ),
                }],
            ),
        }],
    );
    let output = layout_tree(&segment_tree, segment);
    clamp_rect_to_bounds(rect_for(&output.rects, STATUS_TEXT_LINE_ID, empty), segment)
}

fn percent_child(node_id: u64, ratio: f32) -> SlotChild {
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Percent(ratio.max(0.0)),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(0.0, f32::INFINITY, 0.0, f32::INFINITY),
            margin: Insets::default(),
            align_cross_override: Some(CrossAlign::Stretch),
            allow_fixed_compress: false,
        },
        child: LayoutNode::widget(node_id, Vector2::new(1.0, 1.0)),
    }
}

fn fixed_gap_child(node_id: u64, width: f32) -> SlotChild {
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
    fn status_segments_preserve_order_and_spacing() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let bar = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(1280.0, 20.0));
        let segments = compute_status_bar_segments(bar, style.sizing);
        assert_inside(bar, segments.left);
        assert_inside(bar, segments.center);
        assert_inside(bar, segments.right);
        assert!(segments.left.max.x <= segments.center.min.x);
        assert!(segments.center.max.x <= segments.right.min.x);
    }

    #[test]
    fn status_segments_clamp_when_bar_is_narrow() {
        let style = StyleTokens::for_viewport_width(820.0);
        let bar = Rect::from_min_max(Point::new(10.0, 5.0), Point::new(74.0, 20.0));
        let segments = compute_status_bar_segments(bar, style.sizing);
        assert_inside(bar, segments.left);
        assert_inside(bar, segments.center);
        assert_inside(bar, segments.right);
    }

    #[test]
    fn status_text_rect_stays_within_segment() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let segment = Rect::from_min_max(Point::new(20.0, 4.0), Point::new(380.0, 20.0));
        let text_rect =
            compute_status_text_line_rect(segment, style.sizing, style.sizing.font_status);
        assert_inside(segment, text_rect);
        assert!(text_rect.width() > 0.0);
        assert!(text_rect.height() > 0.0);
    }

    #[test]
    fn status_text_rect_collapses_for_invalid_segment() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let segment = Rect::from_min_max(Point::new(20.0, 4.0), Point::new(20.0, 4.0));
        let text_rect =
            compute_status_text_line_rect(segment, style.sizing, style.sizing.font_status);
        assert_eq!(text_rect, Rect::from_min_max(segment.min, segment.min));
    }
}
