//! Slotized top-bar control partitioning.

use super::super::style::SizingTokens;
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, OverflowPolicy,
    SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};

const TOP_CONTROLS_ROOT_ID: u64 = 200;
const TOP_CONTROLS_ROW_ID: u64 = 201;
const TOP_CONTROLS_OPTIONS_ID: u64 = 202;
const TOP_CONTROLS_METER_ID: u64 = 203;
const TOP_CONTROLS_VALUE_ID: u64 = 204;
const TOP_CONTROLS_LABEL_ID: u64 = 205;

/// Slot-resolved rectangles for top-bar controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TopBarControlsSections {
    pub active: bool,
    pub options_label: Rect,
    pub volume_meter: Rect,
    pub volume_value: Rect,
    pub volume_label: Rect,
}

/// Compute top-bar control rectangles from a strict slot tree.
pub(crate) fn compute_top_bar_controls_sections(
    layout: &super::super::layout::ShellLayout,
    sizing: SizingTokens,
) -> TopBarControlsSections {
    let row = layout.top_bar_title_cluster;
    if row.height() <= 1.0 || row.width() <= 1.0 {
        return inactive_top_controls(row);
    }
    let horizontal_inset = sizing.text_inset_x + sizing.header_label_gutter;
    let options_width = 64.0_f32.min((row.width() * 0.35).max(24.0));
    let meter_width = sizing
        .top_volume_meter_width
        .min((row.width() * 0.45).max(26.0))
        .max(26.0);
    let value_width = 44.0_f32.min((row.width() * 0.2).max(20.0));
    let label_width = 28.0_f32.min((row.width() * 0.12).max(16.0));
    let gap = sizing.action_button_gap.max(2.0);
    let available_width = row.width() - (horizontal_inset * 2.0);
    let total_width = options_width + gap + meter_width + gap + value_width + gap + label_width;
    if available_width <= 12.0 || total_width > available_width {
        return inactive_top_controls(row);
    }
    let meter_height = sizing
        .top_volume_meter_height
        .min(row.height().max(1.0))
        .max(3.0);
    let controls_tree = LayoutNode::container(
        TOP_CONTROLS_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: horizontal_inset,
                right: horizontal_inset,
                top: 0.0,
                bottom: 0.0,
            },
            align_cross: CrossAlign::Stretch,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                TOP_CONTROLS_ROW_ID,
                ContainerPolicy {
                    kind: ContainerKind::Row,
                    spacing: gap,
                    align_cross: CrossAlign::Center,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![
                    fixed_slot_child(TOP_CONTROLS_OPTIONS_ID, options_width, 0.0),
                    fixed_slot_child(TOP_CONTROLS_METER_ID, meter_width, meter_height),
                    fixed_slot_child(TOP_CONTROLS_VALUE_ID, value_width, 0.0),
                    fixed_slot_child(TOP_CONTROLS_LABEL_ID, label_width, 0.0),
                ],
            ),
        }],
    );
    let output = layout_tree(&controls_tree, row);
    let empty = Rect::from_min_max(row.min, row.min);
    let options_label = clamp_rect_to_bounds(
        output
            .rects
            .get(&TOP_CONTROLS_OPTIONS_ID)
            .copied()
            .unwrap_or(empty),
        row,
    );
    let volume_meter = clamp_rect_to_bounds(
        output
            .rects
            .get(&TOP_CONTROLS_METER_ID)
            .copied()
            .unwrap_or(empty),
        row,
    );
    let volume_value = clamp_rect_to_bounds(
        output
            .rects
            .get(&TOP_CONTROLS_VALUE_ID)
            .copied()
            .unwrap_or(empty),
        row,
    );
    let volume_label = clamp_rect_to_bounds(
        output
            .rects
            .get(&TOP_CONTROLS_LABEL_ID)
            .copied()
            .unwrap_or(empty),
        row,
    );
    if options_label.width() <= 0.0
        || volume_meter.width() <= 0.0
        || volume_value.width() <= 0.0
        || volume_label.width() <= 0.0
    {
        return inactive_top_controls(row);
    }
    TopBarControlsSections {
        active: true,
        options_label,
        volume_meter,
        volume_value,
        volume_label,
    }
}

fn fixed_slot_child(node_id: u64, width: f32, height: f32) -> SlotChild {
    let cross_mode = if height > 0.0 {
        SizeModeCross::Fixed(height)
    } else {
        SizeModeCross::Fill
    };
    let constraints = if height > 0.0 {
        Constraints::new(width, width, height, height)
    } else {
        Constraints::new(width, width, 0.0, f32::INFINITY)
    };
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(width),
            size_cross: cross_mode,
            constraints,
            margin: Insets::default(),
            align_cross_override: Some(CrossAlign::Center),
            allow_fixed_compress: false,
        },
        child: LayoutNode::widget(node_id, Vector2::new(width.max(0.0), height.max(1.0))),
    }
}

fn inactive_top_controls(row: Rect) -> TopBarControlsSections {
    let empty = Rect::from_min_max(row.min, row.min);
    TopBarControlsSections {
        active: false,
        options_label: empty,
        volume_meter: empty,
        volume_value: empty,
        volume_label: empty,
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
