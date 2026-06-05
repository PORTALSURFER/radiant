//! Cache keys and virtualized linear metrics for the layout engine.

use super::super::constraints::Constraints;
use super::super::model::VirtualizationAxis;
use super::super::tree::{ContainerNode, LayoutNode, NodeId};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::gui::layout_core::engine) struct ConstraintKey {
    min_w: u32,
    max_w: u32,
    min_h: u32,
    max_h: u32,
}

impl ConstraintKey {
    fn from_constraints(constraints: Constraints) -> Self {
        Self {
            min_w: constraints.min_w.to_bits(),
            max_w: constraints.max_w.to_bits(),
            min_h: constraints.min_h.to_bits(),
            max_h: constraints.max_h.to_bits(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::gui::layout_core::engine) struct MeasureCacheKey {
    pub(super) node_id: NodeId,
    constraints: ConstraintKey,
    state_version: u64,
}

impl MeasureCacheKey {
    pub(super) fn new(node: &LayoutNode, constraints: Constraints) -> Self {
        Self {
            node_id: node.id(),
            constraints: ConstraintKey::from_constraints(constraints),
            state_version: node.state_version(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::gui::layout_core::engine) struct VirtualizationCacheKey {
    node_id: NodeId,
    constraints: ConstraintKey,
    axis: VirtualizationAxis,
    child_count: usize,
    policy_fingerprint: u64,
}

impl VirtualizationCacheKey {
    pub(super) fn new(
        node_id: NodeId,
        constraints: Constraints,
        axis: VirtualizationAxis,
        child_count: usize,
        policy_fingerprint: u64,
    ) -> Self {
        Self {
            node_id,
            constraints: ConstraintKey::from_constraints(constraints),
            axis,
            child_count,
            policy_fingerprint,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::gui::layout_core::engine) struct VirtualSpan {
    pub(super) start: f32,
    pub(super) end: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui::layout_core::engine) struct UniformVirtualMetrics {
    pub(super) count: usize,
    pub(super) main_size: f32,
    pub(super) step: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(in crate::gui::layout_core::engine) struct LinearVirtualMetrics {
    pub(super) spans: Vec<VirtualSpan>,
    pub(super) main_sizes: Vec<f32>,
    pub(super) uniform: Option<UniformVirtualMetrics>,
    pub(super) total_main: f32,
    pub(super) leading_offset: f32,
    pub(super) distributed_spacing: f32,
}

impl LinearVirtualMetrics {
    pub(super) fn len(&self) -> usize {
        self.uniform
            .map(|uniform| uniform.count)
            .unwrap_or(self.spans.len())
    }

    pub(super) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(super) fn span(&self, index: usize) -> Option<VirtualSpan> {
        if let Some(uniform) = self.uniform {
            if index >= uniform.count {
                return None;
            }
            let start = self.leading_offset + uniform.step * index as f32;
            return Some(VirtualSpan {
                start,
                end: start + uniform.main_size,
            });
        }
        self.spans.get(index).copied()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui::layout_core::engine) struct ResolvedLinearWindow {
    pub(super) first: usize,
    pub(super) last_exclusive: usize,
    pub(super) cursor_main_start: f32,
    pub(super) metrics: Arc<LinearVirtualMetrics>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui::layout_core::engine) struct CachedVirtualMetrics {
    pub(super) metrics: Arc<LinearVirtualMetrics>,
    pub(super) dependencies: Vec<NodeId>,
}

pub(in crate::gui::layout_core::engine) fn virtualization_policy_fingerprint(
    container: &ContainerNode,
) -> u64 {
    container.state_version
}

pub(in crate::gui::layout_core::engine) fn invalidate_virtual_cache_for(
    virtual_cache: &mut HashMap<VirtualizationCacheKey, CachedVirtualMetrics>,
    node_id: NodeId,
) {
    virtual_cache.retain(|_, entry| !entry.depends_on(node_id));
}

pub(in crate::gui::layout_core::engine) fn invalidate_virtual_cache_for_any(
    virtual_cache: &mut HashMap<VirtualizationCacheKey, CachedVirtualMetrics>,
    node_ids: &HashSet<NodeId>,
) {
    if node_ids.is_empty() {
        return;
    }
    virtual_cache.retain(|_, entry| !entry.depends_on_any(node_ids));
}

impl CachedVirtualMetrics {
    pub(super) fn new(metrics: Arc<LinearVirtualMetrics>, mut dependencies: Vec<NodeId>) -> Self {
        dependencies.sort_unstable();
        dependencies.dedup();
        Self {
            metrics,
            dependencies,
        }
    }

    fn depends_on(&self, node_id: NodeId) -> bool {
        self.dependencies.binary_search(&node_id).is_ok()
    }

    fn depends_on_any(&self, node_ids: &HashSet<NodeId>) -> bool {
        if self.dependencies.len() <= node_ids.len() {
            return self.dependencies.iter().any(|id| node_ids.contains(id));
        }
        node_ids.iter().any(|id| self.depends_on(*id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::layout_core::constraints::Constraints;

    #[test]
    fn virtual_cache_invalidation_checks_entry_dependencies_against_dirty_set() {
        let mut cache = HashMap::new();
        let key = VirtualizationCacheKey::new(
            1,
            Constraints::unconstrained(),
            VirtualizationAxis::Vertical,
            3,
            0,
        );
        cache.insert(
            key,
            CachedVirtualMetrics::new(Arc::new(LinearVirtualMetrics::default()), vec![10, 20, 30]),
        );

        let unrelated = HashSet::from([40, 50, 60]);
        invalidate_virtual_cache_for_any(&mut cache, &unrelated);
        assert!(cache.contains_key(&key));

        let dirty = HashSet::from([20, 40, 60]);
        invalidate_virtual_cache_for_any(&mut cache, &dirty);
        assert!(!cache.contains_key(&key));
    }

    #[test]
    fn cached_virtual_metrics_canonicalize_dependencies_for_fast_lookup() {
        let cached = CachedVirtualMetrics::new(
            Arc::new(LinearVirtualMetrics::default()),
            vec![30, 10, 20, 10, 30],
        );

        assert_eq!(cached.dependencies, vec![10, 20, 30]);
        assert!(cached.depends_on(20));
        assert!(!cached.depends_on(40));
        assert!(cached.depends_on_any(&HashSet::from([5, 20])));
        assert!(!cached.depends_on_any(&HashSet::from([5, 40])));
    }
}
