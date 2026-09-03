//! Deterministic two-pass layout engine for strict slot-based trees.

mod cache;
mod context;
mod direct;
mod dirty;
mod helpers;
mod layout;
mod measure;
mod types;

use super::MountedContainerStateRead;
use super::constraints::Constraints;
use super::tree::{LayoutNode, NodeId};
use crate::gui::types::{Rect, Vector2};
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use cache::{
    CachedVirtualMetrics, MeasureCacheKey, VirtualizationCacheKey, invalidate_virtual_cache_for,
    invalidate_virtual_cache_for_any,
};
use context::{LayoutContext, LayoutContextParts, LayoutScratch};
pub use types::{
    DebugPrimitiveKind, LayoutDebugOptions, LayoutDebugPrimitive, LayoutDiagnostic,
    LayoutDiagnosticCode, LayoutOutput, LayoutState, LayoutStats, OverflowInfo, VirtualWindowInfo,
};

/// Crate-private immutable source for one complete mounted-container layout
/// evaluation.
pub(crate) trait LayoutContainerStateReadSource {
    fn read_container_state(&self, container_id: NodeId) -> Option<MountedContainerStateRead<'_>>;
}

/// Marker for the owner of root layout-visible values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RootLayoutAuthorityOwner {}

/// Marker for the owner of caller-supplied dynamic layout state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutStateAuthorityOwner {}

/// Marker for the owner of an optional mounted layout-state source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MountedLayoutSourceAuthorityOwner {}

/// Exact caller-owned authority evidence for one layout-visible input owner.
///
/// The authority generation distinguishes recreated owners even when their
/// revisions repeat. Revisions are caller-owned monotonic evidence covering
/// every layout-visible value for that owner; the prepared boundary never
/// derives them from the values themselves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LayoutAuthorityEvidence<Owner> {
    authority_generation: u64,
    revision: u64,
    owner: PhantomData<fn() -> Owner>,
}

impl<Owner> LayoutAuthorityEvidence<Owner> {
    #[allow(dead_code)]
    pub(crate) const fn new(authority_generation: u64, revision: u64) -> Self {
        Self {
            authority_generation,
            revision,
            owner: PhantomData,
        }
    }

    pub(crate) fn is_valid(&self) -> bool {
        self.authority_generation != 0
            && self.authority_generation != u64::MAX
            && self.revision != u64::MAX
    }

    pub(crate) fn advance_revision(&mut self) -> bool {
        if !self.is_valid() {
            return false;
        }
        let Some(next_revision) = self.revision.checked_add(1) else {
            return false;
        };
        if next_revision == u64::MAX {
            return false;
        }
        self.revision = next_revision;
        true
    }

    pub(crate) fn advance_authority_generation(&mut self) -> bool {
        if !self.is_valid() {
            return false;
        }
        let Some(next_generation) = self.authority_generation.checked_add(1) else {
            return false;
        };
        if next_generation == 0 || next_generation == u64::MAX {
            return false;
        }
        self.authority_generation = next_generation;
        true
    }
}

pub(crate) type RootLayoutAuthorityEvidence = LayoutAuthorityEvidence<RootLayoutAuthorityOwner>;
pub(crate) type LayoutStateAuthorityEvidence = LayoutAuthorityEvidence<LayoutStateAuthorityOwner>;
pub(crate) type MountedLayoutSourceAuthorityEvidence =
    LayoutAuthorityEvidence<MountedLayoutSourceAuthorityOwner>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LayoutViewportEvidence {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

impl LayoutViewportEvidence {
    const fn from_rect(rect: Rect) -> Self {
        Self {
            min_x: rect.min.x.to_bits(),
            min_y: rect.min.y.to_bits(),
            max_x: rect.max.x.to_bits(),
            max_y: rect.max.y.to_bits(),
        }
    }
}

/// Exact private-boundary evidence for one prepared layout input set.
///
/// Root and state authorities are required. Mounted authority evidence is
/// present exactly when the preparation source is present. Viewport bits and
/// debug options are captured directly because both affect layout output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LayoutInputEvidence {
    root: Option<RootLayoutAuthorityEvidence>,
    state: Option<LayoutStateAuthorityEvidence>,
    mounted: Option<MountedLayoutSourceAuthorityEvidence>,
    viewport: LayoutViewportEvidence,
    debug: LayoutDebugOptions,
}

