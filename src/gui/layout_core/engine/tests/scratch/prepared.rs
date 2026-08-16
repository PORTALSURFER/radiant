use super::fixtures::fixed_virtualized_root;
use crate::gui::{
    layout_core::{
        SplitPaneAxis, SplitPanePolicy,
        constraints::Constraints,
        engine::{
            LayoutContainerStateReadSource, LayoutDebugOptions, LayoutEngine, LayoutOutput,
            LayoutState, PreparedLayoutCommitError,
        },
        model::{ContainerKind, ContainerPolicy, SizeModeCross, SizeModeMain, SlotParams},
        tree::{LayoutNode, SlotChild, WidgetNode},
    },
    types::{Point, Rect, Vector2},
};
use std::collections::{HashMap, HashSet};
use std::num::NonZeroU64;
use std::sync::Arc;
use std::time::Instant;

use super::super::super::cache::{CachedVirtualMetrics, MeasureCacheKey, VirtualizationCacheKey};
use crate::gui::layout_core::{
    ContainerStateId, MountedContainerStateId, MountedContainerStateRead, SplitPaneRuntimeMode,
    SplitPaneRuntimePolicyRevision, SplitPaneRuntimeState, SplitPaneRuntimeStateInput,
};

#[derive(Clone, Debug, PartialEq)]
struct ScratchSnapshot {
    measured: HashMap<MeasureCacheKey, Vector2>,
    measured_by_node: HashMap<u64, Vector2>,
    virtual_touched: HashSet<VirtualizationCacheKey>,
    linear_windows: HashMap<u64, super::super::super::cache::ResolvedLinearWindow>,
    linear_sizes: Vec<f32>,
    linear_unresolved: Vec<usize>,
    dirty_path: Vec<u64>,
    dirty_marked: HashSet<u64>,
    capacities: (usize, usize, usize, usize, usize, usize, usize, usize),
}

#[derive(Clone, Debug, PartialEq)]
struct EngineSnapshot {
    measure_cache: HashMap<MeasureCacheKey, Vector2>,
    virtual_cache: HashMap<VirtualizationCacheKey, CachedVirtualMetrics>,
    cache_capacities: (usize, usize),
    scratch: ScratchSnapshot,
    layout_dirty: HashSet<u64>,
    measure_dirty: HashSet<u64>,
    generation: u64,
    checked_generation: u64,
    cache_authority: u64,
    generation_exhausted: bool,
}

fn snapshot(engine: &LayoutEngine) -> EngineSnapshot {
    let scratch = &engine.scratch;
    EngineSnapshot {
        measure_cache: engine.measure_cache.clone(),
        virtual_cache: engine.virtual_cache.clone(),
        cache_capacities: (
            engine.measure_cache.capacity(),
            engine.virtual_cache.capacity(),
        ),
        scratch: ScratchSnapshot {
            measured: scratch.measured.clone(),
            measured_by_node: scratch.measured_by_node.clone(),
            virtual_touched: scratch.virtual_touched.clone(),
            linear_windows: scratch.linear_windows.clone(),
            linear_sizes: scratch.linear_sizes.clone(),
            linear_unresolved: scratch.linear_unresolved.clone(),
            dirty_path: scratch.dirty_path.clone(),
            dirty_marked: scratch.dirty_marked.clone(),
            capacities: (
                scratch.measured.capacity(),
                scratch.measured_by_node.capacity(),
                scratch.virtual_touched.capacity(),
                scratch.linear_windows.capacity(),
                scratch.linear_sizes.capacity(),
                scratch.linear_unresolved.capacity(),
                scratch.dirty_path.capacity(),
                scratch.dirty_marked.capacity(),
            ),
        },
        layout_dirty: engine.layout_dirty.clone(),
        measure_dirty: engine.measure_dirty.clone(),
        generation: engine.generation,
        checked_generation: engine.checked_generation,
        cache_authority: engine.cache_authority,
        generation_exhausted: engine.generation_exhausted,
    }
}

