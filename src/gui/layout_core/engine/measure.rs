//! Measure pass implementation for strict slot-based layout trees.

use super::{LayoutContext, MeasureCacheKey};
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::model::{ContainerKind, SizeModeCross, SizeModeMain};
use crate::gui::layout_core::tree::{ContainerNode, LayoutNode, SlotChild};
use crate::gui::types::Vector2;

pub(super) fn measure_node(
    node: &LayoutNode,
    constraints: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    let normalized = context.normalize_constraints(node.id(), constraints);
    let key = MeasureCacheKey::new(node, normalized);
    if let Some(size) = context.cached_measure(key, node.id()) {
        return size;
    }

    let measured = match node {
        LayoutNode::Widget(widget) => Vector2::new(
            context.clamp_width(widget.id, normalized, widget.intrinsic.x),
            context.clamp_height(widget.id, normalized, widget.intrinsic.y),
        ),
        LayoutNode::Container(container) => measure_container(container, normalized, context),
    };
    context.remember_measure(key, measured);
    measured
}

fn measure_container(
    container: &ContainerNode,
    constraints: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    let policy = &container.policy;
    let inner = context.normalize_constraints(
        container.id,
        constraints.inset(
            policy.padding.horizontal() * 0.5,
            policy.padding.vertical() * 0.5,
        ),
    );
    let measured_inner = match policy.kind {
        ContainerKind::Row => {
            measure_linear(true, &container.children, inner, policy.spacing, context)
        }
        ContainerKind::Column => {
            measure_linear(false, &container.children, inner, policy.spacing, context)
        }
        ContainerKind::Stack | ContainerKind::AlignBox | ContainerKind::PaddingBox => {
            measure_stack(&container.children, inner, context)
        }
        ContainerKind::AspectBox => measure_aspect_box(container, inner, context),
        ContainerKind::Grid => measure_grid(container, inner, context),
        ContainerKind::ScrollView => measure_scroll_view(container, inner, context),
        ContainerKind::Wrap => measure_wrap(container, inner, context),
        ContainerKind::SwitchLayout => measure_switch_layout(container, inner, context),
    };

    Vector2::new(
        constraints.clamp_w(measured_inner.x + policy.padding.horizontal()),
        constraints.clamp_h(measured_inner.y + policy.padding.vertical()),
    )
}

fn measure_linear(
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

fn measure_stack(
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

fn measure_aspect_box(
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

fn measure_grid(
    container: &ContainerNode,
    constraints: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    if container.children.is_empty() {
        return Vector2::new(0.0, 0.0);
    }

    let columns = container.policy.grid.columns.max(1);
    let column_gap = container.policy.grid.column_gap.max(0.0);
    let row_gap = container.policy.grid.row_gap.max(0.0);
    let available_w = constraints.max_w.max(0.0);
    let cell_w = ((available_w - (column_gap * (columns.saturating_sub(1) as f32)))
        / columns as f32)
        .max(0.0);

    let mut cell_h: f32 = 0.0;
    for child in &container.children {
        let measured = measure_node(
            &child.child,
            Constraints::new(0.0, cell_w, 0.0, constraints.max_h),
            context,
        );
        cell_h = cell_h.max(measured.y + child.slot.margin.top + child.slot.margin.bottom);
    }

    let rows = container.children.len().div_ceil(columns);
    let used_w = (cell_w * columns as f32) + (column_gap * (columns.saturating_sub(1) as f32));
    let used_h = (cell_h * rows as f32) + (row_gap * (rows.saturating_sub(1) as f32));
    Vector2::new(used_w.min(constraints.max_w), used_h.min(constraints.max_h))
}

fn measure_scroll_view(
    container: &ContainerNode,
    constraints: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    if let Some(child) = container.children.first() {
        let _ = measure_node(&child.child, child.slot.constraints, context);
    }
    Vector2::new(constraints.max_w, constraints.max_h)
}

fn measure_wrap(
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

fn measure_switch_layout(
    container: &ContainerNode,
    constraints: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    let selected = select_switch_child(container, constraints.max_w);
    if let Some(index) = selected {
        if let Some(child) = container.children.get(index) {
            return measure_node(&child.child, child.slot.constraints, context);
        }
    }
    Vector2::new(0.0, 0.0)
}

fn select_switch_child(container: &ContainerNode, width: f32) -> Option<usize> {
    if container.children.is_empty() {
        return None;
    }
    if container.policy.switch_breakpoints.is_empty() {
        return Some(0);
    }

    for (index, breakpoint) in container.policy.switch_breakpoints.iter().enumerate() {
        if breakpoint.contains(width) && index < container.children.len() {
            return Some(index);
        }
    }
    Some(0)
}

fn fit_aspect_box(max_w: f32, max_h: f32, ratio: f32) -> (f32, f32) {
    if max_w <= 0.0 || max_h <= 0.0 {
        return (0.0, 0.0);
    }
    let by_width_h = max_w / ratio;
    if by_width_h <= max_h {
        return (max_w, by_width_h.max(0.0));
    }
    let by_height_w = max_h * ratio;
    (by_height_w.max(0.0), max_h)
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
