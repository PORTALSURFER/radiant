//! Shared linear sizing and overflow helpers.

use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::engine::{LayoutContext, LayoutDiagnosticCode};
use crate::gui::layout_core::model::{MainAlign, OverflowPolicy, SizeModeMain};
use crate::gui::layout_core::tree::{ContainerNode, SlotChild};
use crate::gui::types::Vector2;

pub(in crate::gui::layout_core::engine) struct LinearLayoutState<'a> {
    pub slot_child: &'a SlotChild,
    pub measured: Vector2,
    pub main: f32,
    pub fill: f32,
}

pub(in crate::gui::layout_core::engine) struct LinearSizingSummary {
    pub fixed_main: f32,
    pub fill_weight: f32,
    pub margin_total: f32,
}

impl<'a> LinearLayoutState<'a> {
    pub fn new(slot_child: &'a SlotChild, measured: Vector2, main: f32) -> Self {
        Self {
            slot_child,
            measured,
            main,
            fill: 0.0,
        }
    }
}

pub(in crate::gui::layout_core::engine) fn linear_sizing_summary(
    horizontal: bool,
    states: &[LinearLayoutState<'_>],
) -> LinearSizingSummary {
    let mut fixed_main = 0.0;
    let mut fill_weight = 0.0;
    let mut margin_total = 0.0;
    for state in states {
        match state.slot_child.slot.size_main {
            SizeModeMain::Fill(weight) => {
                fill_weight += weight.max(0.0);
            }
            _ => {
                fixed_main += state.main;
            }
        }
        margin_total += if horizontal {
            state.slot_child.slot.margin.left + state.slot_child.slot.margin.right
        } else {
            state.slot_child.slot.margin.top + state.slot_child.slot.margin.bottom
        };
    }
    LinearSizingSummary {
        fixed_main,
        fill_weight,
        margin_total,
    }
}

pub(in crate::gui::layout_core::engine) fn resolved_main_sizes_into(
    states: &[LinearLayoutState<'_>],
    sizes: &mut Vec<f32>,
) -> f32 {
    sizes.clear();
    if states.len() > sizes.capacity() {
        sizes.reserve(states.len());
    }
    let mut total_main = 0.0;
    for state in states {
        let size = if state.fill > 0.0 {
            state.fill
        } else {
            state.main
        };
        total_main += size;
        sizes.push(size);
    }
    total_main
}

pub(in crate::gui::layout_core::engine) fn allocate_fill_sizes(
    horizontal: bool,
    remaining: f32,
    total_weight: f32,
    states: &mut [LinearLayoutState<'_>],
    unresolved: &mut Vec<usize>,
) {
    unresolved.clear();
    if states.len() > unresolved.capacity() {
        unresolved.reserve(states.len());
    }
    for (index, state) in states.iter().enumerate() {
        if matches!(state.slot_child.slot.size_main, SizeModeMain::Fill(weight) if weight > 0.0) {
            unresolved.push(index);
        }
    }

    let mut remaining_space = remaining;
    let mut remaining_weight = total_weight;
    while !unresolved.is_empty() {
        let mut changed = false;
        let mut retained = 0;
        for read in 0..unresolved.len() {
            let index = unresolved[read];
            let state = &mut states[index];
            let SizeModeMain::Fill(weight) = state.slot_child.slot.size_main else {
                continue;
            };
            if remaining_weight <= 0.0 {
                state.fill = 0.0;
                continue;
            }
            let proposed = remaining_space * (weight / remaining_weight);
            let clamped = clamp_main(horizontal, proposed, state.slot_child.slot.constraints);
            state.fill = clamped;
            if (clamped - proposed).abs() > f32::EPSILON {
                changed = true;
                remaining_space = (remaining_space - clamped).max(0.0);
                remaining_weight = (remaining_weight - weight).max(0.0);
            } else {
                unresolved[retained] = index;
                retained += 1;
            }
        }
        unresolved.truncate(retained);
        if !changed {
            for &index in unresolved.iter() {
                let state = &mut states[index];
                let SizeModeMain::Fill(weight) = state.slot_child.slot.size_main else {
                    continue;
                };
                if remaining_weight <= 0.0 {
                    state.fill = 0.0;
                } else {
                    state.fill = remaining_space * (weight / remaining_weight);
                }
            }
            break;
        }
    }
}

pub(in crate::gui::layout_core::engine) fn compress_if_needed(
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

pub(in crate::gui::layout_core::engine) fn main_margin_total(
    horizontal: bool,
    states: &[LinearLayoutState<'_>],
) -> f32 {
    states
        .iter()
        .map(|state| {
            if horizontal {
                state.slot_child.slot.margin.left + state.slot_child.slot.margin.right
            } else {
                state.slot_child.slot.margin.top + state.slot_child.slot.margin.bottom
            }
        })
        .sum::<f32>()
}

pub(in crate::gui::layout_core::engine) fn align_main_offsets(
    align: MainAlign,
    available_main: f32,
    total_main: f32,
    base_spacing: f32,
    count: usize,
) -> (f32, f32) {
    if count <= 1 {
        let leading = match align {
            MainAlign::Center => (available_main - total_main).max(0.0) * 0.5,
            MainAlign::End => (available_main - total_main).max(0.0),
            _ => 0.0,
        };
        return (leading, 0.0);
    }
    let free = (available_main - total_main).max(0.0);
    match align {
        MainAlign::Start => (0.0, base_spacing),
        MainAlign::Center => (free * 0.5, base_spacing),
        MainAlign::End => (free, base_spacing),
        MainAlign::SpaceBetween => (0.0, base_spacing + free / (count as f32 - 1.0)),
        MainAlign::SpaceAround => {
            let extra = free / count as f32;
            (extra * 0.5, base_spacing + extra)
        }
        MainAlign::SpaceEvenly => {
            let gap = free / (count as f32 + 1.0);
            (gap, base_spacing + gap)
        }
    }
}

fn clamp_main(horizontal: bool, value: f32, constraints: Constraints) -> f32 {
    if horizontal {
        constraints.clamp_w(value.max(0.0))
    } else {
        constraints.clamp_h(value.max(0.0))
    }
}