fn viewport() -> Rect {
    Rect::from_min_size(Point::new(3.0, 5.0), Vector2::new(240.0, 140.0))
}

fn ordinary_root() -> LayoutNode {
    LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 4.0,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(2, Vector2::new(32.0, 18.0)),
            ),
            SlotChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(48.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                LayoutNode::widget(3, Vector2::new(48.0, 24.0)),
            ),
        ],
    )
}

fn split_root() -> LayoutNode {
    LayoutNode::container(
        10,
        ContainerPolicy {
            kind: ContainerKind::SplitPane,
            split_pane: SplitPanePolicy {
                axis: SplitPaneAxis::Horizontal,
                initial_ratio: 0.35,
                divider_extent: 6.0,
                first_min_extent: 24.0,
                second_min_extent: 32.0,
            },
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(11, Vector2::new(30.0, 20.0)),
            ),
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(12, Vector2::new(40.0, 20.0)),
            ),
        ],
    )
}

fn runtime_split_root() -> LayoutNode {
    LayoutNode::container_with_split_pane_runtime_mode(
        10,
        ContainerPolicy {
            kind: ContainerKind::SplitPane,
            split_pane: SplitPanePolicy {
                axis: SplitPaneAxis::Horizontal,
                initial_ratio: 0.35,
                divider_extent: 6.0,
                first_min_extent: 24.0,
                second_min_extent: 32.0,
            },
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(11, Vector2::new(30.0, 20.0)),
            ),
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(12, Vector2::new(40.0, 20.0)),
            ),
        ],
        Some(SplitPaneRuntimeMode::RuntimeOwned {
            collapse_policy: None,
        }),
    )
}

struct RuntimeSplitStateSource {
    mounted_id: MountedContainerStateId,
    state: SplitPaneRuntimeState,
}

impl LayoutContainerStateReadSource for RuntimeSplitStateSource {
    fn read_container_state(&self, container_id: u64) -> Option<MountedContainerStateRead<'_>> {
        (container_id == 10).then(|| MountedContainerStateRead::new(self.mounted_id, &self.state))
    }
}

fn runtime_split_state_source(ratio: f32) -> RuntimeSplitStateSource {
    let state_id = ContainerStateId::new::<SplitPaneRuntimeState>(10, 1);
    RuntimeSplitStateSource {
        mounted_id: MountedContainerStateId::new(state_id, NonZeroU64::MIN),
        state: SplitPaneRuntimeState::from_input(SplitPaneRuntimeStateInput {
            container_id: 10,
            initial_ratio: ratio,
            mode: SplitPaneRuntimeMode::RuntimeOwned {
                collapse_policy: None,
            },
            policy_revision: SplitPaneRuntimePolicyRevision::default(),
        }),
    }
}

fn commit(
    engine: &mut LayoutEngine,
    prepared: super::super::super::PreparedLayoutPass,
    output: &mut LayoutOutput,
) {
    assert_eq!(prepared.commit(engine, output), Ok(()));
}

