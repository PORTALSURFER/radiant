use super::LayoutContext;
use crate::gui::layout_core::engine::cache::{
    CachedVirtualMetrics, LinearVirtualMetrics, MeasureCacheKey, ResolvedLinearWindow,
    VirtualizationCacheKey,
};
use crate::gui::layout_core::tree::NodeId;
use crate::gui::types::Vector2;
use std::sync::Arc;

impl<'a> LayoutContext<'a> {
    pub(crate) fn cached_measure(
        &self,
        key: MeasureCacheKey,
        node_id: NodeId,
        is_container: bool,
    ) -> Option<Vector2> {
        if self.measure_dirty.contains(&node_id) {
            return None;
        }
        if is_container && !self.measure_dirty.is_empty() {
            return None;
        }
        self.measured
            .get(&key)
            .copied()
            .or_else(|| self.cache.get(&key).copied())
    }

    pub(crate) fn remember_measure(&mut self, key: MeasureCacheKey, value: Vector2) {
        self.measured.insert(key, value);
        self.cache.insert(key, value);
        if self.records_measured_bounds() {
            self.measured_by_node.insert(key.node_id, value);
        }
    }

    pub(crate) fn cached_virtual_metrics(
        &self,
        key: VirtualizationCacheKey,
    ) -> Option<Arc<LinearVirtualMetrics>> {
        self.virtual_cache
            .get(&key)
            .map(|entry| Arc::clone(&entry.metrics))
    }

    pub(crate) fn remember_virtual_metrics(
        &mut self,
        key: VirtualizationCacheKey,
        metrics: Arc<LinearVirtualMetrics>,
        dependencies: std::collections::HashSet<NodeId>,
    ) {
        self.virtual_cache.insert(
            key,
            CachedVirtualMetrics {
                metrics,
                dependencies,
            },
        );
    }

    pub(crate) fn record_measured_size(&mut self, node_id: NodeId, value: Vector2) {
        if self.records_measured_bounds() {
            self.measured_by_node.insert(node_id, value);
        }
    }

    pub(crate) fn set_linear_window(&mut self, node_id: NodeId, window: ResolvedLinearWindow) {
        self.linear_windows.insert(node_id, window);
    }

    pub(crate) fn clear_linear_window(&mut self, node_id: NodeId) {
        self.linear_windows.remove(&node_id);
    }

    pub(crate) fn linear_window(&self, node_id: NodeId) -> Option<ResolvedLinearWindow> {
        self.linear_windows.get(&node_id).cloned()
    }

    pub(crate) fn scroll_offset(&self, node_id: NodeId) -> Vector2 {
        self.state.scroll_offset(node_id)
    }

    fn records_measured_bounds(&self) -> bool {
        self.debug_options.enabled && self.debug_options.show_measured
    }
}
