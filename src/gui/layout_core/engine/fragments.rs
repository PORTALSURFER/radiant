//! Bounded exact-input geometry reuse for plain, state-independent subtrees.

use super::{LayoutDebugOptions, LayoutOutput};
use crate::gui::{
    layout_core::{
        WritingDirection,
        model::{ContainerKind, ContainerPolicy, SlotParams},
        tree::{LayoutNode, NodeId},
    },
    types::Rect,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

const MAX_FRAGMENTS: usize = 64;
const MAX_FRAGMENT_NODES: usize = 1024;
const MAX_RETAINED_NODES: usize = 32_768;
const MAX_ADMISSION_NODES: usize = 65_536;
const MIN_FRAGMENT_NODES: usize = 16;

#[derive(Clone, PartialEq)]
struct PlainPolicy {
    kind: ContainerKind,
    spacing: u32,
    padding: [u32; 4],
    main: crate::gui::layout_core::model::MainAlign,
    cross: crate::gui::layout_core::model::CrossAlign,
    overflow: crate::gui::layout_core::model::OverflowPolicy,
}

impl PlainPolicy {
    fn new(policy: &ContainerPolicy) -> Self {
        Self {
            kind: policy.kind,
            spacing: policy.spacing.to_bits(),
            padding: [
                policy.padding.left.to_bits(),
                policy.padding.right.to_bits(),
                policy.padding.top.to_bits(),
                policy.padding.bottom.to_bits(),
            ],
            main: policy.align_main,
            cross: policy.align_cross,
            overflow: policy.overflow,
        }
    }
}

#[derive(Clone, PartialEq)]
enum NodeKind {
    Widget {
        x: u32,
        y: u32,
        version: u64,
    },
    Container {
        policy: PlainPolicy,
        version: u64,
        derived: [Option<u32>; 4],
    },
}

#[derive(Clone, PartialEq)]
struct NodeInput {
    id: NodeId,
    slot: Option<SlotParams>,
    child_count: usize,
    kind: NodeKind,
}

impl NodeInput {
    fn from_node(node: &LayoutNode, slot: Option<SlotParams>) -> Option<Self> {
        let (child_count, kind) = match node {
            LayoutNode::Widget(widget) => (
                0,
                NodeKind::Widget {
                    x: widget.intrinsic.x.to_bits(),
                    y: widget.intrinsic.y.to_bits(),
                    version: node.state_version(),
                },
            ),
            LayoutNode::Container(container) => {
                if container.layout_policy.is_some()
                    || container.split_pane_runtime.is_some()
                    || container.policy.virtualization.is_some()
                    || !matches!(
                        container.policy.kind,
                        ContainerKind::Row | ContainerKind::Column | ContainerKind::Stack
                    )
                {
                    return None;
                }
                (
                    container.children.len(),
                    NodeKind::Container {
                        policy: PlainPolicy::new(&container.policy),
                        version: node.state_version(),
                        derived: [
                            container.known_main_extent_horizontal,
                            container.known_main_extent_vertical,
                            container.known_uniform_main_horizontal,
                            container.known_uniform_main_vertical,
                        ]
                        .map(|value| value.map(f32::to_bits)),
                    },
                )
            }
        };
        Some(Self {
            id: node.id(),
            slot,
            child_count,
            kind,
        })
    }

    fn matches(&self, node: &LayoutNode, slot: Option<SlotParams>) -> bool {
        if self.id != node.id() || self.slot != slot {
            return false;
        }
        match (&self.kind, node) {
            (NodeKind::Widget { x, y, version }, LayoutNode::Widget(widget)) => {
                *x == widget.intrinsic.x.to_bits()
                    && *y == widget.intrinsic.y.to_bits()
                    && *version == node.state_version()
            }
            (
                NodeKind::Container {
                    policy,
                    version,
                    derived,
                },
                LayoutNode::Container(container),
            ) => {
                container.layout_policy.is_none()
                    && container.split_pane_runtime.is_none()
                    && container.policy.virtualization.is_none()
                    && self.child_count == container.children.len()
                    && *policy == PlainPolicy::new(&container.policy)
                    && *version == node.state_version()
                    && *derived
                        == [
                            container.known_main_extent_horizontal,
                            container.known_main_extent_vertical,
                            container.known_uniform_main_horizontal,
                            container.known_uniform_main_vertical,
                        ]
                        .map(|value| value.map(f32::to_bits))
            }
            _ => false,
        }
    }
}

struct Fragment {
    bounds: [u32; 4],
    direction: WritingDirection,
    nodes: Vec<(NodeInput, Rect)>,
}

fn bounds(rect: Rect) -> [u32; 4] {
    [
        rect.min.x.to_bits(),
        rect.min.y.to_bits(),
        rect.max.x.to_bits(),
        rect.max.y.to_bits(),
    ]
}

impl Fragment {
    fn matches(&self, root: &LayoutNode, rect: Rect, direction: WritingDirection) -> bool {
        if self.bounds != bounds(rect) || self.direction != direction {
            return false;
        }
        let LayoutNode::Container(container) = root else {
            return false;
        };
        self.nodes.len() == container.children.len() + 1
            && self.nodes[0].0.matches(root, None)
            && self.nodes[1..]
                .iter()
                .zip(&container.children)
                .all(|((input, _), child)| input.matches(&child.child, Some(child.slot)))
    }

    fn capture(
        root: &LayoutNode,
        rect: Rect,
        direction: WritingDirection,
        output: &LayoutOutput,
    ) -> Option<Self> {
        let LayoutNode::Container(container) = root else {
            return None;
        };
        // The first slice retains flat plain containers. Large ancestors and
        // nested/custom layout graphs keep the ordinary layout path.
        if container.children.len() + 1 > MAX_FRAGMENT_NODES
            || !container
                .children
                .iter()
                .all(|child| matches!(child.child, LayoutNode::Widget(_)))
        {
            return None;
        }
        let mut pending = vec![(root, None)];
        let mut nodes = Vec::new();
        while let Some((node, slot)) = pending.pop() {
            if nodes.len() == MAX_FRAGMENT_NODES {
                return None;
            }
            let id = node.id();
            if output.is_omitted(id)
                || output.overflowed.contains(&id)
                || output.viewport_bounds.contains_key(&id)
                || output.virtual_windows.contains_key(&id)
                || output.scrollbar_placements.contains_key(&id)
                || output
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.node_id == id)
            {
                return None;
            }
            nodes.push((NodeInput::from_node(node, slot)?, *output.rects.get(&id)?));
            if let LayoutNode::Container(container) = node {
                if nodes.len() + pending.len() + container.children.len() > MAX_FRAGMENT_NODES {
                    return None;
                }
                pending.extend(
                    container
                        .children
                        .iter()
                        .rev()
                        .map(|child| (&child.child, Some(child.slot))),
                );
            }
        }
        (nodes.len() >= MIN_FRAGMENT_NODES).then(|| Self {
            bounds: bounds(rect),
            direction,
            nodes,
        })
    }
}

