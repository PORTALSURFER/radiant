//! Dirty-subtree traversal helpers for layout and measure invalidation.

use crate::gui::layout_core::tree::{LayoutNode, NodeId};
use std::collections::BTreeSet;

/// Collect ancestor-path ids and all descendants for `target` in `root`.
///
/// Returns `true` when the target exists in the tree.
pub(super) fn collect_path_and_descendants(
    node: &LayoutNode,
    target: NodeId,
    path: &mut Vec<NodeId>,
    out: &mut BTreeSet<NodeId>,
) -> bool {
    path.push(node.id());
    if node.id() == target {
        out.extend(path.iter().copied());
        collect_descendants(node, out);
        path.pop();
        return true;
    }
    if let LayoutNode::Container(container) = node {
        for child in &container.children {
            if collect_path_and_descendants(&child.child, target, path, out) {
                path.pop();
                return true;
            }
        }
    }
    path.pop();
    false
}

fn collect_descendants(node: &LayoutNode, out: &mut BTreeSet<NodeId>) {
    out.insert(node.id());
    if let LayoutNode::Container(container) = node {
        for child in &container.children {
            collect_descendants(&child.child, out);
        }
    }
}
