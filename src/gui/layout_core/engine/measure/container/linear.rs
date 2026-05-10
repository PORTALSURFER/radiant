//! Linear container measurement strategies.

use super::super::measure_node;
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::engine::LayoutContext;
use crate::gui::layout_core::model::{SizeModeCross, SizeModeMain};
use crate::gui::layout_core::tree::SlotChild;
use crate::gui::types::Vector2;

pub(super) fn measure_linear(
    horizontal: bool,
    children: &[SlotChild],
    constraints: Constraints,
    spacing: f32,
    context: &mut LayoutContext,
) -> Vector2 {
    if children.is_empty() {
        return Vector2::new(0.0, 0.0);
    }

    let mut main_total = 0.0;
    let mut cross_max: f32 = 0.0;
    for slot_child in children {
        let child_size = measure_node(&slot_child.child, slot_child.slot.constraints, context);
        let slot = slot_child.slot;
        let child_main = context.clamp_main(
            slot_child.child.id(),
            horizontal,
            slot.constraints,
            resolve_mode_main(horizontal, slot.size_main, child_size, constraints),
        );
        let child_cross = context.clamp_cross(
            slot_child.child.id(),
            horizontal,
            slot.constraints,
            resolve_mode_cross(horizontal, slot.size_cross, child_size, constraints),
        );
        let margin_main = if horizontal {
            slot.margin.left + slot.margin.right
        } else {
            slot.margin.top + slot.margin.bottom
        };
        let margin_cross = if horizontal {
            slot.margin.top + slot.margin.bottom
        } else {
            slot.margin.left + slot.margin.right
        };
        main_total += child_main + margin_main;
        cross_max = cross_max.max(child_cross + margin_cross);
    }
    main_total += spacing.max(0.0) * (children.len().saturating_sub(1) as f32);

    if horizontal {
        Vector2::new(
            main_total.min(constraints.max_w),
            cross_max.min(constraints.max_h),
        )
    } else {
        Vector2::new(
            cross_max.min(constraints.max_w),
            main_total.min(constraints.max_h),
        )
    }
}

fn resolve_mode_main(
    horizontal: bool,
    mode: SizeModeMain,
    measured: Vector2,
    parent: Constraints,
) -> f32 {
    match mode {
        SizeModeMain::Fixed(value) => value,
        SizeModeMain::Percent(percent) => {
            let parent_main = if horizontal {
                parent.max_w
            } else {
                parent.max_h
            };
            parent_main * percent.clamp(0.0, 1.0)
        }
        SizeModeMain::Intrinsic => {
            if horizontal {
                measured.x
            } else {
                measured.y
            }
        }
        SizeModeMain::Fill(_) => 0.0,
    }
}

fn resolve_mode_cross(
    horizontal: bool,
    mode: SizeModeCross,
    measured: Vector2,
    parent: Constraints,
) -> f32 {
    match mode {
        SizeModeCross::Fixed(value) => value,
        SizeModeCross::Fill => {
            if horizontal {
                parent.max_h
            } else {
                parent.max_w
            }
        }
        SizeModeCross::Intrinsic => {
            if horizontal {
                measured.y
            } else {
                measured.x
            }
        }
    }
}
