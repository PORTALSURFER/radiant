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

#[allow(clippy::too_many_arguments)]
pub(super) fn place_linear_children(
    container: &ContainerNode,
    content: Rect,
    horizontal: bool,
    available_cross: f32,
    states: &[LinearLayoutState<'_>],
    sizes: LinearChildSizes<'_>,
    leading: f32,
    distributed_spacing: f32,
    context: &mut LayoutContext,
) {
    if !sizes.matches_len(states.len()) {
        return;
    }
    let mut cursor = leading;
    for (index, state) in states.iter().enumerate() {
        let slot_child = state.slot_child;
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
        let Some(child_main) = sizes.get(index, state).map(|size| size.max(0.0)) else {
            return;
        };
        let child_cross = resolve_cross_layout(
            horizontal,
            slot.size_cross,
            state.measured,
            available_cross,
            slot,
            context,
            slot_child.child.id(),
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
        context.record_slot_margin(slot_child.child.id(), child_rect, slot.margin);
        layout_node(&slot_child.child, child_rect, context);
        cursor += child_main + main_margin_after + distributed_spacing;
    }
}
