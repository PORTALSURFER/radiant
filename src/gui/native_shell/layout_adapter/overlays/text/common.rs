use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::native_shell::style::SizingTokens;
use crate::gui::types::{Rect, Vector2};
use super::super::shared;

pub(super) fn centered_line_in_rect(
    rect: Rect,
    sizing: SizingTokens,
    font_size: f32,
    node_id: u64,
) -> Rect {
    let empty = shared::empty_rect(rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    let output = layout_tree(
        &centered_line_tree(node_id, sizing, font_size),
        shared::clamp_rect_to_bounds(rect, rect),
    );
    shared::rect_for(&output.rects, node_id, empty)
}

pub(super) fn fixed_height_child(node_id: u64, height: f32) -> SlotChild {
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(height.max(0.0)),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(0.0, f32::INFINITY, height.max(0.0), height.max(0.0)),
            margin: Insets::default(),
            align_cross_override: Some(CrossAlign::Stretch),
            allow_fixed_compress: true,
        },
        child: LayoutNode::widget(node_id, Vector2::new(1.0, height.max(1.0))),
    }
}

pub(super) fn top_line_in_bounds(bounds: Rect, font_size: f32, node_id: u64) -> Rect {
    let empty = shared::empty_rect(bounds);
    if bounds.width() <= 0.0 || bounds.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    let output = layout_tree(&top_line_tree(node_id, 0.0, font_size), bounds);
    shared::rect_for(&output.rects, node_id, empty)
}

pub(super) fn top_line_in_rect(
    rect: Rect,
    sizing: SizingTokens,
    font_size: f32,
    node_id: u64,
) -> Rect {
    let empty = shared::empty_rect(rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    let output = layout_tree(
        &top_line_tree(node_id, sizing.text_inset_x.max(0.0), font_size),
        shared::clamp_rect_to_bounds(rect, rect),
    );
    shared::rect_for(&output.rects, node_id, empty)
}

fn centered_line_tree(node_id: u64, sizing: SizingTokens, font_size: f32) -> LayoutNode {
    LayoutNode::container(
        node_id + 100,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: sizing.text_inset_x.max(0.0),
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
                node_id + 101,
                ContainerPolicy {
                    kind: ContainerKind::AlignBox,
                    align_main: MainAlign::Center,
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![fixed_height_child(node_id, font_size.max(1.0))],
            ),
        }],
    )
}

pub(super) fn column_tree(root_id: u64, children: Vec<SlotChild>) -> LayoutNode {
    LayoutNode::container(
        root_id,
        ContainerPolicy {
            kind: ContainerKind::Column,
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        children,
    )
}

fn top_line_tree(node_id: u64, horizontal_inset: f32, font_size: f32) -> LayoutNode {
    let horizontal = Insets {
        left: horizontal_inset.max(0.0),
        right: horizontal_inset.max(0.0),
        ..Insets::default()
    };
    LayoutNode::container(
        node_id + 200,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: horizontal,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: column_tree(
                node_id + 201,
                vec![
                    fixed_height_child(node_id, font_size.max(1.0)),
                    SlotChild {
                        slot: SlotParams::fill(),
                        child: LayoutNode::widget(node_id + 202, Vector2::new(1.0, 1.0)),
                    },
                ],
            ),
        }],
    )
}
