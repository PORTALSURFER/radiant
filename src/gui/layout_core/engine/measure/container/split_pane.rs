//! Static two-pane split measurement.

use super::super::measure_node;
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::engine::LayoutContext;
use crate::gui::layout_core::tree::ContainerNode;
use crate::gui::panel::{SplitPaneAxis, SplitPaneLayout, SplitPaneLayoutParts};
use crate::gui::types::{Rect, Vector2};

pub(super) fn measure_split_pane(
    container: &ContainerNode,
    constraints: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    // SplitPane is the current product-neutral observation boundary. The
    // state value is intentionally ignored until a later contract gives it a
    // geometry consumer.
    let _ = context.container_state_read(container.id);
    if container.children.len() != 2 {
        return measure_malformed_children(container, constraints, context);
    }

    let first = measure_node(
        &container.children[0].child,
        container.children[0].slot.constraints,
        context,
    );
    let second = measure_node(
        &container.children[1].child,
        container.children[1].slot.constraints,
        context,
    );
    let policy = container.policy.split_pane;
    let normalized = SplitPaneLayout::from_parts(SplitPaneLayoutParts {
        bounds: probe_bounds(constraints, policy.axis),
        axis: policy.axis,
        ratio: policy.initial_ratio,
        divider_extent: policy.divider_extent,
        first_min_extent: policy.first_min_extent,
        second_min_extent: policy.second_min_extent,
    });
    let first_main = main_extent(first, policy.axis);
    let second_main = main_extent(second, policy.axis);
    let first_required = first_main.max(normalized.first_min_extent);
    let second_required = second_main.max(normalized.second_min_extent);
    let main = saturating_add(
        saturating_add(first_required, normalized.divider_extent),
        second_required,
    );
    let cross = cross_extent(first, second, policy.axis);

    match policy.axis {
        SplitPaneAxis::Horizontal => {
            Vector2::new(constraints.clamp_w(main), constraints.clamp_h(cross))
        }
        SplitPaneAxis::Vertical => {
            Vector2::new(constraints.clamp_w(cross), constraints.clamp_h(main))
        }
    }
}

fn measure_malformed_children(
    container: &ContainerNode,
    constraints: Constraints,
    context: &mut LayoutContext,
) -> Vector2 {
    let mut width: f32 = 0.0;
    let mut height: f32 = 0.0;
    for child in &container.children {
        let measured = measure_node(&child.child, child.slot.constraints, context);
        width = width.max(finite_nonnegative(measured.x));
        height = height.max(finite_nonnegative(measured.y));
    }
    Vector2::new(constraints.clamp_w(width), constraints.clamp_h(height))
}

fn probe_bounds(constraints: Constraints, axis: SplitPaneAxis) -> Rect {
    let main = match axis {
        SplitPaneAxis::Horizontal => finite_probe_extent(constraints.max_w),
        SplitPaneAxis::Vertical => finite_probe_extent(constraints.max_h),
    };
    match axis {
        SplitPaneAxis::Horizontal => Rect::from_size(main, 0.0),
        SplitPaneAxis::Vertical => Rect::from_size(0.0, main),
    }
}

fn finite_probe_extent(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        f32::MAX
    }
}

fn main_extent(size: Vector2, axis: SplitPaneAxis) -> f32 {
    match axis {
        SplitPaneAxis::Horizontal => finite_nonnegative(size.x),
        SplitPaneAxis::Vertical => finite_nonnegative(size.y),
    }
}

fn cross_extent(first: Vector2, second: Vector2, axis: SplitPaneAxis) -> f32 {
    match axis {
        SplitPaneAxis::Horizontal => finite_nonnegative(first.y).max(finite_nonnegative(second.y)),
        SplitPaneAxis::Vertical => finite_nonnegative(first.x).max(finite_nonnegative(second.x)),
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn saturating_add(left: f32, right: f32) -> f32 {
    if left >= f32::MAX - right {
        f32::MAX
    } else {
        left + right
    }
}
