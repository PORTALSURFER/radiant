//! Scroll virtualization cache dependency helpers.

use crate::gui::layout_core::tree::{ContainerNode, LayoutNode, NodeId};

/// Collect all node ids under a virtualized content container.
///
/// Virtual span metrics depend on every descendant because child intrinsic
/// sizes, slot constraints, and nested container policies can all affect the
/// resolved linear window.
pub(super) fn collect_virtual_metric_dependencies(
    container: &ContainerNode,
    out: &mut Vec<NodeId>,
) {
    out.push(container.id);
    for child in &container.children {
        collect_subtree_ids(&child.child, out);
    }
}

fn collect_subtree_ids(node: &LayoutNode, out: &mut Vec<NodeId>) {
    out.push(node.id());
    if let LayoutNode::Container(container) = node {
        for child in &container.children {
            collect_subtree_ids(&child.child, out);
        }
    }
}
