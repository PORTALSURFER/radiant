//! Uniform-size virtual metrics fast paths.

use super::state::direct_widget_intrinsic_size;
use crate::gui::layout_core::engine::LayoutContext;
use crate::gui::layout_core::engine::cache::{LinearVirtualMetrics, UniformVirtualMetrics};
use crate::gui::layout_core::engine::helpers::align_main_offsets;
use crate::gui::layout_core::model::{SizeModeMain, VirtualizationAxis};
use crate::gui::layout_core::tree::{ContainerNode, SlotChild};

pub(super) fn build_uniform_linear_metrics(
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
        if has_main_axis_margin(horizontal, child) {
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

fn has_main_axis_margin(horizontal: bool, child: &SlotChild) -> bool {
    let (before, after) = main_margins_for_slot(horizontal, child);
    before.abs() > f32::EPSILON || after.abs() > f32::EPSILON
}

fn main_margins_for_slot(horizontal: bool, child: &SlotChild) -> (f32, f32) {
    if horizontal {
        (child.slot.margin.left, child.slot.margin.right)
    } else {
        (child.slot.margin.top, child.slot.margin.bottom)
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
