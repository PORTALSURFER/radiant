//! Container-specific layout routines for non-linear container kinds.

use super::super::helpers::place_child_rect;
use super::super::{LayoutContext, LayoutDiagnosticCode};
use super::{layout_node, resolve_cross_layout, resolve_nonfill_main};
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::model::{CrossAlign, MainAlign};
use crate::gui::layout_core::tree::{ContainerNode, SlotChild};
use crate::gui::types::{Point, Rect, Vector2};

pub(super) fn layout_stack(container: &ContainerNode, content: Rect, context: &mut LayoutContext) {
    for child in &container.children {
        let measured =
            super::super::measure::measure_node(&child.child, child.slot.constraints, context);
        let width = resolve_cross_layout(
            false,
            child.slot.size_cross,
            measured,
            content.width(),
            child.slot,
            context,
            child.child.id(),
        );
        let height = resolve_cross_layout(
            true,
            child.slot.size_cross,
            measured,
            content.height(),
            child.slot,
            context,
            child.child.id(),
        );
        let align = child
            .slot
            .align_cross_override
            .unwrap_or(container.policy.align_cross);
        let rect = place_child_rect(content, false, 0.0, height, width, child.slot, align);
        context.record_slot_margin(child.child.id(), rect, child.slot.margin);
        layout_node(&child.child, rect, context);
    }
}

pub(super) fn layout_single_fill(
    container: &ContainerNode,
    content: Rect,
    context: &mut LayoutContext,
) {
    let Some(child) = container.children.first() else {
        return;
    };
    let slot = child.slot;
    let width = context.clamp_width(
        child.child.id(),
        slot.constraints,
        content.width() - slot.margin.left - slot.margin.right,
    );
    let height = context.clamp_height(
        child.child.id(),
        slot.constraints,
        content.height() - slot.margin.top - slot.margin.bottom,
    );
    let rect = Rect::from_min_size(
        Point::new(
            content.min.x + slot.margin.left,
            content.min.y + slot.margin.top,
        ),
        Vector2::new(width, height),
    );
    context.record_slot_margin(child.child.id(), rect, slot.margin);
    layout_node(&child.child, rect, context);
}

pub(super) fn layout_align_box(
    container: &ContainerNode,
    content: Rect,
    context: &mut LayoutContext,
) {
    let Some(child) = container.children.first() else {
        return;
    };
    let measured =
        super::super::measure::measure_node(&child.child, child.slot.constraints, context);
    let width = resolve_nonfill_main(
        true,
        child,
        measured,
        content.width(),
        context,
        child.child.id(),
    );
    let height = resolve_nonfill_main(
        false,
        child,
        measured,
        content.height(),
        context,
        child.child.id(),
    );
    let rect = place_aligned_rect(
        content,
        width,
        height,
        container.policy.align_main,
        container.policy.align_cross,
    );
    context.record_slot_margin(child.child.id(), rect, child.slot.margin);
    layout_node(&child.child, rect, context);
}

pub(super) fn layout_aspect_box(
    container: &ContainerNode,
    content: Rect,
    context: &mut LayoutContext,
) {
    let Some(child) = container.children.first() else {
        return;
    };
    let ratio = container.policy.aspect_ratio.unwrap_or(1.0).max(0.0001);
    let (w, h) = fit_aspect_box(content.width(), content.height(), ratio);
    let aspect_rect = place_aligned_rect(
        content,
        w,
        h,
        container.policy.align_main,
        container.policy.align_cross,
    );
    context.record_slot_margin(child.child.id(), aspect_rect, child.slot.margin);
    layout_node(&child.child, aspect_rect, context);
}

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

