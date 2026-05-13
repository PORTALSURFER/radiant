//! Linear virtual metrics construction for scrollable row and column content.

use super::state::collect_layout_states;
use super::uniform::build_uniform_linear_metrics;
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::engine::LayoutContext;
use crate::gui::layout_core::engine::cache::{LinearVirtualMetrics, VirtualSpan};
use crate::gui::layout_core::engine::helpers::{
    align_main_offsets, allocate_fill_sizes, apply_linear_overflow_policy, linear_sizing_summary,
    resolved_main_size, resolved_main_sizes_into, resolved_main_total,
};
use crate::gui::layout_core::model::VirtualizationAxis;
use crate::gui::layout_core::tree::ContainerNode;

/// Resolve full linear sizing/alignment and return virtualizable span metrics.
pub(in crate::gui::layout_core::engine::layout) fn build_linear_metrics(
    content: &ContainerNode,
    constraints: Constraints,
    axis: VirtualizationAxis,
    context: &mut LayoutContext,
) -> LinearVirtualMetrics {
    let horizontal = matches!(axis, VirtualizationAxis::Horizontal);
    let available_main = if horizontal {
        constraints.max_w
    } else {
        constraints.max_h
    }
    .max(0.0);

    let spacing = content.policy.spacing.max(0.0);
    if let Some(metrics) =
        build_uniform_linear_metrics(content, axis, context, available_main, spacing)
    {
        return metrics;
    }

    let mut states = collect_layout_states(content, context, horizontal, available_main);

    let summary = linear_sizing_summary(horizontal, &states);

    let spacing_total = spacing * (states.len().saturating_sub(1) as f32);
    let remaining =
        (available_main - summary.fixed_main - summary.margin_total - spacing_total).max(0.0);
    if summary.fill_weight > 0.0 {
        let mut unresolved = context.take_linear_unresolved();
        allocate_fill_sizes(
            horizontal,
            remaining,
            summary.fill_weight,
            &mut states,
            &mut unresolved,
        );
        context.restore_linear_unresolved(unresolved);
    }

    let mut total_main = resolved_main_total(&states);
    total_main += summary.margin_total + spacing_total;
    let adjusted_sizes = if total_main > available_main {
        let mut sizes = context.take_linear_sizes();
        resolved_main_sizes_into(&states, &mut sizes);
        apply_linear_overflow_policy(
            content,
            horizontal,
            available_main,
            spacing_total,
            &states,
            &mut sizes,
            context,
        );
        total_main = sizes.iter().sum::<f32>() + summary.margin_total + spacing_total;
        Some(sizes)
    } else {
        None
    };

    let (leading_offset, distributed_spacing) = align_main_offsets(
        content.policy.align_main,
        available_main,
        total_main,
        spacing,
        states.len(),
    );

    let mut spans = Vec::with_capacity(states.len());
    let mut main_sizes = Vec::with_capacity(states.len());
    let mut cursor = leading_offset;
    for (index, state) in states.iter().enumerate() {
        let slot_child = state.slot_child;
        let margin_before = if horizontal {
            slot_child.slot.margin.left
        } else {
            slot_child.slot.margin.top
        };
        let margin_after = if horizontal {
            slot_child.slot.margin.right
        } else {
            slot_child.slot.margin.bottom
        };
        cursor += margin_before;
        let size = adjusted_sizes
            .as_ref()
            .map_or_else(|| resolved_main_size(state), |sizes| sizes[index])
            .max(0.0);
        spans.push(VirtualSpan {
            start: cursor,
            end: cursor + size,
        });
        main_sizes.push(size);
        cursor += size + margin_after + distributed_spacing;
    }

    if let Some(sizes) = adjusted_sizes {
        context.restore_linear_sizes(sizes);
    }

    LinearVirtualMetrics {
        spans,
        main_sizes,
        uniform: None,
        total_main,
        leading_offset,
        distributed_spacing,
    }
}

/// Resolve the main-axis content extent for virtualized fixed-size children
/// without measuring every child.
pub(in crate::gui::layout_core::engine::layout) fn known_linear_main_extent(
    content: &ContainerNode,
    axis: VirtualizationAxis,
) -> Option<f32> {
    let horizontal = matches!(axis, VirtualizationAxis::Horizontal);
    if horizontal {
        content.known_main_extent_horizontal
    } else {
        content.known_main_extent_vertical
    }
}
