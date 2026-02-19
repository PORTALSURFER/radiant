//! Layout pass implementation for strict slot-based layout trees.

mod boxes;

use super::helpers::{
    align_main_offsets, allocate_fill_sizes, clamp_cross, clamp_main, compress_if_needed,
    content_rect, main_margin_total, place_child_rect, scale_sizes_to_fit,
};
use super::{LayoutContext, round_rect};
use crate::gui::layout_core::model::{ContainerKind, OverflowPolicy, SizeModeCross, SizeModeMain};
use crate::gui::layout_core::tree::{ContainerNode, LayoutNode, SlotChild};
use crate::gui::types::{Rect, Vector2};

pub(super) fn layout_node(node: &LayoutNode, rect: Rect, context: &mut LayoutContext) {
    context.output.rects.insert(node.id(), round_rect(rect));
    let LayoutNode::Container(container) = node else {
        return;
    };
    let policy = &container.policy;
    let content = content_rect(rect, policy.padding);
    match policy.kind {
        ContainerKind::Row => layout_linear(container, content, true, context),
        ContainerKind::Column => layout_linear(container, content, false, context),
        ContainerKind::Stack => boxes::layout_stack(container, content, context),
        ContainerKind::PaddingBox => boxes::layout_single_fill(container, content, context),
        ContainerKind::AlignBox => boxes::layout_align_box(container, content, context),
        ContainerKind::AspectBox => boxes::layout_aspect_box(container, content, context),
        ContainerKind::Grid => boxes::layout_grid(container, content, context),
        ContainerKind::ScrollView => boxes::layout_scroll_view(container, content, context),
        ContainerKind::Wrap => boxes::layout_wrap(container, content, context),
        ContainerKind::SwitchLayout => boxes::layout_switch(container, content, context),
    }
}

fn layout_linear(
    container: &ContainerNode,
    content: Rect,
    horizontal: bool,
    context: &mut LayoutContext,
) {
    if container.children.is_empty() {
        return;
    }
    let policy = &container.policy;
    let spacing = policy.spacing.max(0.0);
    let available_main = if horizontal {
        content.width()
    } else {
        content.height()
    }
    .max(0.0);
    let available_cross = if horizontal {
        content.height()
    } else {
        content.width()
    }
    .max(0.0);

    let mut states = collect_layout_states(container, context, horizontal, available_main);
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
            container,
            horizontal,
            available_main,
            spacing_total,
            &states,
            &mut sizes,
            context,
        );
        total_main = sizes.iter().sum::<f32>() + margin_total + spacing_total;
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
        &sizes,
        leading,
        distributed_spacing,
        context,
    );

    if total_main > available_main {
        let (x, y) = if horizontal {
            (true, false)
        } else {
            (false, true)
        };
        context.record_overflow(container.id, policy.overflow, x, y);
    }
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
                "linear container overflowed and delegated to scroll policy",
            );
        }
        OverflowPolicy::Wrap => {
            context.push_diagnostic(
                container.id,
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

fn collect_layout_states<'a>(
    container: &'a ContainerNode,
    context: &mut LayoutContext,
    horizontal: bool,
    available_main: f32,
) -> Vec<(&'a SlotChild, Vector2, f32, f32)> {
    let mut states = Vec::with_capacity(container.children.len());
    for child in &container.children {
        let measured = super::measure::measure_node(&child.child, child.slot.constraints, context);
        let main = resolve_nonfill_main(horizontal, child, measured, available_main);
        states.push((child, measured, main, 0.0));
    }
    states
}

#[allow(clippy::too_many_arguments)]
fn place_linear_children(
    container: &ContainerNode,
    content: Rect,
    horizontal: bool,
    available_cross: f32,
    states: &[(&SlotChild, Vector2, f32, f32)],
    sizes: &[f32],
    leading: f32,
    distributed_spacing: f32,
    context: &mut LayoutContext,
) {
    let mut cursor = leading;
    for (index, (slot_child, measured, _, _)) in states.iter().enumerate() {
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
        let child_main = sizes[index].max(0.0);
        let child_cross = resolve_cross_layout(
            horizontal,
            slot.size_cross,
            *measured,
            available_cross,
            slot,
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
        layout_node(&slot_child.child, child_rect, context);
        cursor += child_main + main_margin_after + distributed_spacing;
    }
}

fn resolve_nonfill_main(
    horizontal: bool,
    slot_child: &SlotChild,
    measured: Vector2,
    available_main: f32,
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
    clamp_main(horizontal, raw, slot.constraints)
}

fn resolve_cross_layout(
    horizontal: bool,
    mode: SizeModeCross,
    measured: Vector2,
    available_cross: f32,
    slot: crate::gui::layout_core::model::SlotParams,
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
    clamp_cross(horizontal, raw, slot.constraints)
}