#[test]
fn prepared_pass_matches_direct_for_ordinary_dirty_debug_split_and_virtualized_inputs() {
    let root = ordinary_root();
    let state = LayoutState::default();
    let debug = LayoutDebugOptions::default();
    let mut direct = LayoutEngine::default();
    let expected = direct.layout_with_state(&root, viewport(), &state, debug);
    let mut prepared_engine = LayoutEngine::default();
    let mut output = LayoutOutput::default();
    let prepared = prepared_engine.prepare_layout_with_state(&root, viewport(), &state, debug);
    commit(&mut prepared_engine, prepared, &mut output);
    assert_eq!(output, expected);
    assert_eq!(prepared_engine.measure_cache, direct.measure_cache);
    assert_eq!(prepared_engine.virtual_cache, direct.virtual_cache);

    let mut direct = LayoutEngine::default();
    let _ = direct.layout(&root, viewport());
    direct.mark_measure_dirty(2);
    let expected = direct.layout_with_state(&root, viewport(), &state, debug);
    let mut prepared_engine = LayoutEngine::default();
    let _ = prepared_engine.layout(&root, viewport());
    prepared_engine.mark_measure_dirty(2);
    let mut output = LayoutOutput::default();
    let prepared = prepared_engine.prepare_layout_with_state(&root, viewport(), &state, debug);
    commit(&mut prepared_engine, prepared, &mut output);
    assert_eq!(output, expected);

    let debug = LayoutDebugOptions::all_enabled();
    let mut direct = LayoutEngine::default();
    let expected = direct.layout_with_state(&root, viewport(), &state, debug);
    let mut prepared_engine = LayoutEngine::default();
    let mut output = LayoutOutput::default();
    let prepared = prepared_engine.prepare_layout_with_state(&root, viewport(), &state, debug);
    commit(&mut prepared_engine, prepared, &mut output);
    assert_eq!(output, expected);

    let split = split_root();
    let mut direct = LayoutEngine::default();
    let expected = direct.layout(&split, viewport());
    let mut prepared_engine = LayoutEngine::default();
    let mut output = LayoutOutput::default();
    let prepared = prepared_engine.prepare_layout_with_state(
        &split,
        viewport(),
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );
    commit(&mut prepared_engine, prepared, &mut output);
    assert_eq!(output, expected);

    let virtual_root = fixed_virtualized_root(96, 12.0);
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(0.0, 160.0));
    let mut direct = LayoutEngine::default();
    let expected = direct.layout_with_state(
        &virtual_root,
        viewport(),
        &state,
        LayoutDebugOptions::default(),
    );
    let mut prepared_engine = LayoutEngine::default();
    let mut output = LayoutOutput::default();
    let prepared = prepared_engine.prepare_layout_with_state(
        &virtual_root,
        viewport(),
        &state,
        LayoutDebugOptions::default(),
    );
    commit(&mut prepared_engine, prepared, &mut output);
    assert_eq!(output, expected);
}

#[test]
fn prepared_pass_matches_direct_for_runtime_container_state_split() {
    let root = runtime_split_root();
    let state = LayoutState::default();
    let debug = LayoutDebugOptions::all_enabled();
    let viewport = viewport();
    let mut direct_engine = LayoutEngine::default();
    let mut direct_output = LayoutOutput::default();
    let direct_source = runtime_split_state_source(0.72);
    direct_engine.layout_with_state_and_source_into(
        &root,
        viewport,
        &state,
        debug,
        Some(&direct_source),
        &mut direct_output,
    );

    let mut prepared_engine = LayoutEngine::default();
    let mut prepared_output = LayoutOutput::default();
    let prepared_source = runtime_split_state_source(0.72);
    let prepared = prepared_engine.prepare_layout_with_state_and_source(
        &root,
        viewport,
        &state,
        debug,
        Some(&prepared_source),
    );
    commit(&mut prepared_engine, prepared, &mut prepared_output);

    assert_eq!(prepared_output, direct_output);
    assert_eq!(prepared_engine.measure_cache, direct_engine.measure_cache);
    assert_eq!(prepared_engine.virtual_cache, direct_engine.virtual_cache);
}

#[test]
fn prepare_discard_and_drop_leave_active_state_and_output_inert() {
    let root = ordinary_root();
    let mut engine = LayoutEngine::default();
    let active_output = engine.layout(&root, viewport());
    let before = snapshot(&engine);
    let output_before = active_output.clone();

    let prepared = engine.prepare_layout_with_state(
        &root,
        viewport(),
        &LayoutState::default(),
        LayoutDebugOptions::all_enabled(),
    );
    assert_eq!(snapshot(&engine), before);
    assert_eq!(active_output, output_before);
    engine.discard_prepared_layout(prepared);
    assert_eq!(snapshot(&engine), before);
    assert_eq!(active_output, output_before);

    let prepared = engine.prepare_layout_with_state(
        &root,
        viewport(),
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );
    assert_eq!(snapshot(&engine), before);
    drop(prepared);
    assert_eq!(snapshot(&engine), before);
    assert_eq!(active_output, output_before);
}

