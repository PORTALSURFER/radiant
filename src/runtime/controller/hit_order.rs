use super::*;

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
mod tests {
    use super::*;

    #[test]
    fn sparse_visible_hit_order_preserves_traversal_order() {
        let mut layout = LayoutOutput::default();
        for node_id in [100, 50, 2] {
            layout.rects.insert(
                node_id,
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(10.0, 10.0)),
            );
        }
        let order = vec![100, 200, 201, 202, 203, 204, 205, 206, 207, 50, 208, 209, 2];
        let rank = hit_rank(&order);

        assert_eq!(visible_hit_order(&layout, &order, &rank), vec![100, 50, 2]);
    }

    #[test]
    fn dense_visible_hit_order_reuses_output_buffer() {
        let mut layout = LayoutOutput::default();
        for node_id in [100, 50, 2] {
            layout.rects.insert(
                node_id,
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(10.0, 10.0)),
            );
        }
        let order = vec![100, 200, 201, 202, 203, 204, 205, 206, 207, 50, 208, 209, 2];
        let rank = hit_rank(&order);
        let mut visible = Vec::with_capacity(8);
        visible.push(999);
        let capacity = visible.capacity();

        collect_visible_hit_order(&layout, &order, &rank, &mut visible);

        assert_eq!(visible, vec![100, 50, 2]);
        assert_eq!(visible.capacity(), capacity);
    }

    #[test]
    fn visible_hit_order_presizes_empty_output_buffer() {
        let mut layout = LayoutOutput::default();
        for node_id in [100, 50, 2] {
            layout.rects.insert(
                node_id,
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(10.0, 10.0)),
            );
        }
        let order = vec![100, 200, 201, 202, 203, 204, 205, 206, 207, 50, 208, 209, 2];
        let rank = hit_rank(&order);
        let mut visible = Vec::new();

        collect_visible_hit_order(&layout, &order, &rank, &mut visible);

        assert_eq!(visible, vec![100, 50, 2]);
        assert!(visible.capacity() >= 3);
    }

    #[test]
    fn visible_hit_order_grows_reused_output_buffer_to_visible_capacity() {
        let mut layout = LayoutOutput::default();
        for node_id in 0..64 {
            layout.rects.insert(
                node_id,
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(10.0, 10.0)),
            );
        }
        let order = (0..128).collect::<Vec<_>>();
        let rank = hit_rank(&order);
        let mut visible = Vec::with_capacity(8);

        collect_visible_hit_order(&layout, &order, &rank, &mut visible);

        assert_eq!(visible.len(), 64);
        assert!(visible.capacity() >= 64);
    }

    #[test]
    fn hit_rank_reuses_output_map() {
        let mut rank = HashMap::with_capacity(8);
        rank.insert(999, 999);
        let capacity = rank.capacity();

        collect_hit_rank(&[5, 1, 9], &mut rank);

        assert_eq!(rank.get(&5), Some(&0));
        assert_eq!(rank.get(&1), Some(&1));
        assert_eq!(rank.get(&9), Some(&2));
        assert!(!rank.contains_key(&999));
        assert!(rank.capacity() >= capacity);
    }

    #[test]
    fn hit_rank_presizes_reused_map_for_growth() {
        let mut rank = HashMap::with_capacity(4);
        let order = (0..96).collect::<Vec<_>>();

        collect_hit_rank(&order, &mut rank);

        assert_eq!(rank.len(), 96);
        assert!(rank.capacity() >= 96);
        assert_eq!(rank.get(&95), Some(&95));
    }

    #[test]
    fn hit_order_index_replaces_order_and_clears_visible_cache() {
        let mut layout = LayoutOutput::default();
        layout.rects.insert(
            2,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(10.0, 10.0)),
        );
        let mut index = HitOrderIndex::default();
        index.set_order(vec![1, 2, 3]);
        index.refresh_visible(&layout);

        assert_eq!(index.visible(), &[2]);
        assert!(index.contains(3));

        index.set_order(vec![4, 5]);

        assert!(index.visible().is_empty());
        assert!(!index.contains(3));
        assert!(index.contains(4));
        assert_eq!(index.order(), &[4, 5]);
    }
}
