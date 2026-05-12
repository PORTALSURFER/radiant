//! Shared linear virtualization resolution routines for ScrollView.

use super::super::LayoutContext;
use super::super::cache::{LinearVirtualMetrics, UniformVirtualMetrics, VirtualSpan};
use super::super::helpers::{
    LinearLayoutState, align_main_offsets, allocate_fill_sizes, apply_linear_overflow_policy,
    linear_sizing_summary, resolved_main_sizes_into,
};
use super::super::measure::measure_node;
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::model::{SizeModeMain, VirtualizationAxis};
use crate::gui::layout_core::tree::{ContainerNode, LayoutNode, SlotChild, WidgetNode};
use crate::gui::types::Vector2;

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

fn resolved_main_total(states: &[LinearLayoutState<'_>]) -> f32 {
    states.iter().map(resolved_main_size).sum()
}

fn resolved_main_size(state: &LinearLayoutState<'_>) -> f32 {
    if state.fill > 0.0 {
        state.fill
    } else {
        state.main
    }
}

fn build_uniform_linear_metrics(
    content: &ContainerNode,
    axis: VirtualizationAxis,
    context: &mut LayoutContext,
    available_main: f32,
    spacing: f32,
) -> Option<LinearVirtualMetrics> {
    let count = content.children.len();
    if count == 0 {
        return None;
    }
    let horizontal = matches!(axis, VirtualizationAxis::Horizontal);
    let mut uniform_main: Option<f32> = None;
    if let Some(main_size) = known_uniform_linear_main_size(content, axis) {
        let spacing_total = spacing * count.saturating_sub(1) as f32;
        let total_main = main_size * count as f32 + spacing_total;
        if total_main > available_main + f32::EPSILON {
            return None;
        }
        let (leading_offset, distributed_spacing) = align_main_offsets(
            content.policy.align_main,
            available_main,
            total_main,
            spacing,
            count,
        );
        return Some(LinearVirtualMetrics {
            spans: Vec::new(),
            main_sizes: Vec::new(),
            uniform: Some(UniformVirtualMetrics {
                count,
                main_size,
                step: main_size + distributed_spacing,
            }),
            total_main,
            leading_offset,
            distributed_spacing,
        });
    }
    for child in &content.children {
        if main_margin_total_for_slot(horizontal, child) > 0.0 {
            return None;
        }
        let raw = match child.slot.size_main {
            SizeModeMain::Fixed(value) => value,
            SizeModeMain::Intrinsic => {
                let measured = direct_widget_intrinsic_size(&child.child)?;
                if horizontal { measured.x } else { measured.y }
            }
            SizeModeMain::Percent(_) | SizeModeMain::Fill(_) => return None,
        };
        let main = context.clamp_main(
            child.child.id(),
            horizontal,
            child.slot.constraints,
            raw.max(0.0),
        );
        if !main.is_finite() {
            return None;
        }
        match uniform_main {
            Some(expected) if (expected - main).abs() > f32::EPSILON => return None,
            Some(_) => {}
            None => uniform_main = Some(main),
        }
    }

    let main_size = uniform_main?;
    let spacing_total = spacing * count.saturating_sub(1) as f32;
    let total_main = main_size * count as f32 + spacing_total;
    if total_main > available_main + f32::EPSILON {
        return None;
    }
    let (leading_offset, distributed_spacing) = align_main_offsets(
        content.policy.align_main,
        available_main,
        total_main,
        spacing,
        count,
    );
    Some(LinearVirtualMetrics {
        spans: Vec::new(),
        main_sizes: Vec::new(),
        uniform: Some(UniformVirtualMetrics {
            count,
            main_size,
            step: main_size + distributed_spacing,
        }),
        total_main,
        leading_offset,
        distributed_spacing,
    })
}

fn main_margin_total_for_slot(horizontal: bool, child: &SlotChild) -> f32 {
    if horizontal {
        child.slot.margin.left + child.slot.margin.right
    } else {
        child.slot.margin.top + child.slot.margin.bottom
    }
}

/// Resolve the main-axis content extent for virtualized fixed-size children
/// without measuring every child.
pub(super) fn known_linear_main_extent(
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

fn known_uniform_linear_main_size(
    content: &ContainerNode,
    axis: VirtualizationAxis,
) -> Option<f32> {
    let horizontal = matches!(axis, VirtualizationAxis::Horizontal);
    if horizontal {
        content.known_uniform_main_horizontal
    } else {
        content.known_uniform_main_vertical
    }
}

/// Return true when virtualized metrics are safe to consume.
pub(super) fn metrics_is_valid(metrics: &LinearVirtualMetrics, expected_len: usize) -> bool {
    if metrics.len() != expected_len {
        return false;
    }
    if metrics.uniform.is_none()
        && (metrics.spans.len() != expected_len || metrics.main_sizes.len() != expected_len)
    {
        return false;
    }
    if !metrics.total_main.is_finite()
        || !metrics.leading_offset.is_finite()
        || !metrics.distributed_spacing.is_finite()
    {
        return false;
    }
    if let Some(uniform) = metrics.uniform {
        return uniform.main_size.is_finite()
            && uniform.main_size >= 0.0
            && uniform.step.is_finite()
            && uniform.step >= 0.0;
    }
    metrics.spans.iter().all(|span| {
        span.start.is_finite()
            && span.end.is_finite()
            && span.end >= span.start
            && span.start >= 0.0
    })
}

fn collect_layout_states<'a>(
    container: &'a ContainerNode,
    context: &mut LayoutContext,
    horizontal: bool,
    available_main: f32,
) -> Vec<LinearLayoutState<'a>> {
    let mut states = Vec::with_capacity(container.children.len());
    for child in &container.children {
        let measured = if matches!(child.slot.size_main, SizeModeMain::Intrinsic) {
            direct_widget_intrinsic_size(&child.child)
                .unwrap_or_else(|| measure_node(&child.child, child.slot.constraints, context))
        } else {
            Vector2::default()
        };
        let main = resolve_main_for_virtual(horizontal, child, measured, available_main, context);
        states.push(LinearLayoutState::new(child, measured, main));
    }
    states
}

fn direct_widget_intrinsic_size(node: &LayoutNode) -> Option<Vector2> {
    let LayoutNode::Widget(WidgetNode { intrinsic, .. }) = node else {
        return None;
    };
    (intrinsic.x.is_finite() && intrinsic.y.is_finite())
        .then_some(Vector2::new(intrinsic.x.max(0.0), intrinsic.y.max(0.0)))
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