#[test]
fn consuming_commit_installs_exact_output_and_prunes_untouched_cache_entries() {
    let old_widget = LayoutNode::Widget(WidgetNode {
        id: 1,
        intrinsic: Vector2::new(12.0, 12.0),
        state_version: 1,
    });
    let new_widget = LayoutNode::Widget(WidgetNode {
        id: 1,
        intrinsic: Vector2::new(52.0, 28.0),
        state_version: 2,
    });
    let mut engine = LayoutEngine::default();
    let _ = engine.layout(&old_widget, viewport());
    let mut output = LayoutOutput::default();
    let prepared = engine.prepare_layout_with_state(
        &new_widget,
        viewport(),
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );
    commit(&mut engine, prepared, &mut output);
    assert!(output.rects.contains_key(&1));
    assert_eq!(engine.measure_cache.len(), 1);
    assert_eq!(
        engine.measure_cache.values().next(),
        Some(&Vector2::new(52.0, 28.0))
    );
    assert!(engine.layout_dirty.is_empty());
    assert!(engine.measure_dirty.is_empty());
    assert_eq!(engine.checked_generation, 1);

    let old_virtual = fixed_virtualized_root(32, 12.0);
    let new_virtual = fixed_virtualized_root(48, 12.0);
    let mut engine = LayoutEngine::default();
    let _ = engine.layout(&old_virtual, viewport());
    assert_eq!(engine.virtual_cache.len(), 1);
    let mut output = LayoutOutput::default();
    let prepared = engine.prepare_layout_with_state(
        &new_virtual,
        viewport(),
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );
    commit(&mut engine, prepared, &mut output);
    assert_eq!(engine.virtual_cache.len(), 1);
    assert_eq!(
        engine
            .virtual_cache
            .values()
            .next()
            .map(|entry| entry.metrics.len()),
        Some(48)
    );
}

#[test]
fn incomplete_candidate_vetoes_without_active_mutation() {
    let root = ordinary_root();
    let mut engine = LayoutEngine::default();
    let held = engine.prepare_layout_with_state(
        &root,
        viewport(),
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );
    let incomplete = engine.prepare_layout_with_state(
        &root,
        viewport(),
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );
    let mut output = LayoutOutput::default();
    let before = snapshot(&engine);

    assert_eq!(
        engine.commit_prepared_layout(incomplete, &mut output),
        Err(PreparedLayoutCommitError::Incomplete)
    );
    assert_eq!(snapshot(&engine), before);
    drop(held);
}

#[test]
fn ambiguous_cache_key_vetoes_without_active_mutation() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Stack,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::Widget(WidgetNode {
                    id: 2,
                    intrinsic: Vector2::new(10.0, 10.0),
                    state_version: 1,
                }),
            ),
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::Widget(WidgetNode {
                    id: 2,
                    intrinsic: Vector2::new(20.0, 20.0),
                    state_version: 1,
                }),
            ),
        ],
    );
    let mut engine = LayoutEngine::default();
    engine.mark_measure_dirty(2);
    let prepared = engine.prepare_layout_with_state(
        &root,
        viewport(),
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );
    let mut output = LayoutOutput::default();
    let before = snapshot(&engine);

    assert_eq!(
        engine.commit_prepared_layout(prepared, &mut output),
        Err(PreparedLayoutCommitError::CacheKeyAmbiguous)
    );
    assert_eq!(snapshot(&engine), before);
}

