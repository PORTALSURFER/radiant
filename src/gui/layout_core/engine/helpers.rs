//! Shared layout-pass helpers for linear placement and compression.

use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::model::{CrossAlign, Insets, MainAlign, SizeModeMain, SlotParams};
use crate::gui::layout_core::tree::SlotChild;
use crate::gui::types::{Point, Rect, Vector2};

pub(super) fn allocate_fill_sizes(
    horizontal: bool,
    remaining: f32,
    total_weight: f32,
    states: &mut [(&SlotChild, Vector2, f32, f32)],
) {
    let mut unresolved: Vec<usize> = states
        .iter()
        .enumerate()
        .filter_map(
            |(index, (slot_child, _, _, _))| match slot_child.slot.size_main {
                SizeModeMain::Fill(weight) if weight > 0.0 => Some((index, weight)),
                _ => None,
            },
        )
        .map(|(index, _)| index)
        .collect();

    let mut remaining_space = remaining;
    let mut remaining_weight = total_weight;
    while !unresolved.is_empty() {
        let mut changed = false;
        let mut next_unresolved = Vec::new();
        for index in unresolved {
            let (slot_child, _, _, fill_size) = &mut states[index];
            let SizeModeMain::Fill(weight) = slot_child.slot.size_main else {
                continue;
            };
            if remaining_weight <= 0.0 {
                *fill_size = 0.0;
                continue;
            }
            let proposed = remaining_space * (weight / remaining_weight);
            let clamped = clamp_main(horizontal, proposed, slot_child.slot.constraints);
            *fill_size = clamped;
            if (clamped - proposed).abs() > f32::EPSILON {
                changed = true;
                remaining_space = (remaining_space - clamped).max(0.0);
                remaining_weight = (remaining_weight - weight).max(0.0);
            } else {
                next_unresolved.push(index);
            }
        }
        if !changed {
            for index in next_unresolved {
                let (slot_child, _, _, fill_size) = &mut states[index];
                let SizeModeMain::Fill(weight) = slot_child.slot.size_main else {
                    continue;
                };
                if remaining_weight <= 0.0 {
                    *fill_size = 0.0;
                } else {
                    *fill_size = remaining_space * (weight / remaining_weight);
                }
            }
            break;
        }
        unresolved = next_unresolved;
    }
}

pub(super) fn compress_if_needed(
    horizontal: bool,
    available_main: f32,
    states: &[(&SlotChild, Vector2, f32, f32)],
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

pub(super) fn scale_sizes_to_fit(
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
    states: &[(&SlotChild, Vector2, f32, f32)],
    overflow: &mut f32,
    sizes: &mut [f32],
) {
    for (index, (slot_child, _, _, _)) in states.iter().enumerate() {
        let mode = slot_child.slot.size_main;
        if !predicate(mode, slot_child.slot.allow_fixed_compress) {
            continue;
        }
        let min_main = if horizontal {
            slot_child.slot.constraints.min_w
        } else {
            slot_child.slot.constraints.min_h
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

pub(super) fn main_margin_total(
    horizontal: bool,
    states: &[(&SlotChild, Vector2, f32, f32)],
) -> f32 {
    states
        .iter()
        .map(|(slot_child, _, _, _)| {
            if horizontal {
                slot_child.slot.margin.left + slot_child.slot.margin.right
            } else {
                slot_child.slot.margin.top + slot_child.slot.margin.bottom
            }
        })
        .sum::<f32>()
}

pub(super) fn align_main_offsets(
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

pub(super) fn place_child_rect(
    content: Rect,
    horizontal: bool,
    cursor_main: f32,
    child_main: f32,
    child_cross: f32,
    slot: SlotParams,
    align: CrossAlign,
) -> Rect {
    if horizontal {
        let x = content.min.x + cursor_main;
        let avail_cross = content.height() - slot.margin.top - slot.margin.bottom;
        let y = match align {
            CrossAlign::Start | CrossAlign::Stretch => content.min.y + slot.margin.top,
            CrossAlign::Center => content.min.y + ((content.height() - child_cross) * 0.5),
            CrossAlign::End => content.max.y - child_cross - slot.margin.bottom,
        };
        let h = if matches!(align, CrossAlign::Stretch) {
            avail_cross.max(0.0)
        } else {
            child_cross
        };
        return Rect::from_min_size(
            Point::new(x, y),
            Vector2::new(child_main.max(0.0), h.max(0.0)),
        );
    }

    let y = content.min.y + cursor_main;
    let avail_cross = content.width() - slot.margin.left - slot.margin.right;
    let x = match align {
        CrossAlign::Start | CrossAlign::Stretch => content.min.x + slot.margin.left,
        CrossAlign::Center => content.min.x + ((content.width() - child_cross) * 0.5),
        CrossAlign::End => content.max.x - child_cross - slot.margin.right,
    };
    let w = if matches!(align, CrossAlign::Stretch) {
        avail_cross.max(0.0)
    } else {
        child_cross
    };
    Rect::from_min_size(
        Point::new(x, y),
        Vector2::new(w.max(0.0), child_main.max(0.0)),
    )
}

pub(super) fn content_rect(rect: Rect, padding: Insets) -> Rect {
    let min_x = rect.min.x + padding.left;
    let max_x = (rect.max.x - padding.right).max(min_x);
    let min_y = rect.min.y + padding.top;
    let max_y = (rect.max.y - padding.bottom).max(min_y);
    Rect::from_min_max(Point::new(min_x, min_y), Point::new(max_x, max_y))
}

pub(super) fn clamp_main(horizontal: bool, value: f32, constraints: Constraints) -> f32 {
    if horizontal {
        constraints.clamp_w(value.max(0.0))
    } else {
        constraints.clamp_h(value.max(0.0))
    }
}

pub(super) fn clamp_cross(horizontal: bool, value: f32, constraints: Constraints) -> f32 {
    if horizontal {
        constraints.clamp_h(value.max(0.0))
    } else {
        constraints.clamp_w(value.max(0.0))
    }
}
