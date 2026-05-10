//! Wrap container measurement strategy.

use super::super::measure_node;
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::engine::LayoutContext;
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
        let measured = measure_node(&child.child, child.slot.constraints, context);
        let item_w = measured.x + child.slot.margin.left + child.slot.margin.right;
        let item_h = measured.y + child.slot.margin.top + child.slot.margin.bottom;
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
