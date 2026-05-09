//! Grid container layout strategy.

use super::boxes::place_aligned_rect;
use super::layout_node;
use super::linear::resolve_nonfill_main;
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::engine::LayoutContext;
use crate::gui::layout_core::tree::{ContainerNode, SlotChild};
use crate::gui::types::{Point, Rect, Vector2};

pub(super) fn layout_grid(container: &ContainerNode, content: Rect, context: &mut LayoutContext) {
    if container.children.is_empty() {
        return;
    }

    let columns = container.policy.grid.columns.max(1);
    let column_gap = container.policy.grid.column_gap.max(0.0);
    let row_gap = container.policy.grid.row_gap.max(0.0);
    let cell_w = ((content.width() - (column_gap * (columns.saturating_sub(1) as f32)))
        / columns as f32)
        .max(0.0);

    let mut max_cell_h: f32 = 0.0;
    let mut measured_children = Vec::with_capacity(container.children.len());
    for child in &container.children {
        let measured = super::super::measure::measure_node(
            &child.child,
            ConstraintsForGrid::for_cell(child, cell_w, content.height()),
            context,
        );
        max_cell_h = max_cell_h.max(measured.y + child.slot.margin.top + child.slot.margin.bottom);
        measured_children.push((child, measured));
    }

    for (index, (child, measured)) in measured_children.into_iter().enumerate() {
        let row = index / columns;
        let col = index % columns;
        let cell_x = content.min.x + (col as f32 * (cell_w + column_gap));
        let cell_y = content.min.y + (row as f32 * (max_cell_h + row_gap));
        let cell =
            Rect::from_min_size(Point::new(cell_x, cell_y), Vector2::new(cell_w, max_cell_h));

        let width = resolve_nonfill_main(
            true,
            child,
            measured,
            cell.width(),
            context,
            child.child.id(),
        );
        let height = resolve_nonfill_main(
            false,
            child,
            measured,
            cell.height(),
            context,
            child.child.id(),
        );
        let rect = place_aligned_rect(
            cell,
            width,
            height,
            container.policy.align_main,
            child
                .slot
                .align_cross_override
                .unwrap_or(container.policy.align_cross),
        );
        context.record_slot_margin(child.child.id(), rect, child.slot.margin);
        layout_node(&child.child, rect, context);
    }

    let rows = container.children.len().div_ceil(columns);
    let used_h = (max_cell_h * rows as f32) + (row_gap * (rows.saturating_sub(1) as f32));
    if used_h > content.height() {
        context.record_overflow(container.id, container.policy.overflow, false, true);
    }
}

struct ConstraintsForGrid;

impl ConstraintsForGrid {
    fn for_cell(child: &SlotChild, cell_w: f32, cell_h: f32) -> Constraints {
        let slot = child.slot;
        Constraints::new(
            slot.constraints.min_w,
            slot.constraints.max_w.min(cell_w),
            slot.constraints.min_h,
            slot.constraints.max_h.min(cell_h),
        )
    }
}
