use super::LayoutContext;
use crate::gui::layout_core::MountedContainerStateRead;
use crate::gui::layout_core::engine::cache::{
    CachedVirtualMetrics, LinearVirtualMetrics, MeasureCacheKey, ResolvedLinearWindow,
    VirtualizationCacheKey,
};
use crate::gui::layout_core::tree::NodeId;
use crate::gui::types::Vector2;
use std::sync::Arc;

impl<'a> LayoutContext<'a> {
    pub(crate) const fn direction(&self) -> crate::gui::layout_core::WritingDirection {
        self.direction
    }
    pub(crate) fn cached_measure(
        &mut self,
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
        if let Some(value) = self.measured.get(&key).copied() {
            return Some(value);
        }
        let value = self
            .cache
            .get(&key)
            .copied()
            .or_else(|| self.active_cache.and_then(|cache| cache.get(&key).copied()))?;
        self.measured.insert(key, value);
        Some(value)
    }

    pub(crate) fn remember_measure(&mut self, key: MeasureCacheKey, value: Vector2) {
        self.measured.insert(key, value);
        if let Some(previous) = self.cache.insert(key, value)
            && previous != value
            && let Some(ambiguity) = self.cache_key_ambiguity.as_deref_mut()
        {
            *ambiguity = true;
        }
        if self.records_measured_bounds() {
            self.measured_by_node.insert(key.node_id, value);
        }
    }

    pub(crate) fn cached_virtual_metrics(
        &mut self,
        key: VirtualizationCacheKey,
    ) -> Option<Arc<LinearVirtualMetrics>> {
        let metrics = self
            .virtual_cache
            .get(&key)
            .or_else(|| self.active_virtual_cache.and_then(|cache| cache.get(&key)))
            .map(|entry| Arc::clone(&entry.metrics))?;
        self.virtual_touched.insert(key);
        Some(metrics)
    }

    pub(crate) fn remember_virtual_metrics(
        &mut self,
        key: VirtualizationCacheKey,
        metrics: Arc<LinearVirtualMetrics>,
        dependencies: Vec<NodeId>,
    ) {
        self.virtual_touched.insert(key);
        let entry = CachedVirtualMetrics::new(metrics, dependencies);
        let ambiguous = self
            .virtual_cache
            .get(&key)
            .is_some_and(|previous| previous != &entry);
        self.virtual_cache.insert(key, entry);
        if ambiguous && let Some(ambiguity) = self.cache_key_ambiguity.as_deref_mut() {
            *ambiguity = true;
        }
    }

    pub(crate) fn discard_virtual_metrics(&mut self, key: VirtualizationCacheKey) {
        self.virtual_touched.remove(&key);
        self.virtual_cache.remove(&key);
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

    pub(crate) fn take_linear_sizes(&mut self) -> Vec<f32> {
        std::mem::take(self.linear_sizes)
    }

    pub(crate) fn restore_linear_sizes(&mut self, mut sizes: Vec<f32>) {
        sizes.clear();
        *self.linear_sizes = sizes;
    }

    pub(crate) fn take_linear_unresolved(&mut self) -> Vec<usize> {
        std::mem::take(self.linear_unresolved)
    }

    pub(crate) fn restore_linear_unresolved(&mut self, mut unresolved: Vec<usize>) {
        unresolved.clear();
        *self.linear_unresolved = unresolved;
    }

    pub(crate) fn scroll_offset(&self, node_id: NodeId) -> Vector2 {
        self.state.scroll_offset(node_id)
    }

    pub(crate) fn container_state_read(
        &self,
        container_id: NodeId,
    ) -> Option<MountedContainerStateRead<'_>> {
        self.container_state_source
            .and_then(|source| source.read_container_state(container_id))
    }

    fn records_measured_bounds(&self) -> bool {
        self.debug_options.enabled && self.debug_options.show_measured
    }
}
