//! Linear layout state collection and main-axis sizing helpers.

use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::model::SizeModeMain;
use crate::gui::layout_core::tree::SlotChild;
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
        let size = resolved_main_size(state);
        total_main += size;
        sizes.push(size);
    }
    total_main
}

pub(in crate::gui::layout_core::engine) fn resolved_main_total(
    states: &[LinearLayoutState<'_>],
) -> f32 {
    states.iter().map(resolved_main_size).sum()
}

pub(in crate::gui::layout_core::engine) fn resolved_main_size(
    state: &LinearLayoutState<'_>,
) -> f32 {
    if state.fill > 0.0 {
        state.fill
    } else {
        state.main
    }
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

pub(super) fn main_margin_total(horizontal: bool, states: &[LinearLayoutState<'_>]) -> f32 {
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

fn clamp_main(horizontal: bool, value: f32, constraints: Constraints) -> f32 {
    if horizontal {
        constraints.clamp_w(value.max(0.0))
    } else {
        constraints.clamp_h(value.max(0.0))
    }
}