pub(super) fn layout_scroll_view(
    container: &ContainerNode,
    content: Rect,
    context: &mut LayoutContext,
) {
    let Some(child) = container.children.first() else {
        return;
    };
    let slot = child.slot;
    let measured = super::super::measure::measure_node(&child.child, slot.constraints, context);
    let viewport_w = (content.width() - slot.margin.left - slot.margin.right).max(0.0);
    let viewport_h = (content.height() - slot.margin.top - slot.margin.bottom).max(0.0);
    let width = measured.x.max(viewport_w);
    let height = measured.y.max(viewport_h);
    let max_x = (width - viewport_w).max(0.0);
    let max_y = (height - viewport_h).max(0.0);

    let requested = context.scroll_offset(container.id);
    let mut req_x = requested.x;
    let mut req_y = requested.y;
    let mut invalid = false;
    if !req_x.is_finite() {
        req_x = 0.0;
        invalid = true;
    }
    if !req_y.is_finite() {
        req_y = 0.0;
        invalid = true;
    }
    let clamped_x = req_x.clamp(0.0, max_x);
    let clamped_y = req_y.clamp(0.0, max_y);
    if invalid
        || (clamped_x - req_x).abs() > f32::EPSILON
        || (clamped_y - req_y).abs() > f32::EPSILON
    {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::InvalidScrollOffsetClamped,
            "scroll offset was out of bounds and was clamped",
        );
    }

    let origin = Point::new(
        content.min.x + slot.margin.left - clamped_x,
        content.min.y + slot.margin.top - clamped_y,
    );
    let rect = Rect::from_min_size(origin, Vector2::new(width, height));
    context.record_slot_margin(child.child.id(), rect, slot.margin);
    layout_node(&child.child, rect, context);
    if width > viewport_w || height > viewport_h {
        context.record_overflow(
            container.id,
            container.policy.overflow,
            width > viewport_w,
            height > viewport_h,
        );
    }
}

pub(super) fn layout_wrap(container: &ContainerNode, content: Rect, context: &mut LayoutContext) {
    let item_gap = container.policy.wrap.item_gap.max(0.0);
    let line_gap = container.policy.wrap.line_gap.max(0.0);

    let mut line_x = content.min.x;
    let mut line_y = content.min.y;
    let mut line_h = 0.0;

    for child in &container.children {
        let measured =
            super::super::measure::measure_node(&child.child, child.slot.constraints, context);
        let width = resolve_nonfill_main(
            true,
            child,
            measured,
            content.width(),
            context,
            child.child.id(),
        );
        let height = resolve_nonfill_main(
            false,
            child,
            measured,
            content.height(),
            context,
            child.child.id(),
        );
        let span_w = width + child.slot.margin.left + child.slot.margin.right;

        if line_x > content.min.x && (line_x + span_w) > content.max.x {
            line_x = content.min.x;
            line_y += line_h + line_gap;
            line_h = 0.0;
        }

        let item_rect = Rect::from_min_size(
            Point::new(
                line_x + child.slot.margin.left,
                line_y + child.slot.margin.top,
            ),
            Vector2::new(width, height),
        );
        context.record_slot_margin(child.child.id(), item_rect, child.slot.margin);
        layout_node(&child.child, item_rect, context);
        line_x += span_w + item_gap;
        line_h = line_h.max(height + child.slot.margin.top + child.slot.margin.bottom);
    }

    if (line_y + line_h) > content.max.y {
        context.record_overflow(container.id, container.policy.overflow, false, true);
    }
}

pub(super) fn layout_switch(container: &ContainerNode, content: Rect, context: &mut LayoutContext) {
    let Some(index) = select_switch_child(container, content.width()) else {
        return;
    };
    let Some(child) = container.children.get(index) else {
        return;
    };
    layout_node(&child.child, content, context);
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

fn place_aligned_rect(
    content: Rect,
    width: f32,
    height: f32,
    main_align: MainAlign,
    cross_align: CrossAlign,
) -> Rect {
    let x = match cross_align {
        CrossAlign::Start | CrossAlign::Stretch => content.min.x,
        CrossAlign::Center => content.min.x + ((content.width() - width) * 0.5),
        CrossAlign::End => content.max.x - width,
    };
    let y = match main_align {
        MainAlign::Start
        | MainAlign::SpaceBetween
        | MainAlign::SpaceAround
        | MainAlign::SpaceEvenly => content.min.y,
        MainAlign::Center => content.min.y + ((content.height() - height) * 0.5),
        MainAlign::End => content.max.y - height,
    };
    let resolved_w = if matches!(cross_align, CrossAlign::Stretch) {
        content.width()
    } else {
        width
    };
    Rect::from_min_size(
        Point::new(x, y),
        Vector2::new(resolved_w.max(0.0), height.max(0.0)),
    )
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
