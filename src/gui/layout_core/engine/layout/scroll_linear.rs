//! Shared linear virtualization resolution routines for ScrollView.

use super::super::cache::{LinearVirtualMetrics, VirtualSpan};
use super::super::helpers::{
    align_main_offsets, allocate_fill_sizes, compress_if_needed, main_margin_total,
    scale_sizes_to_fit,
};
use super::super::measure::measure_node;
use super::super::{LayoutContext, LayoutDiagnosticCode};
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::model::{OverflowPolicy, SizeModeMain, VirtualizationAxis};
use crate::gui::layout_core::tree::{ContainerNode, LayoutNode, NodeId, SlotChild};
use crate::gui::types::Vector2;
use std::collections::BTreeSet;

/// Resolve full linear sizing/alignment and return virtualizable span metrics.
pub(super) fn build_linear_metrics(
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
    let mut states = collect_layout_states(content, context, horizontal, available_main);

    let fixed_main = states
        .iter()
        .filter(|(slot_child, _, _, _)| !matches!(slot_child.slot.size_main, SizeModeMain::Fill(_)))
        .map(|(_, _, main, _)| *main)
        .sum::<f32>();
    let fill_weight = states
        .iter()
        .filter_map(|(slot_child, _, _, _)| match slot_child.slot.size_main {
            SizeModeMain::Fill(weight) => Some(weight.max(0.0)),
            _ => None,
        })
        .sum::<f32>();
    let margin_total = main_margin_total(horizontal, &states);

    let spacing_total = spacing * (states.len().saturating_sub(1) as f32);
    let remaining = (available_main - fixed_main - margin_total - spacing_total).max(0.0);
    if fill_weight > 0.0 {
        allocate_fill_sizes(horizontal, remaining, fill_weight, &mut states);
    }

    let mut sizes: Vec<f32> = states
        .iter()
        .map(|(_, _, main, fill)| if *fill > 0.0 { *fill } else { *main })
        .collect();

    let mut total_main = sizes.iter().sum::<f32>() + margin_total + spacing_total;
    if total_main > available_main {
        apply_linear_overflow_policy(
            content,
            horizontal,
            available_main,
            spacing_total,
            &states,
            &mut sizes,
            context,
        );
        total_main = sizes.iter().sum::<f32>() + margin_total + spacing_total;
    }

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
    for (index, (slot_child, _, _, _)) in states.iter().enumerate() {
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
        let size = sizes[index].max(0.0);
        spans.push(VirtualSpan {
            start: cursor,
            end: cursor + size,
        });
        main_sizes.push(size);
        cursor += size + margin_after + distributed_spacing;
    }

    LinearVirtualMetrics {
        spans,
        main_sizes,
        total_main,
        leading_offset,
        distributed_spacing,
    }
}

/// Return true when virtualized metrics are safe to consume.
pub(super) fn metrics_is_valid(metrics: &LinearVirtualMetrics, expected_len: usize) -> bool {
    if metrics.spans.len() != expected_len || metrics.main_sizes.len() != expected_len {
        return false;
    }
    if !metrics.total_main.is_finite()
        || !metrics.leading_offset.is_finite()
        || !metrics.distributed_spacing.is_finite()
    {
        return false;
    }
    metrics.spans.iter().all(|span| {
        span.start.is_finite()
            && span.end.is_finite()
            && span.end >= span.start
            && span.start >= 0.0
    })
}

/// Collect all node ids under a content container for precise cache invalidation.
pub(super) fn collect_subtree_ids_from_container(
    container: &ContainerNode,
    out: &mut BTreeSet<NodeId>,
) {
    out.insert(container.id);
    for child in &container.children {
        collect_subtree_ids(&child.child, out);
    }
}

fn collect_layout_states<'a>(
    container: &'a ContainerNode,
    context: &mut LayoutContext,
    horizontal: bool,
    available_main: f32,
) -> Vec<(&'a SlotChild, Vector2, f32, f32)> {
    let mut states = Vec::with_capacity(container.children.len());
    for child in &container.children {
        let measured = measure_node(&child.child, child.slot.constraints, context);
        let main = resolve_main_for_virtual(horizontal, child, measured, available_main, context);
        states.push((child, measured, main, 0.0));
    }
    states
}

#[allow(clippy::too_many_arguments)]
fn apply_linear_overflow_policy(
    container: &ContainerNode,
    horizontal: bool,
    available_main: f32,
    spacing_total: f32,
    states: &[(&SlotChild, Vector2, f32, f32)],
    sizes: &mut [f32],
    context: &mut LayoutContext,
) {
    let policy = container.policy.overflow;
    let margin_total = main_margin_total(horizontal, states);

    match policy {
        OverflowPolicy::Clip => {
            compress_if_needed(horizontal, available_main, states, sizes, spacing_total);
        }
        OverflowPolicy::Scroll => {
            context.push_diagnostic(
                container.id,
                LayoutDiagnosticCode::OverflowOccurred,
                "linear container overflowed and delegated to scroll policy",
            );
        }
        OverflowPolicy::Wrap => {
            context.push_diagnostic(
                container.id,
                LayoutDiagnosticCode::OverflowPolicyDefaulted,
                "overflow wrap policy is unsupported for Row/Column; use ContainerKind::Wrap",
            );
            compress_if_needed(horizontal, available_main, states, sizes, spacing_total);
        }
        OverflowPolicy::Shrink => {
            compress_if_needed(horizontal, available_main, states, sizes, spacing_total);
            scale_sizes_to_fit(available_main, sizes, margin_total, spacing_total);
        }
    }
}

fn resolve_main_for_virtual(
    horizontal: bool,
    slot_child: &SlotChild,
    measured: Vector2,
    available_main: f32,
    context: &mut LayoutContext,
) -> f32 {
    let raw = match slot_child.slot.size_main {
        SizeModeMain::Fixed(value) => value,
        SizeModeMain::Intrinsic => {
            if horizontal {
                measured.x
            } else {
                measured.y
            }
        }
        SizeModeMain::Percent(percent) => available_main * percent.clamp(0.0, 1.0),
        SizeModeMain::Fill(_) => available_main,
    };
    context.clamp_main(
        slot_child.child.id(),
        horizontal,
        slot_child.slot.constraints,
        raw,
    )
}

fn collect_subtree_ids(node: &LayoutNode, out: &mut BTreeSet<NodeId>) {
    out.insert(node.id());
    if let LayoutNode::Container(container) = node {
        for child in &container.children {
            collect_subtree_ids(&child.child, out);
        }
    }
}
