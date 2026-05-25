use std::collections::HashMap;

use crate::layout::{LayoutOutput, NodeId};

/// Reusable order/rank/visible buffers for one runtime hit-test family.
#[derive(Default)]
pub(super) struct HitOrderIndex {
    order: Vec<NodeId>,
    rank: HashMap<NodeId, usize>,
    visible: Vec<NodeId>,
}

impl HitOrderIndex {
    pub(super) fn set_order(&mut self, order: Vec<NodeId>) {
        self.order = order;
        collect_hit_rank(&self.order, &mut self.rank);
        self.visible.clear();
    }

    pub(super) fn refresh_visible(&mut self, layout: &LayoutOutput) {
        collect_visible_hit_order(layout, &self.order, &self.rank, &mut self.visible);
    }

    pub(super) fn contains(&self, node_id: NodeId) -> bool {
        self.rank.contains_key(&node_id)
    }

    pub(super) fn order(&self) -> &[NodeId] {
        &self.order
    }

    pub(super) fn rank(&self) -> &HashMap<NodeId, usize> {
        &self.rank
    }

    pub(super) fn visible(&self) -> &[NodeId] {
        &self.visible
    }

    pub(super) fn visible_after(&self, node_id: NodeId) -> &[NodeId] {
        let Some(rank) = self.rank.get(&node_id).copied() else {
            return &[];
        };
        let start = self.visible.partition_point(|visible_id| {
            self.rank
                .get(visible_id)
                .copied()
                .is_none_or(|visible_rank| visible_rank <= rank)
        });
        &self.visible[start..]
    }

    pub(super) fn take_order(&mut self) -> Vec<NodeId> {
        self.rank.clear();
        self.visible.clear();
        std::mem::take(&mut self.order)
    }
}

pub(super) fn collect_hit_rank(order: &[NodeId], out: &mut HashMap<NodeId, usize>) {
    out.clear();
    if order.len() > out.capacity() {
        out.reserve(order.len());
    }
    out.extend(
        order
            .iter()
            .copied()
            .enumerate()
            .map(|(index, node_id)| (node_id, index)),
    );
}

pub(super) fn collect_visible_hit_order(
    layout: &LayoutOutput,
    order: &[NodeId],
    rank: &HashMap<NodeId, usize>,
    out: &mut Vec<NodeId>,
) {
    const SPARSE_LAYOUT_SCAN_FACTOR: usize = 4;
    out.clear();
    let visible_capacity = layout.rects.len().min(order.len());
    if visible_capacity > out.capacity() {
        out.reserve(visible_capacity);
    }
    if order.len() <= layout.rects.len().saturating_mul(SPARSE_LAYOUT_SCAN_FACTOR) {
        out.extend(
            order
                .iter()
                .copied()
                .filter(|node_id| layout.rects.contains_key(node_id)),
        );
        return;
    }

    out.extend(
        layout
            .rects
            .keys()
            .filter(|node_id| rank.contains_key(node_id))
            .copied(),
    );
    out.sort_by_key(|node_id| rank.get(node_id).copied().unwrap_or(usize::MAX));
}

#[cfg(test)]
fn hit_rank(order: &[NodeId]) -> HashMap<NodeId, usize> {
    let mut rank = HashMap::with_capacity(order.len());
    collect_hit_rank(order, &mut rank);
    rank
}

#[cfg(test)]
fn visible_hit_order(
    layout: &LayoutOutput,
    order: &[NodeId],
    rank: &HashMap<NodeId, usize>,
) -> Vec<NodeId> {
    let mut visible = Vec::new();
    collect_visible_hit_order(layout, order, rank, &mut visible);
    visible
}

#[cfg(test)]
#[path = "hit_order/tests.rs"]
mod tests;
