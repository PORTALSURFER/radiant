//! Grid container measurement strategy.

use super::super::measure_node;
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::engine::LayoutContext;
use crate::gui::layout_core::tree::ContainerNode;
use crate::gui::types::Vector2;

pub(super) fn measure_grid(
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
