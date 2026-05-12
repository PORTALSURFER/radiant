//! Layout pass implementation for strict slot-based layout trees.

mod boxes;
mod grid;
mod linear;
mod scroll;
mod scroll_cache;
mod scroll_helpers;
mod scroll_linear;
mod wrap;

use super::helpers::content_rect;
use super::{LayoutContext, round_rect};
use crate::gui::layout_core::model::ContainerKind;
use crate::gui::layout_core::tree::LayoutNode;
use crate::gui::types::Rect;

pub(super) fn layout_node(node: &LayoutNode, rect: Rect, context: &mut LayoutContext) {
    context.record_layout_visit();
    let rounded = round_rect(rect);
    context.output.rects.insert(node.id(), rounded);
    context.record_node_bounds(node.id(), rounded);
    let LayoutNode::Container(container) = node else {
        return;
    };
    let policy = &container.policy;
    let content = content_rect(rounded, policy.padding);
    context.record_content_bounds(node.id(), content);
    match policy.kind {
        ContainerKind::Row => linear::layout_linear(container, content, true, context),
        ContainerKind::Column => linear::layout_linear(container, content, false, context),
        ContainerKind::Stack => boxes::layout_stack(container, content, context),
        ContainerKind::PaddingBox => boxes::layout_single_fill(container, content, context),
        ContainerKind::AlignBox => boxes::layout_align_box(container, content, context),
        ContainerKind::AspectBox => boxes::layout_aspect_box(container, content, context),
        ContainerKind::Grid => grid::layout_grid(container, content, context),
        ContainerKind::ScrollView => scroll::layout_scroll_view(container, content, context),
        ContainerKind::Wrap => wrap::layout_wrap(container, content, context),
        ContainerKind::SwitchLayout => boxes::layout_switch(container, content, context),
    }
}
