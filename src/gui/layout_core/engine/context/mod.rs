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
    pub(super) virtual_touched: HashSet<VirtualizationCacheKey>,
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
    virtual_touched: &'a mut HashSet<VirtualizationCacheKey>,
    cache: &'a mut HashMap<MeasureCacheKey, Vector2>,
    virtual_cache: &'a mut HashMap<VirtualizationCacheKey, CachedVirtualMetrics>,
    linear_windows: &'a mut HashMap<NodeId, ResolvedLinearWindow>,
    linear_sizes: &'a mut Vec<f32>,
    linear_unresolved: &'a mut Vec<usize>,
    measure_dirty: &'a HashSet<NodeId>,
    state: &'a LayoutState,
    debug_options: LayoutDebugOptions,
    debug_node_filter: Option<&'a HashSet<NodeId>>,
    pub(super) output: &'a mut LayoutOutput,
}

pub(super) struct LayoutContextParts<'a> {
    pub(super) cache: &'a mut HashMap<MeasureCacheKey, Vector2>,
    pub(super) virtual_cache: &'a mut HashMap<VirtualizationCacheKey, CachedVirtualMetrics>,
    pub(super) scratch: &'a mut LayoutScratch,
    pub(super) output: &'a mut LayoutOutput,
    pub(super) measure_dirty: &'a HashSet<NodeId>,
    pub(super) state: &'a LayoutState,
    pub(super) debug_options: LayoutDebugOptions,
    pub(super) debug_node_filter: Option<&'a HashSet<NodeId>>,
}

impl<'a> LayoutContext<'a> {
    /// Build a fresh layout-engine scratchpad for one evaluation pass.
    pub(super) fn new(parts: LayoutContextParts<'a>) -> Self {
        parts.scratch.measured.clear();
        parts.scratch.measured_by_node.clear();
        parts.scratch.virtual_touched.clear();
        parts.scratch.linear_windows.clear();
        parts.scratch.linear_sizes.clear();
        parts.scratch.linear_unresolved.clear();
        parts.output.clear_reusing_storage();
        Self {
            measured: &mut parts.scratch.measured,
            measured_by_node: &mut parts.scratch.measured_by_node,
            virtual_touched: &mut parts.scratch.virtual_touched,
            cache: parts.cache,
            virtual_cache: parts.virtual_cache,
            linear_windows: &mut parts.scratch.linear_windows,
            linear_sizes: &mut parts.scratch.linear_sizes,
            linear_unresolved: &mut parts.scratch.linear_unresolved,
            measure_dirty: parts.measure_dirty,
            state: parts.state,
            debug_options: parts.debug_options,
            debug_node_filter: parts.debug_node_filter,
            output: parts.output,
        }
    }
}