#[derive(Default)]
pub(super) struct LayoutFragmentCache {
    pub(super) enabled: bool,
    admitted: bool,
    entries: HashMap<NodeId, Arc<Fragment>>,
    touched: HashSet<NodeId>,
    retained_nodes: usize,
    admission: HashSet<NodeId>,
}

impl LayoutFragmentCache {
    pub(super) fn fork_from(&mut self, active: &Self) {
        self.enabled = active.enabled;
        self.admitted = false;
        self.entries.clone_from(&active.entries);
        self.retained_nodes = active.retained_nodes;
        self.touched.clear();
        self.admission.clear();
    }

    pub(super) fn begin(&mut self, root: &LayoutNode, debug: LayoutDebugOptions, dirty: bool) {
        self.touched.clear();
        self.admitted = self.enabled
            && debug == LayoutDebugOptions::default()
            && !dirty
            && unique_ids(root, &mut self.admission);
    }

    pub(super) fn reuse(
        &mut self,
        root: &LayoutNode,
        rect: Rect,
        direction: WritingDirection,
        output: &mut LayoutOutput,
    ) -> bool {
        if !self.admitted {
            return false;
        }
        let Some(fragment) = self.entries.get(&root.id()) else {
            return false;
        };
        if !fragment.matches(root, rect, direction) {
            return false;
        }
        self.touched.insert(root.id());
        for (node, rect) in &fragment.nodes {
            output.rects.insert(node.id, *rect);
        }
        output.stats.materialized_nodes += fragment.nodes.len();
        true
    }

    pub(super) fn capture(
        &mut self,
        root: &LayoutNode,
        rect: Rect,
        direction: WritingDirection,
        output: &LayoutOutput,
    ) {
        if !self.admitted {
            return;
        }
        if let Some(old) = self.entries.remove(&root.id()) {
            self.retained_nodes -= old.nodes.len();
        }
        if self.entries.len() == MAX_FRAGMENTS {
            return;
        }
        let Some(fragment) = Fragment::capture(root, rect, direction, output) else {
            return;
        };
        if fragment.nodes.len() > MAX_RETAINED_NODES - self.retained_nodes {
            return;
        }
        self.retained_nodes += fragment.nodes.len();
        self.touched.insert(root.id());
        self.entries.insert(root.id(), Arc::new(fragment));
    }

    pub(super) fn finish(&mut self) {
        self.entries.retain(|key, _| self.touched.contains(key));
        self.retained_nodes = self
            .entries
            .values()
            .map(|fragment| fragment.nodes.len())
            .sum();
    }
}

fn unique_ids(root: &LayoutNode, seen: &mut HashSet<NodeId>) -> bool {
    seen.clear();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if seen.len() == MAX_ADMISSION_NODES || !seen.insert(node.id()) {
            return false;
        }
        if let LayoutNode::Container(container) = node {
            if seen.len() + pending.len() + container.children.len() > MAX_ADMISSION_NODES {
                return false;
            }
            pending.extend(container.children.iter().map(|child| &child.child));
        }
    }
    true
}

#[cfg(test)]
mod tests;
