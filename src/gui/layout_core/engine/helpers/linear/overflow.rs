//! Linear overflow policy and compression helpers.

use super::sizing::{LinearLayoutState, main_margin_total};
use crate::gui::layout_core::engine::{LayoutContext, LayoutDiagnosticCode};
use crate::gui::layout_core::model::{OverflowPolicy, SizeModeMain};
use crate::gui::layout_core::tree::ContainerNode;

pub(in crate::gui::layout_core::engine) fn apply_linear_overflow_policy(
    container: &ContainerNode,
    horizontal: bool,
    available_main: f32,
    spacing_total: f32,
    states: &[LinearLayoutState<'_>],
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

fn compress_if_needed(
    horizontal: bool,
    available_main: f32,
    states: &[LinearLayoutState<'_>],
    sizes: &mut [f32],
    spacing_total: f32,
) {
    let mut overflow =
        sizes.iter().sum::<f32>() + spacing_total + main_margin_total(horizontal, states)
            - available_main;
    if overflow <= 0.0 {
        return;
    }

    reduce_group(
        horizontal,
        |mode, _| matches!(mode, SizeModeMain::Fill(_)),
        states,
        &mut overflow,
        sizes,
    );
    if overflow > f32::EPSILON {
        reduce_group(
            horizontal,
            |mode, _| matches!(mode, SizeModeMain::Intrinsic),
            states,
            &mut overflow,
            sizes,
        );
    }
    if overflow > f32::EPSILON {
        reduce_group(
            horizontal,
            |mode, allow_fixed| matches!(mode, SizeModeMain::Fixed(_)) && allow_fixed,
            states,
            &mut overflow,
            sizes,
        );
    }
}

fn scale_sizes_to_fit(
    available_main: f32,
    sizes: &mut [f32],
    main_margin_total: f32,
    spacing_total: f32,
) {
    let main_budget = (available_main - main_margin_total - spacing_total).max(0.0);
    let total = sizes.iter().sum::<f32>();
    if total <= f32::EPSILON || total <= main_budget {
        return;
    }
    let scale = (main_budget / total).clamp(0.0, 1.0);
    for size in sizes.iter_mut() {
        *size *= scale;
    }
}

fn reduce_group(
    horizontal: bool,
    predicate: fn(SizeModeMain, bool) -> bool,
    states: &[LinearLayoutState<'_>],
    overflow: &mut f32,
    sizes: &mut [f32],
) {
    for (index, state) in states.iter().enumerate() {
        let mode = state.slot_child.slot.size_main;
        if !predicate(mode, state.slot_child.slot.allow_fixed_compress) {
            continue;
        }
        let min_main = if horizontal {
            state.slot_child.slot.constraints.min_w
        } else {
            state.slot_child.slot.constraints.min_h
        };
        let reducible = (sizes[index] - min_main).max(0.0);
        if reducible <= 0.0 {
            continue;
        }
        let delta = reducible.min(*overflow);
        sizes[index] -= delta;
        *overflow -= delta;
        if *overflow <= f32::EPSILON {
            break;
        }
    }
}
