use std::collections::HashMap;

use crate::layout::{LayoutOutput, NodeId};

/// Reusable order/rank/visible buffers for one runtime hit-test family.
#[derive(Default)]
pub(super) struct HitOrderIndex {
    order: Vec<NodeId>,
    rank: HashMap<NodeId, usize>,
    visible: Vec<NodeId>,
    visible_ranks: Vec<usize>,
    ranked_visible: Vec<(NodeId, usize)>,
}

impl HitOrderIndex {
    pub(super) fn set_order(&mut self, order: Vec<NodeId>) {
        self.order = order;
        collect_hit_rank(&self.order, &mut self.rank);
        self.visible.clear();
        self.visible_ranks.clear();
        self.ranked_visible.clear();
    }

    pub(super) fn refresh_visible(&mut self, layout: &LayoutOutput) {
        collect_visible_hit_order_reusing_rank_scratch(
            layout,
            &self.order,
            &self.rank,
            &mut self.visible,
            &mut self.visible_ranks,
            &mut self.ranked_visible,
        );
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
        let start = self
            .visible_ranks
            .partition_point(|visible_rank| *visible_rank <= rank);
        &self.visible[start..]
    }

    pub(super) fn take_order(&mut self) -> Vec<NodeId> {
        self.rank.clear();
        self.visible.clear();
        self.visible_ranks.clear();
        self.ranked_visible.clear();
        std::mem::take(&mut self.order)
    }
}

pub(super) fn collect_hit_rank(order: &[NodeId], out: &mut HashMap<NodeId, usize>) {
    out.clear();
    let additional = additional_reserve_for_target_capacity(out.len(), out.capacity(), order.len());
    if additional > 0 {
        out.reserve(additional);
    }
    out.extend(
        order
            .iter()
            .copied()
            .enumerate()
            .map(|(index, node_id)| (node_id, index)),
    );
}

#[cfg(test)]
pub(super) fn collect_visible_hit_order(
    layout: &LayoutOutput,
    order: &[NodeId],
    rank: &HashMap<NodeId, usize>,
    out: &mut Vec<NodeId>,
) {
    let mut visible_ranks = Vec::new();
    let mut ranked_visible = Vec::new();
    collect_visible_hit_order_reusing_rank_scratch(
        layout,
        order,
        rank,
        out,
        &mut visible_ranks,
        &mut ranked_visible,
    );
}

fn collect_visible_hit_order_reusing_rank_scratch(
    layout: &LayoutOutput,
    order: &[NodeId],
    rank: &HashMap<NodeId, usize>,
    out: &mut Vec<NodeId>,
    visible_ranks: &mut Vec<usize>,
    ranked_visible: &mut Vec<(NodeId, usize)>,
) {
    const SPARSE_LAYOUT_SCAN_FACTOR: usize = 4;
    out.clear();
    visible_ranks.clear();
    ranked_visible.clear();
    let visible_capacity = layout.rects.len().min(order.len());
    let additional =
        additional_reserve_for_target_capacity(out.len(), out.capacity(), visible_capacity);
    if additional > 0 {
        out.reserve(additional);
    }
    let additional = additional_reserve_for_target_capacity(
        visible_ranks.len(),
        visible_ranks.capacity(),
        visible_capacity,
    );
    if additional > 0 {
        visible_ranks.reserve(additional);
    }
    if order.len() <= layout.rects.len().saturating_mul(SPARSE_LAYOUT_SCAN_FACTOR) {
        for (order_rank, node_id) in order.iter().copied().enumerate() {
            if layout.rects.contains_key(&node_id) {
                out.push(node_id);
                visible_ranks.push(order_rank);
            }
        }
        return;
    }

    let additional = additional_reserve_for_target_capacity(
        ranked_visible.len(),
        ranked_visible.capacity(),
        visible_capacity,
    );
    if additional > 0 {
        ranked_visible.reserve(additional);
    }
    ranked_visible.extend(
        layout
            .rects
            .keys()
            .filter_map(|node_id| rank.get(node_id).copied().map(|rank| (*node_id, rank))),
    );
    ranked_visible.sort_by_key(|(_, rank)| *rank);
    out.extend(ranked_visible.iter().map(|(node_id, _)| *node_id));
    visible_ranks.extend(ranked_visible.iter().map(|(_, rank)| *rank));
}

fn additional_reserve_for_target_capacity(
    current_len: usize,
    current_capacity: usize,
    target_capacity: usize,
) -> usize {
    if target_capacity > current_capacity {
        target_capacity.saturating_sub(current_len)
    } else {
        0
    }
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
