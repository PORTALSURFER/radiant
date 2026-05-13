//! Row and column layout implementation.

mod placement;
mod sizing;

use super::super::helpers::{
    LayoutAxis, align_main_offsets, allocate_fill_sizes, apply_linear_overflow_policy,
    linear_sizing_summary, resolved_main_sizes_into, resolved_main_total,
};
use super::super::{LayoutContext, LayoutDiagnosticCode};
use crate::gui::layout_core::tree::ContainerNode;
use crate::gui::types::Rect;

use placement::{LinearChildSizes, place_linear_children};
use sizing::collect_layout_states;
pub(super) use sizing::{resolve_cross_layout, resolve_nonfill_main};

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
    if total_main > available_main {
        let mut sizes = context.take_linear_sizes();
        resolved_main_sizes_into(&states, &mut sizes);
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
        context.restore_linear_sizes(sizes);
        return;
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
        LinearChildSizes::Resolved,
        leading,
        distributed_spacing,
        context,
    );
}
