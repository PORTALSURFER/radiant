//! Box-like container measurement strategies.

use super::super::measure_node;
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::engine::LayoutContext;
use crate::gui::layout_core::engine::helpers::{fit_aspect_box, select_switch_child};
use crate::gui::layout_core::tree::{ContainerNode, SlotChild};
use crate::gui::types::Vector2;

pub(super) fn measure_stack(
    children: &[SlotChild],
    constraints: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    let mut max_w: f32 = 0.0;
    let mut max_h: f32 = 0.0;
    for slot_child in children {
        let child_size = measure_node(&slot_child.child, slot_child.slot.constraints, context);
        let width = child_size.x + slot_child.slot.margin.left + slot_child.slot.margin.right;
        let height = child_size.y + slot_child.slot.margin.top + slot_child.slot.margin.bottom;
        max_w = max_w.max(width.min(constraints.max_w));
        max_h = max_h.max(height.min(constraints.max_h));
    }
    Vector2::new(max_w, max_h)
}

pub(super) fn measure_aspect_box(
    container: &ContainerNode,
    constraints: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    let ratio = container.policy.aspect_ratio.unwrap_or(1.0).max(0.0001);
    let (target_w, target_h) = fit_aspect_box(constraints.max_w, constraints.max_h, ratio);
    if let Some(child) = container.children.first() {
        let _ = measure_node(
            &child.child,
            Constraints::new(0.0, target_w, 0.0, target_h),
            context,
        );
    }
    Vector2::new(target_w, target_h)
}

pub(super) fn measure_switch_layout(
    container: &ContainerNode,
    constraints: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    let selected = select_switch_child(container, constraints.max_w);
    if let Some(index) = selected
        && let Some(child) = container.children.get(index)
    {
        return measure_node(&child.child, child.slot.constraints, context);
    }
    Vector2::new(0.0, 0.0)
}

pub(super) fn measure_floating_layer(
    container: &ContainerNode,
    context: &mut LayoutContext,
) -> Vector2 {
    if let Some(child) = container.children.first() {
        let policy = container.policy.floating;
        let _ = measure_node(
            &child.child,
            Constraints::new(0.0, policy.size.x.max(0.0), 0.0, policy.size.y.max(0.0)),
            context,
        );
    }
    Vector2::new(0.0, 0.0)
}
