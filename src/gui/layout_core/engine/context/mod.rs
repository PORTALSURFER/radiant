//! Shared mutable context for one measure/layout evaluation.
//!
//! [`LayoutContext`] intentionally remains one cohesive scratchpad for cache
//! reuse, diagnostics, virtualization metadata, and debug recording across the
//! measure/layout/scroll passes. These concerns are tightly coupled during one
//! engine evaluation, so the preferred maintenance approach is to keep the
//! scratch state together and only split out clearly pure helpers around it.

mod accessors;
mod clamp;
mod diagnostics;

use super::cache::{
    CachedVirtualMetrics, MeasureCacheKey, ResolvedLinearWindow, VirtualizationCacheKey,
};
use super::{LayoutDebugOptions, LayoutOutput, LayoutState};
use crate::gui::layout_core::tree::NodeId;
use crate::gui::types::Vector2;
use std::collections::{BTreeSet, HashMap};

/// Shared mutable scratch state for one layout-engine evaluation.
///
/// This type centralizes the transient caches, diagnostic sinks, and debug
/// recording needed by the engine's measure and layout passes so those passes
/// can share one consistent view of normalization and overflow state.
pub(super) struct LayoutContext<'a> {
    measured: HashMap<MeasureCacheKey, Vector2>,
    measured_by_node: HashMap<NodeId, Vector2>,
    cache: &'a mut HashMap<MeasureCacheKey, Vector2>,
    virtual_cache: &'a mut HashMap<VirtualizationCacheKey, CachedVirtualMetrics>,
    linear_windows: HashMap<NodeId, ResolvedLinearWindow>,
    measure_dirty: &'a BTreeSet<NodeId>,
    state: &'a LayoutState,
    debug_options: LayoutDebugOptions,
    debug_node_filter: Option<&'a BTreeSet<NodeId>>,
    pub(super) output: LayoutOutput,
}

impl<'a> LayoutContext<'a> {
    /// Build a fresh layout-engine scratchpad for one evaluation pass.
    pub(super) fn new(
        cache: &'a mut HashMap<MeasureCacheKey, Vector2>,
        virtual_cache: &'a mut HashMap<VirtualizationCacheKey, CachedVirtualMetrics>,
        measure_dirty: &'a BTreeSet<NodeId>,
        state: &'a LayoutState,
        debug_options: LayoutDebugOptions,
        debug_node_filter: Option<&'a BTreeSet<NodeId>>,
    ) -> Self {
        Self {
            measured: HashMap::new(),
            measured_by_node: HashMap::new(),
            cache,
            virtual_cache,
            linear_windows: HashMap::new(),
            measure_dirty,
            state,
            debug_options,
            debug_node_filter,
            output: LayoutOutput::default(),
        }
    }
}