#[test]
fn stale_commit_vetoes_before_any_active_mutation() {
    let root = ordinary_root();
    let mut engine = LayoutEngine::default();
    let mut output = engine.layout(&root, viewport());
    let prepared = engine.prepare_layout_with_state(
        &root,
        viewport(),
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );
    engine.mark_layout_dirty(999);
    let before_commit = snapshot(&engine);
    let output_before = output.clone();

    assert_eq!(
        engine.commit_prepared_layout(prepared, &mut output),
        Err(PreparedLayoutCommitError::StaleEngineGeneration)
    );
    assert_eq!(snapshot(&engine), before_commit);
    assert_eq!(output, output_before);
}

#[test]
fn stale_cache_authority_vetoes_before_any_active_mutation() {
    let root = ordinary_root();
    let mut engine = LayoutEngine::default();
    let mut output = engine.layout(&root, viewport());
    let prepared = engine.prepare_layout_with_state(
        &root,
        viewport(),
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );
    engine.cache_authority += 1;
    let before_commit = snapshot(&engine);
    let output_before = output.clone();

    assert_eq!(
        engine.commit_prepared_layout(prepared, &mut output),
        Err(PreparedLayoutCommitError::StaleCacheAuthority)
    );
    assert_eq!(snapshot(&engine), before_commit);
    assert_eq!(output, output_before);
}

#[test]
fn warmed_preparation_reads_active_caches_without_cloning_complete_cache_state() {
    let root = fixed_virtualized_root(96, 12.0);
    let mut engine = LayoutEngine::default();
    let mut output = engine.layout(&root, viewport());
    let active_virtual_metrics = engine
        .virtual_cache
        .values()
        .next()
        .map(|entry| Arc::clone(&entry.metrics));
    let before = snapshot(&engine);
    let prepared = engine.prepare_layout_with_state(
        &root,
        viewport(),
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );
    assert_eq!(snapshot(&engine), before);
    let storage = prepared
        .workspace
        .as_ref()
        .map(|workspace| workspace.measure_updates.len());
    assert_eq!(storage, Some(0));
    assert_eq!(
        prepared
            .workspace
            .as_ref()
            .map(|workspace| workspace.virtual_updates.len()),
        Some(0)
    );
    commit(&mut engine, prepared, &mut output);
    if let Some(metrics) = active_virtual_metrics {
        let committed = engine
            .virtual_cache
            .values()
            .next()
            .map(|entry| &entry.metrics);
        assert!(committed.is_some_and(|value| Arc::ptr_eq(value, &metrics)));
    }
}

#[test]
fn warmed_preparation_reuses_workspace_capacities() {
    let root = fixed_virtualized_root(128, 12.0);
    let mut engine = LayoutEngine::default();
    let initial = engine.preparation_workspace.capacities();
    let prepared = engine.prepare_layout_with_state(
        &root,
        viewport(),
        &LayoutState::default(),
        LayoutDebugOptions::all_enabled(),
    );
    let established = engine.preparation_workspace.capacities();
    assert!(established.measure_updates >= initial.measure_updates);
    assert!(established.measured > 0);
    assert!(established.debug_primitives > 0);
    let mut output = LayoutOutput::default();
    commit(&mut engine, prepared, &mut output);
    let warmed = engine.preparation_workspace.capacities();

    let prepared = engine.prepare_layout_with_state(
        &root,
        viewport(),
        &LayoutState::default(),
        LayoutDebugOptions::all_enabled(),
    );
    let second = engine.preparation_workspace.capacities();
    assert_eq!(second, warmed);
    drop(prepared);
    assert_eq!(engine.preparation_workspace.capacities(), second);
}

