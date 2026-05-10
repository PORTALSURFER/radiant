//! Row and column layout implementation.

use super::super::helpers::{
    LayoutAxis, LinearLayoutState, align_main_offsets, allocate_fill_sizes,
    apply_linear_overflow_policy, linear_sizing_summary, place_child_rect, resolved_main_sizes,
};
use super::super::{LayoutContext, LayoutDiagnosticCode};
use super::layout_node;
use crate::gui::layout_core::model::{SizeModeCross, SizeModeMain, SlotParams};
use crate::gui::layout_core::tree::{ContainerNode, NodeId, SlotChild};
use crate::gui::types::{Rect, Vector2};

pub(super) fn layout_linear(
    container: &ContainerNode,
    content: Rect,
    horizontal: bool,
    context: &mut LayoutContext,
) {
    if container.children.is_empty() {
        return;
    }
    let axis = LayoutAxis::from_horizontal(horizontal);
    let policy = &container.policy;
    let spacing = policy.spacing.max(0.0);
    let available_main = axis.main_extent(content).max(0.0);
    let available_cross = axis.cross_extent(content).max(0.0);

    if let Some(window) = context.linear_window(container.id) {
        let states = collect_layout_states(
            container,
            context,
            horizontal,
            available_main,
            window.first,
            window.last_exclusive,
        );
        let cursor_main_start = window.cursor_main_start;
        let distributed_spacing = window.metrics.distributed_spacing;
        if let Some(uniform) = window.metrics.uniform {
            if window.last_exclusive <= uniform.count {
                place_linear_children(
                    container,
                    content,
                    horizontal,
                    available_cross,
                    &states,
                    LinearChildSizes::Uniform {
                        main_size: uniform.main_size,
                        len: states.len(),
                    },
                    cursor_main_start,
                    distributed_spacing,
                    context,
                );
                return;
            }
        } else {
            let sizes = if window.first <= window.last_exclusive
                && window.last_exclusive <= window.metrics.main_sizes.len()
            {
                &window.metrics.main_sizes[window.first..window.last_exclusive]
            } else {
                &[]
            };
            if sizes.len() == states.len() {
                place_linear_children(
                    container,
                    content,
                    horizontal,
                    available_cross,
                    &states,
                    LinearChildSizes::Slice(sizes),
                    cursor_main_start,
                    distributed_spacing,
                    context,
                );
                return;
            }
        }
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::VirtualizationSpanResolutionFallback,
            "virtualization window sizes did not match materialized children",
        );
    }

    let mut states = collect_layout_states(
        container,
        context,
        horizontal,
        available_main,
        0,
        container.children.len(),
    );
    let summary = linear_sizing_summary(horizontal, &states);

    let spacing_total = spacing * (states.len().saturating_sub(1) as f32);
    let remaining =
        (available_main - summary.fixed_main - summary.margin_total - spacing_total).max(0.0);
    if summary.fill_weight > 0.0 {
        allocate_fill_sizes(horizontal, remaining, summary.fill_weight, &mut states);
    }

    let (mut sizes, mut total_main) = resolved_main_sizes(&states);
    total_main += summary.margin_total + spacing_total;
    if total_main > available_main {
        apply_linear_overflow_policy(
            container,
            horizontal,
            available_main,
            spacing_total,
            &states,
            &mut sizes,
            context,
        );
        total_main = sizes.iter().sum::<f32>() + summary.margin_total + spacing_total;
    }

    let (leading, distributed_spacing) = align_main_offsets(
        policy.align_main,
        available_main,
        total_main,
        spacing,
        states.len(),
    );
    place_linear_children(
        container,
        content,
        horizontal,
        available_cross,
        &states,
        LinearChildSizes::Slice(&sizes),
        leading,
        distributed_spacing,
        context,
    );

    if total_main > available_main {
        let (x, y) = axis.overflow_flags();
        context.record_overflow(container.id, policy.overflow, x, y);
    }
}

enum LinearChildSizes<'a> {
    Slice(&'a [f32]),
    Uniform { main_size: f32, len: usize },
}

