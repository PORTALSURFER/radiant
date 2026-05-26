//! Wrap container measurement strategy.

use super::super::measure_node;
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::engine::LayoutContext;
use crate::gui::layout_core::engine::direct::direct_widget_measure;
use crate::gui::layout_core::model::{SizeModeCross, SizeModeMain};
use crate::gui::layout_core::tree::ContainerNode;
use crate::gui::types::Vector2;

pub(super) fn measure_wrap(
    container: &ContainerNode,
    constraints: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    let available_w = constraints.max_w.max(0.0);
    let item_gap = container.policy.wrap.item_gap.max(0.0);
    let line_gap = container.policy.wrap.line_gap.max(0.0);

    let mut line_w = 0.0;
    let mut line_h: f32 = 0.0;
    let mut total_h = 0.0;
    let mut used_w: f32 = 0.0;

    for child in &container.children {
        let measured = if let Some(measured) = direct_widget_measure(child) {
            context.record_measured_size(child.child.id(), measured);
            measured
        } else {
            measure_node(&child.child, child.slot.constraints, context)
        };
        let item_w = wrap_item_width(child.slot.size_main, measured, available_w)
            + child.slot.margin.left
            + child.slot.margin.right;
        let item_h = wrap_item_height(child.slot.size_cross, measured, constraints.max_h)
            + child.slot.margin.top
            + child.slot.margin.bottom;
        let proposed = if line_w <= 0.0 {
            item_w
        } else {
            line_w + item_gap + item_w
        };
        if proposed > available_w && line_w > 0.0 {
            used_w = used_w.max(line_w);
            total_h += line_h + line_gap;
            line_w = item_w;
            line_h = item_h;
            continue;
        }
        line_w = proposed;
        line_h = line_h.max(item_h);
    }

    if line_w > 0.0 {
        used_w = used_w.max(line_w);
        total_h += line_h;
    }

    Vector2::new(
        used_w.min(constraints.max_w),
        total_h.min(constraints.max_h),
    )
}

fn wrap_item_width(mode: SizeModeMain, measured: Vector2, available_w: f32) -> f32 {
    match mode {
        SizeModeMain::Fixed(value) => value,
        SizeModeMain::Percent(percent) => available_w * percent.clamp(0.0, 1.0),
        SizeModeMain::Intrinsic => measured.x,
        SizeModeMain::Fill(_) => available_w,
    }
    .max(0.0)
}

fn wrap_item_height(mode: SizeModeCross, measured: Vector2, available_h: f32) -> f32 {
    match mode {
        SizeModeCross::Fixed(value) => value,
        SizeModeCross::Fill => measured.y.min(available_h),
        SizeModeCross::Intrinsic => measured.y,
    }
    .max(0.0)
}
