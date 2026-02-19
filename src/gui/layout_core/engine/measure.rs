//! Measure pass implementation for strict slot-based layout trees.

use super::{ConstraintKey, LayoutContext};
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::model::{ContainerKind, SizeModeCross, SizeModeMain};
use crate::gui::layout_core::tree::{ContainerNode, LayoutNode, SlotChild};
use crate::gui::types::Vector2;

pub(super) fn measure_node(
    node: &LayoutNode,
    constraints: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    let key = (node.id(), ConstraintKey::from_constraints(constraints));
    if let Some(size) = context.measured.get(&key).copied() {
        return size;
    }
    let normalized = constraints.normalized();
    let measured = match node {
        LayoutNode::Widget(widget) => Vector2::new(
            normalized.clamp_w(widget.intrinsic.x.max(0.0)),
            normalized.clamp_h(widget.intrinsic.y.max(0.0)),
        ),
        LayoutNode::Container(container) => measure_container(container, normalized, context),
    };
    context.measured.insert(key, measured);
    measured
}

fn measure_container(
    container: &ContainerNode,
    constraints: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    let policy = container.policy;
    let inner_max_w = (constraints.max_w - policy.padding.horizontal()).max(0.0);
    let inner_max_h = (constraints.max_h - policy.padding.vertical()).max(0.0);
    let inner_constraints = Constraints::new(0.0, inner_max_w, 0.0, inner_max_h);
    if container.children.is_empty() {
        return Vector2::new(
            constraints.clamp_w(policy.padding.horizontal()),
            constraints.clamp_h(policy.padding.vertical()),
        );
    }

    let spacing_total =
        policy.spacing.max(0.0) * (container.children.len().saturating_sub(1) as f32);
    let (main, cross) = match policy.kind {
        ContainerKind::Row => measure_linear(true, &container.children, inner_constraints, context),
        ContainerKind::Column => {
            measure_linear(false, &container.children, inner_constraints, context)
        }
        ContainerKind::Stack => measure_stack(&container.children, inner_constraints, context),
    };

    let (size_w, size_h) = match policy.kind {
        ContainerKind::Row => (
            policy.padding.horizontal() + main + spacing_total,
            policy.padding.vertical() + cross,
        ),
        ContainerKind::Column => (
            policy.padding.horizontal() + cross,
            policy.padding.vertical() + main + spacing_total,
        ),
        ContainerKind::Stack => (
            policy.padding.horizontal() + main,
            policy.padding.vertical() + cross,
        ),
    };

    Vector2::new(constraints.clamp_w(size_w), constraints.clamp_h(size_h))
}

fn measure_linear(
    horizontal: bool,
    children: &[SlotChild],
    constraints: Constraints,
    context: &mut LayoutContext,
) -> (f32, f32) {
    let mut main_total = 0.0;
    let mut cross_max: f32 = 0.0;
    for slot_child in children {
        let child_size = measure_node(&slot_child.child, slot_child.slot.constraints, context);
        let slot = slot_child.slot;
        let child_main = resolve_mode_main(horizontal, slot.size_main, child_size, constraints);
        let child_cross = resolve_mode_cross(horizontal, slot.size_cross, child_size, constraints);
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
    (main_total, cross_max)
}

fn measure_stack(
    children: &[SlotChild],
    constraints: Constraints,
    context: &mut LayoutContext,
) -> (f32, f32) {
    let mut max_w: f32 = 0.0;
    let mut max_h: f32 = 0.0;
    for slot_child in children {
        let child_size = measure_node(&slot_child.child, slot_child.slot.constraints, context);
        let width = child_size.x + slot_child.slot.margin.left + slot_child.slot.margin.right;
        let height = child_size.y + slot_child.slot.margin.top + slot_child.slot.margin.bottom;
        max_w = max_w.max(width.min(constraints.max_w));
        max_h = max_h.max(height.min(constraints.max_h));
    }
    (max_w, max_h)
}

fn resolve_mode_main(
    horizontal: bool,
    mode: SizeModeMain,
    measured: Vector2,
    parent: Constraints,
) -> f32 {
    let raw = match mode {
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
    };
    raw.max(0.0)
}

fn resolve_mode_cross(
    horizontal: bool,
    mode: SizeModeCross,
    measured: Vector2,
    parent: Constraints,
) -> f32 {
    match mode {
        SizeModeCross::Fixed(value) => value.max(0.0),
        SizeModeCross::Fill => if horizontal {
            parent.max_h
        } else {
            parent.max_w
        }
        .max(0.0),
        SizeModeCross::Intrinsic => if horizontal { measured.y } else { measured.x }.max(0.0),
    }
}