impl LinearChildSizes<'_> {
    fn len(&self) -> usize {
        match self {
            Self::Slice(sizes) => sizes.len(),
            Self::Uniform { len, .. } => *len,
        }
    }

    fn get(&self, index: usize) -> Option<f32> {
        match self {
            Self::Slice(sizes) => sizes.get(index).copied(),
            Self::Uniform { main_size, len } => (index < *len).then_some(*main_size),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_layout_states<'a>(
    container: &'a ContainerNode,
    context: &mut LayoutContext,
    horizontal: bool,
    available_main: f32,
    start_index: usize,
    end_index_exclusive: usize,
) -> Vec<LinearLayoutState<'a>> {
    let clamped_start = start_index.min(container.children.len());
    let clamped_end = end_index_exclusive.min(container.children.len());
    let selected = &container.children[clamped_start..clamped_end];
    let mut states = Vec::with_capacity(selected.len());
    for child in selected {
        let measured =
            super::super::measure::measure_node(&child.child, child.slot.constraints, context);
        let main = resolve_nonfill_main(
            horizontal,
            child,
            measured,
            available_main,
            context,
            child.child.id(),
        );
        states.push(LinearLayoutState::new(child, measured, main));
    }
    states
}

#[allow(clippy::too_many_arguments)]
fn place_linear_children(
    container: &ContainerNode,
    content: Rect,
    horizontal: bool,
    available_cross: f32,
    states: &[LinearLayoutState<'_>],
    sizes: LinearChildSizes<'_>,
    leading: f32,
    distributed_spacing: f32,
    context: &mut LayoutContext,
) {
    if sizes.len() != states.len() {
        return;
    }
    let mut cursor = leading;
    for (index, state) in states.iter().enumerate() {
        let slot_child = state.slot_child;
        let slot = slot_child.slot;
        let main_margin_before = if horizontal {
            slot.margin.left
        } else {
            slot.margin.top
        };
        let main_margin_after = if horizontal {
            slot.margin.right
        } else {
            slot.margin.bottom
        };
        cursor += main_margin_before;
        let Some(child_main) = sizes.get(index).map(|size| size.max(0.0)) else {
            return;
        };
        let child_cross = resolve_cross_layout(
            horizontal,
            slot.size_cross,
            state.measured,
            available_cross,
            slot,
            context,
            slot_child.child.id(),
        );
        let cross_align = slot
            .align_cross_override
            .unwrap_or(container.policy.align_cross);
        let child_rect = place_child_rect(
            content,
            horizontal,
            cursor,
            child_main,
            child_cross,
            slot,
            cross_align,
        );
        context.record_slot_margin(slot_child.child.id(), child_rect, slot.margin);
        layout_node(&slot_child.child, child_rect, context);
        cursor += child_main + main_margin_after + distributed_spacing;
    }
}

pub(super) fn resolve_nonfill_main(
    horizontal: bool,
    slot_child: &SlotChild,
    measured: Vector2,
    available_main: f32,
    context: &mut LayoutContext,
    node_id: NodeId,
) -> f32 {
    let slot = slot_child.slot;
    let raw = match slot.size_main {
        SizeModeMain::Fixed(value) => value,
        SizeModeMain::Percent(percent) => available_main * percent.clamp(0.0, 1.0),
        SizeModeMain::Intrinsic => {
            if horizontal {
                measured.x
            } else {
                measured.y
            }
        }
        SizeModeMain::Fill(_) => available_main,
    };
    context.clamp_main(node_id, horizontal, slot.constraints, raw)
}

pub(super) fn resolve_cross_layout(
    horizontal: bool,
    mode: SizeModeCross,
    measured: Vector2,
    available_cross: f32,
    slot: SlotParams,
    context: &mut LayoutContext,
    node_id: NodeId,
) -> f32 {
    let raw = match mode {
        SizeModeCross::Fixed(value) => value,
        SizeModeCross::Fill => available_cross,
        SizeModeCross::Intrinsic => {
            if horizontal {
                measured.y
            } else {
                measured.x
            }
        }
    };
    context.clamp_cross(node_id, horizontal, slot.constraints, raw)
}