impl LayoutInputEvidence {
    #[allow(dead_code)]
    pub(crate) const fn new(
        root: Option<RootLayoutAuthorityEvidence>,
        state: Option<LayoutStateAuthorityEvidence>,
        mounted: Option<MountedLayoutSourceAuthorityEvidence>,
        viewport: Rect,
        debug: LayoutDebugOptions,
    ) -> Self {
        Self {
            root,
            state,
            mounted,
            viewport: LayoutViewportEvidence::from_rect(viewport),
            debug,
        }
    }

    pub(crate) fn is_valid_for_prepare(
        &self,
        viewport: Rect,
        debug: LayoutDebugOptions,
        mounted_source_present: bool,
    ) -> bool {
        self.is_valid_for_presence(mounted_source_present)
            && self.viewport == LayoutViewportEvidence::from_rect(viewport)
            && self.debug == debug
    }

    pub(crate) fn is_valid_for_presence(&self, mounted_source_present: bool) -> bool {
        self.root.is_some_and(|evidence| evidence.is_valid())
            && self.state.is_some_and(|evidence| evidence.is_valid())
            && self.mounted.is_some() == mounted_source_present
            && self.mounted.is_none_or(|evidence| evidence.is_valid())
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct LayoutPreparationWorkspaceCapacities {
    measure_updates: usize,
    virtual_updates: usize,
    measured: usize,
    virtual_touched: usize,
    linear_windows: usize,
    linear_sizes: usize,
    linear_unresolved: usize,
    layout_dirty: usize,
    measure_dirty: usize,
    diagnostics: usize,
    debug_primitives: usize,
}

#[allow(dead_code)]
struct LayoutPreparationWorkspacePool {
    storage: Option<LayoutPreparationWorkspaceStorage>,
    capacities: LayoutPreparationWorkspaceCapacities,
}

#[allow(dead_code)]
impl Default for LayoutPreparationWorkspacePool {
    fn default() -> Self {
        Self {
            storage: Some(LayoutPreparationWorkspaceStorage::default()),
            capacities: LayoutPreparationWorkspaceCapacities::default(),
        }
    }
}

/// Reusable candidate-owned buffers for one prepared layout pass.
///
/// The pool is shared only by the preparation boundary. Direct layout never
/// takes this lock, retains an overlay, or changes its cache access path.
#[derive(Clone, Default)]
#[allow(dead_code)]
pub(crate) struct LayoutPreparationWorkspace {
    pool: Arc<Mutex<LayoutPreparationWorkspacePool>>,
}

#[allow(dead_code)]
impl LayoutPreparationWorkspace {
    fn take_storage(&self) -> Option<LayoutPreparationWorkspaceStorage> {
        let mut pool = self.pool.lock().ok()?;
        pool.storage.take()
    }

    fn observe(&self, storage: &LayoutPreparationWorkspaceStorage) {
        let Ok(mut pool) = self.pool.lock() else {
            return;
        };
        pool.capacities = storage.capacities();
    }

    fn return_storage(&self, storage: LayoutPreparationWorkspaceStorage) {
        let Ok(mut pool) = self.pool.lock() else {
            return;
        };
        pool.capacities = storage.capacities();
        if pool.storage.is_none() {
            pool.storage = Some(storage);
        }
    }

    #[cfg(test)]
    fn capacities(&self) -> LayoutPreparationWorkspaceCapacities {
        self.pool
            .lock()
            .map(|pool| pool.capacities)
            .unwrap_or_default()
    }
}

#[allow(dead_code)]
#[derive(Default)]
struct LayoutPreparationWorkspaceStorage {
    output: LayoutOutput,
    scratch: LayoutScratch,
    measure_updates: HashMap<MeasureCacheKey, Vector2>,
    virtual_updates: HashMap<VirtualizationCacheKey, CachedVirtualMetrics>,
    layout_dirty: HashSet<NodeId>,
    measure_dirty: HashSet<NodeId>,
    cache_key_ambiguity: bool,
}

#[allow(dead_code)]
impl LayoutPreparationWorkspaceStorage {
    fn begin(&mut self, layout_dirty: &HashSet<NodeId>, measure_dirty: &HashSet<NodeId>) {
        self.measure_updates.clear();
        self.virtual_updates.clear();
        self.layout_dirty.clear();
        self.measure_dirty.clear();
        self.layout_dirty.extend(layout_dirty.iter().copied());
        self.measure_dirty.extend(measure_dirty.iter().copied());
        self.cache_key_ambiguity = false;
    }

    fn pruning_is_reproducible(&self) -> bool {
        self.measure_updates
            .keys()
            .all(|key| self.scratch.measured.contains_key(key))
            && self
                .virtual_updates
                .keys()
                .all(|key| self.scratch.virtual_touched.contains(key))
    }

    fn capacities(&self) -> LayoutPreparationWorkspaceCapacities {
        LayoutPreparationWorkspaceCapacities {
            measure_updates: self.measure_updates.capacity(),
            virtual_updates: self.virtual_updates.capacity(),
            measured: self.scratch.measured.capacity(),
            virtual_touched: self.scratch.virtual_touched.capacity(),
            linear_windows: self.scratch.linear_windows.capacity(),
            linear_sizes: self.scratch.linear_sizes.capacity(),
            linear_unresolved: self.scratch.linear_unresolved.capacity(),
            layout_dirty: self.layout_dirty.capacity(),
            measure_dirty: self.measure_dirty.capacity(),
            diagnostics: self.output.diagnostics.capacity(),
            debug_primitives: self.output.debug_primitives.capacity(),
        }
    }

    fn retain_reusable_capacity(&mut self, scratch: &LayoutScratch, output: &LayoutOutput) {
        if self.scratch.measured.capacity() < scratch.measured.capacity() {
            self.scratch
                .measured
                .reserve(scratch.measured.capacity() - self.scratch.measured.capacity());
        }
        if self.scratch.measured_by_node.capacity() < scratch.measured_by_node.capacity() {
            self.scratch.measured_by_node.reserve(
                scratch.measured_by_node.capacity() - self.scratch.measured_by_node.capacity(),
            );
        }
        if self.scratch.virtual_touched.capacity() < scratch.virtual_touched.capacity() {
            self.scratch.virtual_touched.reserve(
                scratch.virtual_touched.capacity() - self.scratch.virtual_touched.capacity(),
            );
        }
        if self.scratch.linear_windows.capacity() < scratch.linear_windows.capacity() {
            self.scratch.linear_windows.reserve(
                scratch.linear_windows.capacity() - self.scratch.linear_windows.capacity(),
            );
        }
        if self.scratch.linear_sizes.capacity() < scratch.linear_sizes.capacity() {
            self.scratch
                .linear_sizes
                .reserve(scratch.linear_sizes.capacity() - self.scratch.linear_sizes.capacity());
        }
        if self.scratch.linear_unresolved.capacity() < scratch.linear_unresolved.capacity() {
            self.scratch.linear_unresolved.reserve(
                scratch.linear_unresolved.capacity() - self.scratch.linear_unresolved.capacity(),
            );
        }
        if self.output.diagnostics.capacity() < output.diagnostics.capacity() {
            self.output
                .diagnostics
                .reserve(output.diagnostics.capacity() - self.output.diagnostics.capacity());
        }
        if self.output.debug_primitives.capacity() < output.debug_primitives.capacity() {
            self.output.debug_primitives.reserve(
                output.debug_primitives.capacity() - self.output.debug_primitives.capacity(),
            );
        }
    }
}

/// A complete layout result and cache delta that can be consumed once.
///
/// The candidate is deliberately non-`Clone`: output, scratch, and newly
/// written cache values have one owner until either commit or drop.
#[allow(dead_code)]
pub(crate) struct PreparedLayoutPass {
    workspace: Option<LayoutPreparationWorkspaceStorage>,
    workspace_pool: LayoutPreparationWorkspace,
    generation: u64,
    checked_generation: u64,
    cache_authority: u64,
    generation_exhausted: bool,
    complete: bool,
    input_evidence: Option<LayoutInputEvidence>,
    invalid_input_evidence: bool,
}

#[allow(dead_code)]
impl PreparedLayoutPass {
    fn incomplete(workspace: &LayoutPreparationWorkspace, engine: &LayoutEngine) -> Self {
        Self {
            workspace: None,
            workspace_pool: workspace.clone(),
            generation: engine.generation,
            checked_generation: engine.checked_generation,
            cache_authority: engine.cache_authority,
            generation_exhausted: engine.generation_exhausted,
            complete: false,
            input_evidence: None,
            invalid_input_evidence: false,
        }
    }

    fn invalid_input(workspace: &LayoutPreparationWorkspace, engine: &LayoutEngine) -> Self {
        Self {
            workspace: None,
            workspace_pool: workspace.clone(),
            generation: engine.generation,
            checked_generation: engine.checked_generation,
            cache_authority: engine.cache_authority,
            generation_exhausted: engine.generation_exhausted,
            complete: false,
            input_evidence: None,
            invalid_input_evidence: true,
        }
    }

    fn release_workspace(&mut self) {
        if let Some(storage) = self.workspace.take() {
            self.workspace_pool.return_storage(storage);
        }
    }

    fn take_workspace(&mut self) -> Option<LayoutPreparationWorkspaceStorage> {
        self.workspace.take()
    }

    /// Return the candidate-owned layout output without transferring ownership.
    pub(crate) fn output(&self) -> Option<&LayoutOutput> {
        self.workspace.as_ref().map(|storage| &storage.output)
    }

    /// Return whether preparation produced a complete, internally reproducible
    /// pass that is safe to expose as a runtime candidate.
    pub(crate) fn is_usable(&self) -> bool {
        self.complete
            && !self.invalid_input_evidence
            && !self.generation_exhausted
            && self.workspace.as_ref().is_some_and(|storage| {
                !storage.cache_key_ambiguity && storage.pruning_is_reproducible()
            })
    }

    /// Return whether the active engine still has the exact authority and
    /// cache/dirty evidence observed when this pass was prepared.
    pub(crate) fn is_current_for_engine(&self, engine: &LayoutEngine) -> bool {
        self.input_evidence
            .is_some_and(|input| self.validate_for_engine(engine, input).is_ok())
    }

    /// Validate every prepared-engine authority without touching active
    /// output, caches, scratch, or dirty sets.
    pub(crate) fn validate_for_engine(
        &self,
        engine: &LayoutEngine,
        current_input_evidence: LayoutInputEvidence,
    ) -> Result<(), PreparedLayoutCommitError> {
        let Some(storage) = self.workspace.as_ref() else {
            return Err(if self.invalid_input_evidence {
                PreparedLayoutCommitError::InvalidInputEvidence
            } else {
                PreparedLayoutCommitError::Incomplete
            });
        };
        let veto = if !self.complete {
            Some(if self.invalid_input_evidence {
                PreparedLayoutCommitError::InvalidInputEvidence
            } else {
                PreparedLayoutCommitError::Incomplete
            })
        } else if !self.input_evidence.is_some_and(|prepared_input_evidence| {
            prepared_input_evidence.is_valid_for_presence(prepared_input_evidence.mounted.is_some())
                && current_input_evidence
                    .is_valid_for_presence(prepared_input_evidence.mounted.is_some())
                && prepared_input_evidence == current_input_evidence
        }) {
            Some(PreparedLayoutCommitError::InvalidInputEvidence)
        } else if self.generation_exhausted || engine.generation_exhausted {
            Some(PreparedLayoutCommitError::GenerationExhausted)
        } else if self.generation != engine.generation {
            Some(PreparedLayoutCommitError::StaleEngineGeneration)
        } else if self.checked_generation != engine.checked_generation {
            Some(PreparedLayoutCommitError::StaleCheckedGeneration)
        } else if self.cache_authority != engine.cache_authority {
            Some(PreparedLayoutCommitError::StaleCacheAuthority)
        } else if storage.layout_dirty != engine.layout_dirty
            || storage.measure_dirty != engine.measure_dirty
        {
            Some(PreparedLayoutCommitError::StaleDirtyEvidence)
        } else if storage.cache_key_ambiguity {
            Some(PreparedLayoutCommitError::CacheKeyAmbiguous)
        } else if !storage.pruning_is_reproducible() {
            Some(PreparedLayoutCommitError::PruningEvidenceUnavailable)
        } else if engine.generation.checked_add(1).is_none()
            || engine.checked_generation.checked_add(1).is_none()
            || engine.cache_authority.checked_add(1).is_none()
        {
            Some(PreparedLayoutCommitError::GenerationExhausted)
        } else {
            None
        };

        if let Some(veto) = veto {
            Err(veto)
        } else {
            Ok(())
        }
    }

    /// Consume this candidate into `engine` and `output` after exact evidence
    /// validation.
    pub(crate) fn commit(
        self,
        engine: &mut LayoutEngine,
        output: &mut LayoutOutput,
        current_input_evidence: LayoutInputEvidence,
    ) -> Result<(), PreparedLayoutCommitError> {
        engine.commit_prepared_layout(self, output, current_input_evidence)
    }

    /// Explicitly discard this candidate without touching active layout state.
    pub(crate) fn discard(self) {}
}

#[allow(dead_code)]
impl Drop for PreparedLayoutPass {
    fn drop(&mut self) {
        self.release_workspace();
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreparedLayoutCommitError {
    Incomplete,
    InvalidInputEvidence,
    StaleEngineGeneration,
    StaleCheckedGeneration,
    StaleDirtyEvidence,
    StaleCacheAuthority,
    CacheKeyAmbiguous,
    PruningEvidenceUnavailable,
    GenerationExhausted,
}

/// Reusable stateful layout engine with measurement and virtualization caches.
#[derive(Default)]
pub struct LayoutEngine {
    measure_cache: HashMap<MeasureCacheKey, Vector2>,
    virtual_cache: HashMap<VirtualizationCacheKey, CachedVirtualMetrics>,
    scratch: LayoutScratch,
    layout_dirty: HashSet<NodeId>,
    measure_dirty: HashSet<NodeId>,
    #[allow(dead_code)]
    preparation_workspace: Option<LayoutPreparationWorkspace>,
    generation: u64,
    #[allow(dead_code)]
    checked_generation: u64,
    cache_authority: u64,
    generation_exhausted: bool,
}

impl LayoutEngine {
    fn note_mutation(&mut self, cache_authority_changed: bool) {
        let Some(next_generation) = self.generation.checked_add(1) else {
            self.generation_exhausted = true;
            return;
        };
        self.generation = next_generation;
        if cache_authority_changed {
            let Some(next_authority) = self.cache_authority.checked_add(1) else {
                self.generation_exhausted = true;
                return;
            };
            self.cache_authority = next_authority;
        }
    }

    /// Return whether an explicit layout or measurement invalidation is pending.
    pub(crate) fn has_explicit_dirty(&self) -> bool {
        !self.layout_dirty.is_empty() || !self.measure_dirty.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn prepared_workspace_capacity_signature(&self) -> Option<[usize; 11]> {
        let capacities = self.preparation_workspace.as_ref()?.capacities();
        Some([
            capacities.measure_updates,
            capacities.virtual_updates,
            capacities.measured,
            capacities.virtual_touched,
            capacities.linear_windows,
            capacities.linear_sizes,
            capacities.linear_unresolved,
            capacities.layout_dirty,
            capacities.measure_dirty,
            capacities.diagnostics,
            capacities.debug_primitives,
        ])
    }

    /// Mark a node as geometry-dirty.
    pub fn mark_layout_dirty(&mut self, node_id: NodeId) {
        self.layout_dirty.insert(node_id);
        invalidate_virtual_cache_for(&mut self.virtual_cache, node_id);
        self.note_mutation(true);
    }

    /// Mark a node as intrinsic-measure dirty.
    pub fn mark_measure_dirty(&mut self, node_id: NodeId) {
        self.measure_dirty.insert(node_id);
        invalidate_virtual_cache_for(&mut self.virtual_cache, node_id);
        self.note_mutation(true);
    }

    /// Mark a node subtree as geometry-dirty, including ancestor path nodes.
    pub fn mark_layout_dirty_subtree(&mut self, root: &LayoutNode, node_id: NodeId) {
        self.mark_subtree_dirty(root, node_id, false);
    }

    /// Mark a node subtree as measure-dirty, including ancestor path nodes.
    pub fn mark_measure_dirty_subtree(&mut self, root: &LayoutNode, node_id: NodeId) {
        self.mark_subtree_dirty(root, node_id, true);
    }

    /// Clear all dirty markers.
    pub fn clear_dirty(&mut self) {
        if self.has_explicit_dirty() {
            self.note_mutation(false);
        }
        self.clear_dirty_without_generation();
    }

    fn clear_dirty_without_generation(&mut self) {
        self.layout_dirty.clear();
        self.measure_dirty.clear();
    }

    fn mark_subtree_dirty(&mut self, root: &LayoutNode, node_id: NodeId, measure: bool) {
        self.scratch.dirty_path.clear();
        self.scratch.dirty_marked.clear();
        if !dirty::collect_path_and_descendants(
            root,
            node_id,
            &mut self.scratch.dirty_path,
            &mut self.scratch.dirty_marked,
        ) {
            self.scratch.dirty_marked.insert(node_id);
        }
        for id in &self.scratch.dirty_marked {
            if measure {
                self.measure_dirty.insert(*id);
            } else {
                self.layout_dirty.insert(*id);
            }
        }
        invalidate_virtual_cache_for_any(&mut self.virtual_cache, &self.scratch.dirty_marked);
        self.scratch.dirty_marked.clear();
        self.note_mutation(true);
    }

    /// Compute layout output for `root` in `root_rect` using default state/options.
    pub fn layout(&mut self, root: &LayoutNode, root_rect: Rect) -> LayoutOutput {
        self.layout_with_state(
            root,
            root_rect,
            &LayoutState::default(),
            LayoutDebugOptions::default(),
        )
    }

    /// Compute layout output with dynamic layout state and debug output controls.
    pub fn layout_with_state(
        &mut self,
        root: &LayoutNode,
        root_rect: Rect,
        state: &LayoutState,
        debug: LayoutDebugOptions,
    ) -> LayoutOutput {
        let mut output = LayoutOutput::default();
        self.layout_with_state_into(root, root_rect, state, debug, &mut output);
        output
    }

    /// Compute layout output into an existing output buffer.
    pub fn layout_with_state_into(
        &mut self,
        root: &LayoutNode,
        root_rect: Rect,
        state: &LayoutState,
        debug: LayoutDebugOptions,
        output: &mut LayoutOutput,
    ) {
        self.layout_with_state_and_source_into(root, root_rect, state, debug, None, output);
    }

    /// Compute layout with one immutable runtime-owned mounted-state source.
    ///
    /// This is crate-private so the public layout entry points remain
    /// source-compatible. The source is borrowed by the shared context for
    /// both bottom-up measurement and top-down placement; it is not part of
    /// cache identity or geometry policy.
    pub(crate) fn layout_with_state_and_source_into(
        &mut self,
        root: &LayoutNode,
        root_rect: Rect,
        state: &LayoutState,
        debug: LayoutDebugOptions,
        container_state_source: Option<&dyn LayoutContainerStateReadSource>,
        output: &mut LayoutOutput,
    ) {
        self.note_mutation(true);
        let constraints = Constraints {
            min_w: 0.0,
            max_w: root_rect.width().max(0.0),
            min_h: 0.0,
            max_h: root_rect.height().max(0.0),
        };

        {
            let debug_node_filter = if debug.enabled && !self.layout_dirty.is_empty() {
                Some(&self.layout_dirty)
            } else {
                None
            };
            let mut context = LayoutContext::new(LayoutContextParts {
                cache: &mut self.measure_cache,
                active_cache: None,
                virtual_cache: &mut self.virtual_cache,
                active_virtual_cache: None,
                scratch: &mut self.scratch,
                output,
                measure_dirty: &self.measure_dirty,
                state,
                debug_options: debug,
                debug_node_filter,
                container_state_source,
                cache_key_ambiguity: None,
            });
            let normalized = context.normalize_constraints(root.id(), constraints);
            measure::measure_node(root, normalized, &mut context);
            layout::layout_node(root, root_rect, &mut context);
        }

        self.prune_stale_measure_cache();
        self.prune_stale_virtual_cache();
        self.clear_dirty_without_generation();
    }

    /// Prepare one complete layout pass without changing active engine state.
    #[allow(dead_code)]
    pub(crate) fn prepare_layout_with_state(
        &mut self,
        root: &LayoutNode,
        root_rect: Rect,
        state: &LayoutState,
        debug: LayoutDebugOptions,
        input_evidence: LayoutInputEvidence,
    ) -> PreparedLayoutPass {
        self.prepare_layout_with_state_and_source(
            root,
            root_rect,
            state,
            debug,
            None,
            input_evidence,
        )
    }

    /// Prepare one complete layout pass using an immutable mounted-state source.
    #[allow(dead_code)]
    pub(crate) fn prepare_layout_with_state_and_source(
        &mut self,
        root: &LayoutNode,
        root_rect: Rect,
        state: &LayoutState,
        debug: LayoutDebugOptions,
        container_state_source: Option<&dyn LayoutContainerStateReadSource>,
        input_evidence: LayoutInputEvidence,
    ) -> PreparedLayoutPass {
        let workspace = self
            .preparation_workspace
            .get_or_insert_with(LayoutPreparationWorkspace::default)
            .clone();
        if !input_evidence.is_valid_for_prepare(root_rect, debug, container_state_source.is_some())
        {
            return PreparedLayoutPass::invalid_input(&workspace, self);
        }
        let Some(mut storage) = workspace.take_storage() else {
            return PreparedLayoutPass::incomplete(&workspace, self);
        };
        if self.generation_exhausted {
            workspace.return_storage(storage);
            return PreparedLayoutPass::incomplete(&workspace, self);
        }

        storage.begin(&self.layout_dirty, &self.measure_dirty);
        let constraints = Constraints {
            min_w: 0.0,
            max_w: root_rect.width().max(0.0),
            min_h: 0.0,
            max_h: root_rect.height().max(0.0),
        };

        {
            let debug_node_filter = if debug.enabled && !self.layout_dirty.is_empty() {
                Some(&self.layout_dirty)
            } else {
                None
            };
            let mut context = LayoutContext::new(LayoutContextParts {
                cache: &mut storage.measure_updates,
                active_cache: Some(&self.measure_cache),
                virtual_cache: &mut storage.virtual_updates,
                active_virtual_cache: Some(&self.virtual_cache),
                scratch: &mut storage.scratch,
                output: &mut storage.output,
                measure_dirty: &self.measure_dirty,
                state,
                debug_options: debug,
                debug_node_filter,
                container_state_source,
                cache_key_ambiguity: Some(&mut storage.cache_key_ambiguity),
            });
            let normalized = context.normalize_constraints(root.id(), constraints);
            measure::measure_node(root, normalized, &mut context);
            layout::layout_node(root, root_rect, &mut context);
        }

        workspace.observe(&storage);
        PreparedLayoutPass {
            workspace: Some(storage),
            workspace_pool: workspace,
            generation: self.generation,
            checked_generation: self.checked_generation,
            cache_authority: self.cache_authority,
            generation_exhausted: self.generation_exhausted,
            complete: true,
            input_evidence: Some(input_evidence),
            invalid_input_evidence: false,
        }
    }

    /// Consume a prepared pass after exact private engine-evidence validation.
    ///
    /// Every veto occurs before active output, scratch, caches, or dirty sets
    /// are touched. The caller can then run the existing direct layout path.
    pub(crate) fn commit_prepared_layout(
        &mut self,
        mut prepared: PreparedLayoutPass,
        output: &mut LayoutOutput,
        current_input_evidence: LayoutInputEvidence,
    ) -> Result<(), PreparedLayoutCommitError> {
        prepared.validate_for_engine(self, current_input_evidence)?;

        let Some(mut storage) = prepared.take_workspace() else {
            return Err(PreparedLayoutCommitError::Incomplete);
        };
        let next_generation = self.generation + 1;
        let next_checked_generation = self.checked_generation + 1;
        let next_cache_authority = self.cache_authority + 1;

        self.measure_cache.reserve(storage.measure_updates.len());
        self.virtual_cache.reserve(storage.virtual_updates.len());
        self.measure_cache
            .retain(|key, _| storage.scratch.measured.contains_key(key));
        self.virtual_cache
            .retain(|key, _| storage.scratch.virtual_touched.contains(key));
        for (key, value) in storage.measure_updates.drain() {
            self.measure_cache.insert(key, value);
        }
        for (key, value) in storage.virtual_updates.drain() {
            self.virtual_cache.insert(key, value);
        }

        std::mem::swap(output, &mut storage.output);
        std::mem::swap(&mut self.scratch, &mut storage.scratch);
        for node_id in storage.layout_dirty.drain() {
            self.layout_dirty.remove(&node_id);
        }
        for node_id in storage.measure_dirty.drain() {
            self.measure_dirty.remove(&node_id);
        }
        storage.cache_key_ambiguity = false;
        storage.retain_reusable_capacity(&self.scratch, output);
        self.generation = next_generation;
        self.checked_generation = next_checked_generation;
        self.cache_authority = next_cache_authority;
        prepared.workspace_pool.return_storage(storage);
        Ok(())
    }

    /// Discard a prepared pass without changing active layout state.
    #[allow(dead_code)]
    pub(crate) fn discard_prepared_layout(&mut self, prepared: PreparedLayoutPass) {
        prepared.discard();
    }

    fn prune_stale_measure_cache(&mut self) {
        if self.measure_cache.len() == self.scratch.measured.len() {
            return;
        }
        self.measure_cache
            .retain(|key, _| self.scratch.measured.contains_key(key));
    }

    fn prune_stale_virtual_cache(&mut self) {
        if self.virtual_cache.len() == self.scratch.virtual_touched.len() {
            return;
        }
        self.virtual_cache
            .retain(|key, _| self.scratch.virtual_touched.contains(key));
    }
}

/// Measure and layout a strict slot tree into rounded rectangles.
pub fn layout_tree(root: &LayoutNode, root_rect: Rect) -> LayoutOutput {
    let mut engine = LayoutEngine::default();
    engine.layout(root, root_rect)
}

/// Measure and layout a strict slot tree with stateful container input.
///
/// This is the single-call entry point for callers that want scroll offsets or
/// debug primitives without manually reusing a [`LayoutEngine`].
pub fn layout_tree_with_state(
    root: &LayoutNode,
    root_rect: Rect,
    state: &LayoutState,
    debug: LayoutDebugOptions,
) -> LayoutOutput {
    let mut engine = LayoutEngine::default();
    engine.layout_with_state(root, root_rect, state, debug)
}

#[cfg(test)]
pub(super) fn round_rect(rect: Rect) -> Rect {
    crate::gui::layout_core::validated_geometry::ValidatedRect::rounded(rect)
        .map_or_else(Rect::default, |rect| rect.rect())
}
#[cfg(test)]
mod tests;

#[cfg(test)]
mod property_tests;

#[cfg(test)]
mod stress_tests;

#[cfg(test)]
mod virtualization_tests;

#[cfg(test)]
mod contract_tests;
