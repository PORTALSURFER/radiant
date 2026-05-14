//! Child placement for row and column layout.

use super::super::super::LayoutContext;
use super::super::super::helpers::{LinearLayoutState, place_child_rect, resolved_main_size};
use super::super::layout_node;
use super::sizing::resolve_cross_layout;
use crate::gui::layout_core::tree::ContainerNode;
use crate::gui::types::Rect;

pub(super) enum LinearChildSizes<'a> {
    Slice(&'a [f32]),
    Uniform { main_size: f32, len: usize },
    Resolved,
}

impl LinearChildSizes<'_> {
    fn matches_len(&self, states_len: usize) -> bool {
        match self {
            Self::Slice(sizes) => sizes.len() == states_len,
            Self::Uniform { len, .. } => *len == states_len,
            Self::Resolved => true,
        }
    }

    fn get(&self, index: usize, state: &LinearLayoutState<'_>) -> Option<f32> {
        match self {
            Self::Slice(sizes) => sizes.get(index).copied(),
            Self::Uniform { main_size, len } => (index < *len).then_some(*main_size),
            Self::Resolved => Some(resolved_main_size(state)),
        }
    }
}

pub(super) struct LinearPlacement<'a, 'state> {
    pub(super) container: &'a ContainerNode,
    pub(super) content: Rect,
    pub(super) horizontal: bool,
    pub(super) available_cross: f32,
    pub(super) states: &'a [LinearLayoutState<'state>],
    pub(super) sizes: LinearChildSizes<'a>,
    pub(super) leading: f32,
    pub(super) distributed_spacing: f32,
}

pub(super) fn place_linear_children(
    placement: LinearPlacement<'_, '_>,
    context: &mut LayoutContext,
) {
    if !placement.sizes.matches_len(placement.states.len()) {
        return;
    }
    let mut cursor = placement.leading;
    for (index, state) in placement.states.iter().enumerate() {
        let slot_child = state.slot_child;
        let slot = slot_child.slot;
        let main_margin_before = if placement.horizontal {
            slot.margin.left
        } else {
            slot.margin.top
        };
        let main_margin_after = if placement.horizontal {
            slot.margin.right
        } else {
            slot.margin.bottom
        };
        cursor += main_margin_before;
        let Some(child_main) = placement.sizes.get(index, state).map(|size| size.max(0.0)) else {
            return;
        };
        let child_cross = resolve_cross_layout(
            placement.horizontal,
            slot.size_cross,
            state.measured,
            placement.available_cross,
            slot,
            context,
            slot_child.child.id(),
        );
        let cross_align = slot
            .align_cross_override
            .unwrap_or(placement.container.policy.align_cross);
        let child_rect = place_child_rect(
            placement.content,
            placement.horizontal,
            cursor,
            child_main,
            child_cross,
            slot,
            cross_align,
        );
        context.record_slot_margin(slot_child.child.id(), child_rect, slot.margin);
        layout_node(&slot_child.child, child_rect, context);
        cursor += child_main + main_margin_after + placement.distributed_spacing;
    }
}
