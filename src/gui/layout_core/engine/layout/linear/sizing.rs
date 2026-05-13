//! Child measurement and size resolution for row and column layout.

use super::super::super::LayoutContext;
use super::super::super::helpers::LinearLayoutState;
use super::super::super::measure::measure_node;
use crate::gui::layout_core::model::{SizeModeCross, SizeModeMain, SlotParams};
use crate::gui::layout_core::tree::{ContainerNode, NodeId, SlotChild};
use crate::gui::types::Vector2;

#[allow(clippy::too_many_arguments)]
pub(super) fn collect_layout_states<'a>(
    container: &'a ContainerNode,
    context: &mut LayoutContext,
    horizontal: bool,
    available_main: f32,
    start_index: usize,
    end_index_exclusive: usize,
) -> Vec<LinearLayoutState<'a>> {
    let clamped_start = start_index.min(container.children.len());
    let clamped_end = end_index_exclusive.min(container.children.len());
    let selected = &container.children[clamped_start..clamped_end];
    let mut states = Vec::with_capacity(selected.len());
    for child in selected {
        let measured = measure_node(&child.child, child.slot.constraints, context);
        let main = resolve_nonfill_main(
            horizontal,
            child,
            measured,
            available_main,
            context,
            child.child.id(),
        );
        states.push(LinearLayoutState::new(child, measured, main));
    }
    states
}

pub(in crate::gui::layout_core::engine::layout) fn resolve_nonfill_main(
    horizontal: bool,
    slot_child: &SlotChild,
    measured: Vector2,
    available_main: f32,
    context: &mut LayoutContext,
    node_id: NodeId,
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
    context.clamp_main(node_id, horizontal, slot.constraints, raw)
}

pub(in crate::gui::layout_core::engine::layout) fn resolve_cross_layout(
    horizontal: bool,
    mode: SizeModeCross,
    measured: Vector2,
    available_cross: f32,
    slot: SlotParams,
    context: &mut LayoutContext,
    node_id: NodeId,
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
    context.clamp_cross(node_id, horizontal, slot.constraints, raw)
}
