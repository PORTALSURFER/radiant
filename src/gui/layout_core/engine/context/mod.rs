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
use std::collections::{HashMap, HashSet};

/// Reusable scratch maps cleared at the start of each layout evaluation.
#[derive(Default)]
pub(super) struct LayoutScratch {
    pub(super) measured: HashMap<MeasureCacheKey, Vector2>,
    pub(super) measured_by_node: HashMap<NodeId, Vector2>,
    pub(super) linear_windows: HashMap<NodeId, ResolvedLinearWindow>,
    pub(super) linear_sizes: Vec<f32>,
    pub(super) linear_unresolved: Vec<usize>,
    pub(super) dirty_path: Vec<NodeId>,
    pub(super) dirty_marked: HashSet<NodeId>,
}

/// Shared mutable scratch state for one layout-engine evaluation.
///
/// This type centralizes the transient caches, diagnostic sinks, and debug
/// recording needed by the engine's measure and layout passes so those passes
/// can share one consistent view of normalization and overflow state.
pub(super) struct LayoutContext<'a> {
    measured: &'a mut HashMap<MeasureCacheKey, Vector2>,
    measured_by_node: &'a mut HashMap<NodeId, Vector2>,
    cache: &'a mut HashMap<MeasureCacheKey, Vector2>,
    virtual_cache: &'a mut HashMap<VirtualizationCacheKey, CachedVirtualMetrics>,
    linear_windows: &'a mut HashMap<NodeId, ResolvedLinearWindow>,
    linear_sizes: &'a mut Vec<f32>,
    linear_unresolved: &'a mut Vec<usize>,
    measure_dirty: &'a HashSet<NodeId>,
    state: &'a LayoutState,
    debug_options: LayoutDebugOptions,
    debug_node_filter: Option<&'a HashSet<NodeId>>,
    pub(super) output: LayoutOutput,
}

impl<'a> LayoutContext<'a> {
    /// Build a fresh layout-engine scratchpad for one evaluation pass.
    pub(super) fn new(
        cache: &'a mut HashMap<MeasureCacheKey, Vector2>,
        virtual_cache: &'a mut HashMap<VirtualizationCacheKey, CachedVirtualMetrics>,
        scratch: &'a mut LayoutScratch,
        measure_dirty: &'a HashSet<NodeId>,
        state: &'a LayoutState,
        debug_options: LayoutDebugOptions,
        debug_node_filter: Option<&'a HashSet<NodeId>>,
    ) -> Self {
        scratch.measured.clear();
        scratch.measured_by_node.clear();
        scratch.linear_windows.clear();
        scratch.linear_sizes.clear();
        scratch.linear_unresolved.clear();
        Self {
            measured: &mut scratch.measured,
            measured_by_node: &mut scratch.measured_by_node,
            cache,
            virtual_cache,
            linear_windows: &mut scratch.linear_windows,
            linear_sizes: &mut scratch.linear_sizes,
            linear_unresolved: &mut scratch.linear_unresolved,
            measure_dirty,
            state,
            debug_options,
            debug_node_filter,
            output: LayoutOutput::default(),
        }
    }
}