#[test]
fn warmed_preparation_performance_evidence_reports_percentiles() {
    const SAMPLES: usize = 100;

    let root = fixed_virtualized_root(128, 12.0);
    let state = LayoutState::default();
    let debug = LayoutDebugOptions::default();
    let mut engine = LayoutEngine::default();
    let mut output = engine.layout_with_state(&root, viewport(), &state, debug);

    for _ in 0..10 {
        let prepared = engine.prepare_layout_with_state(&root, viewport(), &state, debug);
        assert_eq!(
            prepared
                .workspace
                .as_ref()
                .map(|storage| (storage.measure_updates.len(), storage.virtual_updates.len())),
            Some((0, 0))
        );
        commit(&mut engine, prepared, &mut output);
    }

    let warmed_capacities = engine.preparation_workspace.capacities();
    let warmed_metrics = engine
        .virtual_cache
        .values()
        .next()
        .map(|entry| Arc::clone(&entry.metrics))
        .expect("warmed virtual cache entry");
    let mut discard_samples = Vec::with_capacity(SAMPLES);
    let mut commit_samples = Vec::with_capacity(SAMPLES);

    for _ in 0..SAMPLES {
        let started = Instant::now();
        let prepared = engine.prepare_layout_with_state(&root, viewport(), &state, debug);
        assert_eq!(
            prepared
                .workspace
                .as_ref()
                .map(|storage| (storage.measure_updates.len(), storage.virtual_updates.len())),
            Some((0, 0))
        );
        engine.discard_prepared_layout(prepared);
        discard_samples.push(started.elapsed().as_nanos());
        assert_eq!(engine.preparation_workspace.capacities(), warmed_capacities);
        assert!(
            engine
                .virtual_cache
                .values()
                .next()
                .is_some_and(|entry| Arc::ptr_eq(&entry.metrics, &warmed_metrics))
        );
    }

    for _ in 0..SAMPLES {
        let started = Instant::now();
        let prepared = engine.prepare_layout_with_state(&root, viewport(), &state, debug);
        assert_eq!(
            prepared
                .workspace
                .as_ref()
                .map(|storage| (storage.measure_updates.len(), storage.virtual_updates.len())),
            Some((0, 0))
        );
        commit(&mut engine, prepared, &mut output);
        commit_samples.push(started.elapsed().as_nanos());
        assert_eq!(engine.preparation_workspace.capacities(), warmed_capacities);
        assert!(
            engine
                .virtual_cache
                .values()
                .next()
                .is_some_and(|entry| Arc::ptr_eq(&entry.metrics, &warmed_metrics))
        );
    }

    discard_samples.sort_unstable();
    commit_samples.sort_unstable();
    let percentile =
        |samples: &[u128], percentage: usize| samples[(samples.len() - 1) * percentage / 100];
    eprintln!(
        "{{\"test\":\"warmed_preparation\",\"iterations\":{},\"discard_ns\":{{\"p50\":{},\"p95\":{},\"p99\":{}}},\"commit_ns\":{{\"p50\":{},\"p95\":{},\"p99\":{}}},\"cache_updates_zero\":true,\"arc_reuse\":true,\"workspace_capacities_stable\":true}}",
        SAMPLES,
        percentile(&discard_samples, 50),
        percentile(&discard_samples, 95),
        percentile(&discard_samples, 99),
        percentile(&commit_samples, 50),
        percentile(&commit_samples, 95),
        percentile(&commit_samples, 99),
    );
}

#[test]
fn generation_exhaustion_vetoes_without_active_mutation() {
    let root = ordinary_root();
    let mut engine = LayoutEngine::default();
    let mut output = engine.layout(&root, viewport());
    engine.generation = u64::MAX;
    let prepared = engine.prepare_layout_with_state(
        &root,
        viewport(),
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );
    let before = snapshot(&engine);
    let output_before = output.clone();
    assert_eq!(
        engine.commit_prepared_layout(prepared, &mut output),
        Err(PreparedLayoutCommitError::GenerationExhausted)
    );
    assert_eq!(snapshot(&engine), before);
    assert_eq!(output, output_before);
}
