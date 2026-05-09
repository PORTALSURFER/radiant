//! Container-specific layout routines for non-linear container kinds.

use super::super::LayoutContext;
use super::super::helpers::place_child_rect;
use super::layout_node;
use super::linear::{resolve_cross_layout, resolve_nonfill_main};
use crate::gui::layout_core::model::{CrossAlign, MainAlign};
use crate::gui::layout_core::tree::ContainerNode;
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

pub(super) fn place_aligned_rect(
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
